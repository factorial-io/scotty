//! Tar pack/unpack streaming helpers for `app:cp`.
//!
//! The synchronous `tar` crate is driven from a `spawn_blocking` task. A
//! bounded `tokio::sync::mpsc` channel bridges the blocking side to/from the
//! async `Stream` / `AsyncRead` world without buffering the whole archive.

use std::fs;
use std::io::{self, Read, Seek, Write};
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

/// Pack a local file or directory into a tar byte stream under `entry_name`.
///
/// `entry_name` is the name the single top-level entry carries inside the
/// archive. The caller derives it from the remote destination so that Docker,
/// which extracts the archive into a directory, places (and possibly renames)
/// the content correctly — see `split_remote_dest` in the parent module.
pub fn tar_pack_path(path: &Path, entry_name: &str) -> io::Result<TarByteStream> {
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("local path '{}' does not exist", path.display()),
        ));
    }
    let path = path.to_path_buf();
    let entry_name = PathBuf::from(entry_name);

    let (tx, rx) = mpsc::channel::<io::Result<Bytes>>(CHANNEL_CAPACITY);
    let tx_err = tx.clone();

    tokio::task::spawn_blocking(move || {
        let writer = ChannelWriter { tx };
        let mut builder = tar::Builder::new(writer);
        builder.follow_symlinks(false);

        let result: io::Result<()> = (|| {
            let metadata = std::fs::symlink_metadata(&path)?;

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

/// Unpack a stream of tar bytes into `dest`, following `docker cp` semantics.
///
/// - If `dest` is a directory target — it ends with a path separator, or it
///   already exists as a directory — the archive is extracted *into* it.
/// - Otherwise `dest` is a named target. If the archive contains exactly one
///   top-level entry it is moved to `dest` (so downloading a single file to a
///   non-existent path produces that file, not `dest/<name>`). If it contains
///   zero or several entries, `dest` is created as a directory and the
///   entries are placed inside it.
pub async fn tar_unpack_to_path<S, E>(stream: S, dest: &Path) -> io::Result<()>
where
    S: Stream<Item = Result<Bytes, E>> + Send + 'static,
    E: std::fmt::Display + Send + 'static,
{
    let dest = dest.to_path_buf();
    let dir_target = is_dir_target(&dest);

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
        if dir_target {
            // Extract directly into the destination directory (preserves the
            // merge-into-existing-directory behaviour of `Archive::unpack`).
            fs::create_dir_all(&dest)?;
            let mut archive = tar::Archive::new(reader);
            archive.set_preserve_permissions(true);
            archive.set_preserve_mtime(true);
            archive.unpack(&dest)?;
            return Ok(());
        }

        // Named target: extract into a temporary directory next to `dest`
        // (same filesystem, so the final move is a cheap rename), then
        // reconcile the result onto `dest`.
        let parent = dest
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        fs::create_dir_all(&parent)?;
        let tmp = tempfile::Builder::new()
            .prefix(".scotty-cp-")
            .tempdir_in(&parent)?;

        {
            let mut archive = tar::Archive::new(reader);
            archive.set_preserve_permissions(true);
            archive.set_preserve_mtime(true);
            archive.unpack(tmp.path())?;
        }

        let entries: Vec<PathBuf> = fs::read_dir(tmp.path())?
            .map(|e| e.map(|e| e.path()))
            .collect::<io::Result<_>>()?;

        if entries.len() == 1 {
            // Single entry (file or directory): it becomes `dest` itself.
            // Remove any pre-existing target first so the rename does not fail
            // on a non-empty directory or surprise the user with a merge.
            remove_existing(&dest)?;
            fs::rename(&entries[0], &dest)?;
        } else {
            // Zero or multiple entries: `dest` is a directory holding them.
            fs::create_dir_all(&dest)?;
            for entry in entries {
                let name = entry.file_name().ok_or_else(|| {
                    io::Error::other(format!("archive entry has no name: {}", entry.display()))
                })?;
                let target = dest.join(name);
                remove_existing(&target)?;
                fs::rename(&entry, &target)?;
            }
        }
        Ok(())
    });

    let unpack_result = unpack
        .await
        .map_err(|e| io::Error::other(format!("unpack task panicked: {e}")))?;
    // Make sure the pump finishes; it has no return value worth observing.
    let _ = pump.await;
    unpack_result
}

/// Whether `dest` should be treated as a directory the archive is extracted
/// *into*: it ends with a path separator, or it already exists as a directory.
fn is_dir_target(dest: &Path) -> bool {
    if dest.is_dir() {
        return true;
    }
    // `Path` strips a trailing separator, so check the raw string form.
    let s = dest.to_string_lossy();
    s.ends_with('/') || s.ends_with(std::path::MAIN_SEPARATOR)
}

/// Remove an existing file or directory at `path` so it can be replaced.
/// Missing paths are treated as success.
fn remove_existing(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
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

/// Pack stdin into a single-entry tar stream under `entry_name` so that
/// `app:cp - app:svc:/tmp/dump.sql` produces a tar entry named `dump.sql`
/// extracted into `/tmp/`. The caller derives `entry_name` from the remote
/// destination (see `split_remote_dest`).
pub fn pipe_pack(entry_name: &str) -> io::Result<TarByteStream> {
    let entry_name = entry_name.to_string();
    let (tx, rx) = mpsc::channel::<io::Result<Bytes>>(CHANNEL_CAPACITY);
    let tx_err = tx.clone();

    tokio::task::spawn_blocking(move || {
        let writer = ChannelWriter { tx };
        let mut builder = tar::Builder::new(writer);

        let result: io::Result<()> = (|| {
            // tar requires the entry size up front, so stdin must be fully
            // consumed before the archive can be written. Spool it to a
            // temporary file rather than an in-memory `Vec` so that a large
            // pipe payload (e.g. a multi-GiB DB dump) cannot OOM the client:
            // memory stays bounded to the copy buffer regardless of input
            // size. The temp file is removed when `spool` is dropped.
            let mut spool = tempfile::tempfile()?;
            let size = io::copy(&mut io::stdin().lock(), &mut spool)?;
            spool.rewind()?;

            let mut header = tar::Header::new_gnu();
            header.set_path(&entry_name)?;
            header.set_size(size);
            header.set_mode(0o644);
            header.set_mtime(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            );
            header.set_entry_type(tar::EntryType::Regular);
            header.set_cksum();

            builder.append(&header, &mut spool)?;
            builder.finish()?;
            Ok(())
        })();

        if let Err(err) = result {
            let _ = tx_err.blocking_send(Err(err));
        }
    });

    Ok(Box::pin(ReceiverStream::new(rx)))
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
                // We stop at the second file entry, so we cannot report a
                // precise total — phrase the message accordingly rather than
                // claiming a fixed count.
                return Err(io::Error::other(
                    "pipe mode requires a single file, but the archive contains more than one \
                     file entry; use a local destination path instead of '-' to extract a directory",
                ));
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
    fn dir_target_detection() {
        assert!(is_dir_target(Path::new("/tmp/")));
        assert!(!is_dir_target(Path::new("/tmp/hostname")));
    }

    /// Build a single-entry tar archive in memory whose entry is a regular
    /// file with the given name and contents.
    fn single_file_archive(name: &str, contents: &[u8]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_path(name).unwrap();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        builder.append(&header, contents).unwrap();
        builder.into_inner().unwrap()
    }

    fn stream_of(bytes: Vec<u8>) -> impl Stream<Item = io::Result<Bytes>> + Send + 'static {
        futures_util::stream::once(async move { Ok(Bytes::from(bytes)) })
    }

    #[tokio::test]
    async fn single_file_to_named_dest_becomes_a_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("hostname");
        let archive = single_file_archive("hostname", b"host-contents\n");

        tar_unpack_to_path(stream_of(archive), &dest).await.unwrap();

        // `dest` must be the file itself, not a directory containing it.
        assert!(dest.is_file(), "destination should be a file");
        assert_eq!(fs::read(&dest).unwrap(), b"host-contents\n");
        assert!(!dest.join("hostname").exists());
    }

    #[tokio::test]
    async fn single_file_to_dir_target_lands_inside() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("out");
        fs::create_dir(&dest).unwrap();
        let archive = single_file_archive("hostname", b"host\n");

        tar_unpack_to_path(stream_of(archive), &dest).await.unwrap();

        // Extracting into an existing directory keeps the entry name.
        assert!(dest.join("hostname").is_file());
    }

    #[tokio::test]
    async fn single_file_to_named_dest_overwrites_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("hostname");
        fs::write(&dest, b"stale").unwrap();
        let archive = single_file_archive("hostname", b"fresh\n");

        tar_unpack_to_path(stream_of(archive), &dest).await.unwrap();

        assert!(dest.is_file());
        assert_eq!(fs::read(&dest).unwrap(), b"fresh\n");
    }
}
