//! Tar pack/unpack streaming helpers for `app:cp`.
//!
//! The synchronous `tar` crate is driven from a `spawn_blocking` task. A
//! bounded `tokio::sync::mpsc` channel bridges the blocking side to/from the
//! async `Stream` / `AsyncRead` world without buffering the whole archive.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Channel capacity for the blocking <-> async bridge. Eight 8 KiB buffers
/// is enough to hide scheduling latency while keeping memory bounded.
const CHANNEL_CAPACITY: usize = 8;

/// Concrete byte stream type returned by the pack helpers and accepted by
/// the unpack helpers. The `io::Error` carries either a real I/O failure
/// from disk/stdin or a tar-crate error mapped through `io::Error::other`.
pub type TarByteStream = std::pin::Pin<Box<dyn Stream<Item = io::Result<Bytes>> + Send + 'static>>;

/// `std::io::Write` adapter that forwards each write into an mpsc channel.
struct ChannelWriter {
    tx: mpsc::Sender<io::Result<Bytes>>,
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let bytes = Bytes::copy_from_slice(buf);
        // The blocking task must not call `tokio` async APIs; use the
        // dedicated `blocking_send` for the rendezvous.
        self.tx
            .blocking_send(Ok(bytes))
            .map_err(|_| io::Error::other("receiver dropped"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Pack a local file or directory into a tar byte stream.
pub fn tar_pack_path(path: &Path) -> io::Result<TarByteStream> {
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("local path '{}' does not exist", path.display()),
        ));
    }
    let path = path.to_path_buf();

    let (tx, rx) = mpsc::channel::<io::Result<Bytes>>(CHANNEL_CAPACITY);
    let tx_err = tx.clone();

    tokio::task::spawn_blocking(move || {
        let writer = ChannelWriter { tx };
        let mut builder = tar::Builder::new(writer);
        builder.follow_symlinks(false);

        let result: io::Result<()> = (|| {
            let metadata = std::fs::symlink_metadata(&path)?;
            // Tar entry name is the basename of `path` — this mirrors
            // `docker cp` and lets the server-side extractor place it
            // sensibly inside the destination directory.
            let entry_name: PathBuf = path
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));

            if metadata.is_dir() {
                builder.append_dir_all(&entry_name, &path)?;
            } else {
                let mut file = std::fs::File::open(&path)?;
                builder.append_file(&entry_name, &mut file)?;
            }
            builder.finish()?;
            Ok(())
        })();

        if let Err(err) = result {
            // Best-effort: forward the failure to the consumer. Ignore send
            // errors (receiver is gone).
            let _ = tx_err.blocking_send(Err(err));
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}

/// Unpack a stream of tar bytes into `dest`.
///
/// `dest` is interpreted in the same way `docker cp` interprets the
/// destination: the archive is extracted *into* `dest` (which is created if
/// it does not exist and `dest` looks like a directory target).
pub async fn tar_unpack_to_path<S, E>(stream: S, dest: &Path) -> io::Result<()>
where
    S: Stream<Item = Result<Bytes, E>> + Send + 'static,
    E: std::fmt::Display + Send + 'static,
{
    let dest = dest.to_path_buf();
    // Ensure the destination directory exists. If `dest` is the path of a
    // file the user wants to receive, we extract into its parent and rename
    // afterwards. To keep the helper simple we mirror docker-cp behavior:
    // if `dest` exists as a directory, extract into it; otherwise create it
    // as a directory and extract into it.
    if !dest.exists() {
        std::fs::create_dir_all(&dest)?;
    }

    let (tx, rx) = mpsc::channel::<io::Result<Bytes>>(CHANNEL_CAPACITY);

    // Async side: pump the incoming HTTP stream into the channel.
    let pump = tokio::spawn(async move {
        let mut stream = Box::pin(stream);
        while let Some(item) = stream.next().await {
            let mapped = item.map_err(|e| io::Error::other(e.to_string()));
            if tx.send(mapped).await.is_err() {
                break;
            }
        }
    });

    // Sync side: feed the channel into `tar::Archive::unpack`.
    let unpack = tokio::task::spawn_blocking(move || -> io::Result<()> {
        let reader = ChannelReader::new(rx);
        let mut archive = tar::Archive::new(reader);
        archive.set_preserve_permissions(true);
        archive.set_preserve_mtime(true);
        archive.unpack(&dest)?;
        Ok(())
    });

    let unpack_result = unpack
        .await
        .map_err(|e| io::Error::other(format!("unpack task panicked: {e}")))?;
    // Make sure the pump finishes; it has no return value worth observing.
    let _ = pump.await;
    unpack_result
}

/// `std::io::Read` adapter pulling from a blocking-capable mpsc channel.
struct ChannelReader {
    rx: mpsc::Receiver<io::Result<Bytes>>,
    buf: Bytes,
}

impl ChannelReader {
    fn new(rx: mpsc::Receiver<io::Result<Bytes>>) -> Self {
        Self {
            rx,
            buf: Bytes::new(),
        }
    }
}

impl Read for ChannelReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        loop {
            if !self.buf.is_empty() {
                let n = std::cmp::min(out.len(), self.buf.len());
                out[..n].copy_from_slice(&self.buf[..n]);
                self.buf = self.buf.slice(n..);
                return Ok(n);
            }
            match self.rx.blocking_recv() {
                Some(Ok(bytes)) => self.buf = bytes,
                Some(Err(e)) => return Err(e),
                None => return Ok(0), // EOF
            }
        }
    }
}

/// Pack stdin into a single-entry tar stream. The entry name is the
/// basename of the remote destination path so that `app:cp - app:svc:/tmp/dump.sql`
/// produces a tar entry named `dump.sql` extracted into `/tmp/`.
pub fn pipe_pack(remote_dest_path: &str) -> io::Result<TarByteStream> {
    let entry_name = pipe_entry_name(remote_dest_path);
    let (tx, rx) = mpsc::channel::<io::Result<Bytes>>(CHANNEL_CAPACITY);
    let tx_err = tx.clone();

    tokio::task::spawn_blocking(move || {
        let writer = ChannelWriter { tx };
        let mut builder = tar::Builder::new(writer);

        let result: io::Result<()> = (|| {
            // Buffer stdin into memory — necessary because tar requires the
            // entry size up front. For very large pipe payloads users should
            // prefer file mode. We document this trade-off in the help text.
            let mut buf = Vec::new();
            io::stdin().read_to_end(&mut buf)?;

            let mut header = tar::Header::new_gnu();
            header.set_path(&entry_name)?;
            header.set_size(buf.len() as u64);
            header.set_mode(0o644);
            header.set_mtime(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            );
            header.set_entry_type(tar::EntryType::Regular);
            header.set_cksum();

            builder.append(&header, buf.as_slice())?;
            builder.finish()?;
            Ok(())
        })();

        if let Err(err) = result {
            let _ = tx_err.blocking_send(Err(err));
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
}

/// Derive the tar entry name from a remote destination path.
fn pipe_entry_name(remote_dest_path: &str) -> String {
    let trimmed = remote_dest_path.trim_end_matches('/');
    let base = trimmed.rsplit('/').next().unwrap_or("");
    if base.is_empty() {
        "stdin".to_string()
    } else {
        base.to_string()
    }
}

/// Unpack a download stream and forward the first regular-file entry to
/// stdout. Errors if more than one regular-file entry is present.
pub async fn pipe_unpack<S, E>(stream: S) -> io::Result<()>
where
    S: Stream<Item = Result<Bytes, E>> + Send + 'static,
    E: std::fmt::Display + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<io::Result<Bytes>>(CHANNEL_CAPACITY);

    let pump = tokio::spawn(async move {
        let mut stream = Box::pin(stream);
        while let Some(item) = stream.next().await {
            let mapped = item.map_err(|e| io::Error::other(e.to_string()));
            if tx.send(mapped).await.is_err() {
                break;
            }
        }
    });

    let extract = tokio::task::spawn_blocking(move || -> io::Result<()> {
        let reader = ChannelReader::new(rx);
        let mut archive = tar::Archive::new(reader);
        let mut stdout = io::stdout().lock();
        let mut files_seen: usize = 0;

        for entry in archive.entries()? {
            let mut entry = entry?;
            let entry_type = entry.header().entry_type();
            if !entry_type.is_file() {
                continue;
            }
            files_seen += 1;
            if files_seen > 1 {
                return Err(io::Error::other(format!(
                    "pipe mode requires a single file; got {files_seen} entries"
                )));
            }
            io::copy(&mut entry, &mut stdout)?;
        }

        if files_seen == 0 {
            return Err(io::Error::other(
                "archive contained no regular file entries",
            ));
        }
        Ok(())
    });

    let res = extract
        .await
        .map_err(|e| io::Error::other(format!("extract task panicked: {e}")))?;
    let _ = pump.await;
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_name_uses_basename() {
        assert_eq!(pipe_entry_name("/tmp/dump.sql"), "dump.sql");
        assert_eq!(pipe_entry_name("/tmp/"), "tmp");
        assert_eq!(pipe_entry_name("dump.sql"), "dump.sql");
        assert_eq!(pipe_entry_name("/"), "stdin");
        assert_eq!(pipe_entry_name(""), "stdin");
    }
}
