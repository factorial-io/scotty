//! `app:cp` — docker-cp-style file transfer.
//!
//! See `openspec/changes/app-file-transfer/` for the design.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context};
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use owo_colors::OwoColorize;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::{Body, StatusCode};
use scotty_types::files::{FileTransferError, FileTransferErrorCode};

use crate::api::get_auth_token;
use crate::cli::CopyCommand;
use crate::context::{AppContext, ServerSettings};

use scotty_core::version::VersionManager;

mod spec;
mod stream;

pub use spec::{parse_path_spec, PathSpec};

use stream::{pipe_pack, pipe_unpack, tar_pack_path, tar_unpack_to_path, TarByteStream};

/// Validate that exactly one side of `app:cp` is `Remote`.
pub fn validate_endpoints(src: &PathSpec, dst: &PathSpec) -> anyhow::Result<()> {
    match (src.is_remote(), dst.is_remote()) {
        (true, true) => bail!(
            "exactly one of <source> and <destination> must be a remote spec (<app>:<service>:<path>)"
        ),
        (false, false) => bail!(
            "exactly one of <source> and <destination> must be a remote spec; both look local"
        ),
        _ => Ok(()),
    }
}

/// Entry point for the `app:cp` subcommand.
pub async fn copy(context: &AppContext, cmd: &CopyCommand) -> anyhow::Result<()> {
    let src = parse_path_spec(&cmd.source);
    let dst = parse_path_spec(&cmd.destination);
    validate_endpoints(&src, &dst)?;

    let server = context.server();

    match (src, dst) {
        (PathSpec::Remote { app, service, path }, PathSpec::Local(local)) => {
            download(
                context,
                server,
                &app,
                service.as_deref(),
                &path,
                Some(&local),
            )
            .await
        }
        (PathSpec::Remote { app, service, path }, PathSpec::Stdio) => {
            download(context, server, &app, service.as_deref(), &path, None).await
        }
        (PathSpec::Local(local), PathSpec::Remote { app, service, path }) => {
            upload(
                context,
                server,
                &app,
                service.as_deref(),
                &path,
                UploadSource::Local(local),
            )
            .await
        }
        (PathSpec::Stdio, PathSpec::Remote { app, service, path }) => {
            upload(
                context,
                server,
                &app,
                service.as_deref(),
                &path,
                UploadSource::Stdio,
            )
            .await
        }
        _ => unreachable!("validate_endpoints rejects this case"),
    }
}

/// Where the upload bytes come from.
enum UploadSource {
    Local(PathBuf),
    Stdio,
}

/// Resolve `service` for an app: fetch the app's info and pick the single
/// public service. Errors out (with a candidate list) if 0 or >1 candidates
/// exist.
async fn resolve_service(
    server: &ServerSettings,
    app: &str,
    explicit: Option<&str>,
) -> anyhow::Result<String> {
    if let Some(s) = explicit {
        if !s.is_empty() {
            return Ok(s.to_string());
        }
    }

    let app_data = super::get_app_info(server, app).await.with_context(|| {
        format!("failed to look up app '{app}' for implicit service resolution")
    })?;

    // The "public service" signal from the blueprint is reflected on the
    // running app as `settings.public_services`. We pick the single entry
    // there; if it's missing or ambiguous, list all services to help the
    // user.
    let candidates: Vec<String> = match &app_data.settings {
        Some(settings) => settings
            .public_services
            .iter()
            .map(|s| s.service.clone())
            .collect(),
        None => Vec::new(),
    };

    if candidates.len() == 1 {
        return Ok(candidates.into_iter().next().unwrap());
    }

    let all_services: Vec<String> = app_data
        .services
        .iter()
        .map(|s| s.service.clone())
        .collect();

    if candidates.is_empty() {
        bail!(
            "app '{app}' has no public service; specify one explicitly. Available services: {}",
            all_services.join(", ")
        );
    }
    bail!(
        "app '{app}' has multiple public services ({}); specify one explicitly. Available services: {}",
        candidates.join(", "),
        all_services.join(", ")
    );
}

/// Build the absolute URL for the file-transfer endpoint.
fn build_url(server: &ServerSettings, app: &str, service: &str, path: &str) -> String {
    let base = server.server.trim_end_matches('/');
    format!(
        "{base}/api/v1/apps/{app}/services/{service}/files?path={}",
        urlencoding::encode(path)
    )
}

/// Construct a reqwest client carrying the bearer token and user agent.
fn build_client(token: &str) -> anyhow::Result<reqwest::Client> {
    let version = VersionManager::current_version()
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {token}").parse().context("invalid token")?,
    );
    headers.insert(
        USER_AGENT,
        format!("scottyctl/{version}")
            .parse()
            .context("invalid UA")?,
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .context("failed to build HTTP client")
}

/// Parse a non-2xx response into a friendly `anyhow::Error`. Tries to read
/// the JSON `FileTransferError` body first; falls back to a generic message.
async fn http_error(status: StatusCode, resp: reqwest::Response) -> anyhow::Error {
    let body_text = resp.text().await.unwrap_or_default();
    if let Ok(parsed) = serde_json::from_str::<FileTransferError>(&body_text) {
        let hint = friendly_hint(parsed.code);
        let code_label = format!("{:?}", parsed.code);
        if hint.is_empty() {
            anyhow!("[{code_label}] {}", parsed.message)
        } else {
            anyhow!("[{code_label}] {} — {hint}", parsed.message)
        }
    } else if body_text.is_empty() {
        anyhow!("server returned HTTP {status}")
    } else {
        anyhow!("server returned HTTP {status}: {body_text}")
    }
}

fn friendly_hint(code: FileTransferErrorCode) -> &'static str {
    match code {
        FileTransferErrorCode::Forbidden => {
            "you do not have permission for this operation on this app"
        }
        FileTransferErrorCode::NotFound => "check the app name, service name, and container path",
        FileTransferErrorCode::ServiceNotRunning => {
            "start the service before transferring files (e.g. scottyctl app:run)"
        }
        FileTransferErrorCode::PayloadTooLarge => {
            "the transfer exceeded the server's configured maximum size"
        }
        FileTransferErrorCode::InvalidPath => "the container path must be absolute and non-empty",
        FileTransferErrorCode::Internal => "",
    }
}

async fn download(
    context: &AppContext,
    server: &ServerSettings,
    app: &str,
    service: Option<&str>,
    path: &str,
    dest: Option<&Path>,
) -> anyhow::Result<()> {
    let ui = context.ui();
    let service = resolve_service(server, app, service).await?;
    ui.new_status_line(format!(
        "Downloading {} from {}:{}...",
        path.yellow(),
        app.yellow(),
        service.yellow()
    ));

    let token = get_auth_token(server).await?;
    let client = build_client(&token)?;
    let url = build_url(server, app, &service, path);
    tracing::info!(%url, "GET file transfer");

    let resp = client
        .get(&url)
        .send()
        .await
        .context("failed to issue GET to file transfer endpoint")?;
    let status = resp.status();
    if !status.is_success() {
        return Err(http_error(status, resp).await);
    }

    let counter = Arc::new(AtomicU64::new(0));
    let counted = count_bytes(resp.bytes_stream(), counter.clone());
    let progress = spawn_progress(counter.clone(), "downloaded");

    let result = match dest {
        Some(local) => tar_unpack_to_path(counted, local).await,
        None => pipe_unpack(counted).await,
    };

    progress.finish();
    result.context("failed to extract downloaded archive")?;
    ui.success(format!(
        "Downloaded {} ({} bytes)",
        path.yellow(),
        counter.load(Ordering::Relaxed)
    ));
    Ok(())
}

async fn upload(
    context: &AppContext,
    server: &ServerSettings,
    app: &str,
    service: Option<&str>,
    path: &str,
    source: UploadSource,
) -> anyhow::Result<()> {
    let ui = context.ui();
    let service = resolve_service(server, app, service).await?;
    ui.new_status_line(format!(
        "Uploading to {}:{}{}...",
        app.yellow(),
        service.yellow(),
        path.yellow()
    ));

    let token = get_auth_token(server).await?;
    let client = build_client(&token)?;
    let url = build_url(server, app, &service, path);
    tracing::info!(%url, "PUT file transfer");

    let tar_stream: TarByteStream = match source {
        UploadSource::Local(local) => tar_pack_path(&local)
            .with_context(|| format!("failed to pack local path '{}'", local.display()))?,
        UploadSource::Stdio => pipe_pack(path).context("failed to pack stdin")?,
    };

    let counter = Arc::new(AtomicU64::new(0));
    let counted = count_bytes(tar_stream, counter.clone());
    let progress = spawn_progress(counter.clone(), "uploaded");

    let resp = client
        .put(&url)
        .header(CONTENT_TYPE, "application/x-tar")
        .body(Body::wrap_stream(counted))
        .send()
        .await
        .context("failed to issue PUT to file transfer endpoint")?;

    progress.finish();

    let status = resp.status();
    if !status.is_success() {
        return Err(http_error(status, resp).await);
    }

    ui.success(format!(
        "Uploaded to {}{} ({} bytes)",
        app.yellow(),
        path.yellow(),
        counter.load(Ordering::Relaxed)
    ));
    Ok(())
}

/// Wrap a byte stream so each chunk's length is added to `counter`.
fn count_bytes<S, E>(
    stream: S,
    counter: Arc<AtomicU64>,
) -> impl Stream<Item = Result<Bytes, E>> + Send + 'static
where
    S: Stream<Item = Result<Bytes, E>> + Send + 'static,
    E: Send + 'static,
{
    stream.map(move |item| {
        if let Ok(ref bytes) = item {
            counter.fetch_add(bytes.len() as u64, Ordering::Relaxed);
        }
        item
    })
}

/// Background task that prints `X transferred` to stderr roughly every
/// 500 ms while a transfer is in flight, but only when stderr is a tty.
struct ProgressHandle {
    handle: Option<tokio::task::JoinHandle<()>>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    is_tty: bool,
}

impl ProgressHandle {
    fn finish(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            // Best-effort wait so the final newline lands before the next
            // print. We don't propagate the JoinError.
            h.abort();
        }
        if self.is_tty {
            // Clear the line and emit a newline so subsequent stderr output
            // starts cleanly.
            eprint!("\r\x1b[K");
        }
    }
}

fn spawn_progress(counter: Arc<AtomicU64>, verb: &'static str) -> ProgressHandle {
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let is_tty = std::io::stderr().is_terminal();
    if !is_tty {
        return ProgressHandle {
            handle: None,
            stop,
            is_tty,
        };
    }
    let stop_inner = stop.clone();
    let handle = tokio::spawn(async move {
        let start = Instant::now();
        loop {
            if stop_inner.load(Ordering::SeqCst) {
                break;
            }
            let bytes = counter.load(Ordering::Relaxed);
            let elapsed = start.elapsed().as_secs_f64().max(0.001);
            let rate = bytes as f64 / elapsed;
            eprint!(
                "\r{} {} ({}/s)\x1b[K",
                verb,
                format_bytes(bytes),
                format_bytes(rate as u64)
            );
            let _ = std::io::Write::flush(&mut std::io::stderr());
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
    ProgressHandle {
        handle: Some(handle),
        stop,
        is_tty,
    }
}

/// Format a byte count in human-friendly units (binary prefixes).
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut idx = 0;
    while value >= 1024.0 && idx + 1 < UNITS.len() {
        value /= 1024.0;
        idx += 1;
    }
    if idx == 0 {
        format!("{} {}", bytes, UNITS[idx])
    } else {
        format!("{value:.1} {}", UNITS[idx])
    }
}
