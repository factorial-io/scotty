//! File transfer endpoints.
//!
//! `GET  /api/v1/apps/{app_id}/services/{service}/files?path=<container-path>` streams a
//! tar archive of `container-path` from the targeted service's container.
//!
//! `PUT  /api/v1/apps/{app_id}/services/{service}/files?path=<container-path>` extracts a
//! tar archive received in the request body into `container-path` inside the
//! container.
//!
//! Both endpoints stream their bodies via chunked transfer encoding. A
//! counting wrapper aborts transfers that exceed
//! `Settings.files.max_transfer_size` with `413 Payload Too Large`.

// The internal validation/resolution helpers return `Result<_, Response>`
// to keep the handler bodies linear. `Response` is intentionally large
// because it carries an `axum::body::Body`; boxing it would obscure the
// flow without measurable benefit.
#![allow(clippy::result_large_err)]

use std::pin::Pin;
use std::task::{Context, Poll};

use axum::body::{Body, Bytes};
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use bollard::query_parameters::{
    DownloadFromContainerOptionsBuilder, UploadToContainerOptionsBuilder,
};
use futures_util::{Stream, StreamExt};
use scotty_types::files::{FileTransferError, FileTransferErrorCode, FileTransferQuery};

use crate::app_state::SharedAppState;

/// Maps a `FileTransferErrorCode` to the HTTP status the endpoint should
/// respond with.
fn status_for_code(code: FileTransferErrorCode) -> StatusCode {
    match code {
        FileTransferErrorCode::InvalidPath => StatusCode::BAD_REQUEST,
        FileTransferErrorCode::ServiceNotRunning => StatusCode::CONFLICT,
        FileTransferErrorCode::Forbidden => StatusCode::FORBIDDEN,
        FileTransferErrorCode::NotFound => StatusCode::NOT_FOUND,
        FileTransferErrorCode::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        FileTransferErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Build a JSON error response with the appropriate status code.
fn error_response(code: FileTransferErrorCode, message: impl Into<String>) -> Response {
    let status = status_for_code(code);
    let body = FileTransferError::new(code, message);
    (status, Json(body)).into_response()
}

/// Validate that a container path is non-empty and absolute.
fn validate_path(raw: &str) -> Result<(), Response> {
    if raw.is_empty() {
        return Err(error_response(
            FileTransferErrorCode::InvalidPath,
            "path query parameter must not be empty",
        ));
    }
    if !raw.starts_with('/') {
        return Err(error_response(
            FileTransferErrorCode::InvalidPath,
            "path query parameter must be an absolute path starting with '/'",
        ));
    }
    // Reject `.` and `..` traversal components. Docker resolves the path
    // inside the container's own namespace so this is not a host-side
    // vulnerability, but such components almost always mean the path resolves
    // somewhere the user did not intend. Rejecting them keeps the behaviour
    // predictable.
    if raw
        .split('/')
        .any(|component| component == ".." || component == ".")
    {
        return Err(error_response(
            FileTransferErrorCode::InvalidPath,
            "path query parameter must not contain '.' or '..' traversal components",
        ));
    }
    Ok(())
}

/// Map a Bollard error to a `(code, message)` pair for the JSON response.
///
/// Docker's filesystem-archive endpoint returns `404` both when the container
/// itself does not exist and when the requested path inside the container
/// cannot be found. We surface that as `NotFound`. `409` from the docker
/// daemon means "container not running" and is mapped accordingly. Anything
/// else is treated as an internal error.
fn map_bollard_error(err: &bollard::errors::Error) -> (FileTransferErrorCode, String) {
    match err {
        bollard::errors::Error::DockerResponseServerError {
            status_code,
            message,
        } => match *status_code {
            404 => (FileTransferErrorCode::NotFound, message.clone()),
            409 => (FileTransferErrorCode::ServiceNotRunning, message.clone()),
            _ => (FileTransferErrorCode::Internal, message.clone()),
        },
        other => (FileTransferErrorCode::Internal, other.to_string()),
    }
}

/// Resolve the target container id for `(app_id, service)`. Returns an HTTP
/// response on error.
async fn resolve_container_id(
    state: &SharedAppState,
    app_id: &str,
    service: &str,
) -> Result<String, Response> {
    let Some(app_data) = state.apps.get_app(app_id).await else {
        return Err(error_response(
            FileTransferErrorCode::NotFound,
            format!("app '{app_id}' not found"),
        ));
    };

    let Some(container_state) = app_data.find_container_by_service(service) else {
        return Err(error_response(
            FileTransferErrorCode::NotFound,
            format!("service '{service}' not found in app '{app_id}'"),
        ));
    };

    let Some(container_id) = container_state.id.clone() else {
        return Err(error_response(
            FileTransferErrorCode::ServiceNotRunning,
            format!("service '{service}' has no running container"),
        ));
    };

    if !container_state.is_running() {
        return Err(error_response(
            FileTransferErrorCode::ServiceNotRunning,
            format!("service '{service}' container is not running"),
        ));
    }

    Ok(container_id)
}

/// Stream adapter that counts bytes flowing through it and yields an error
/// once `max` bytes have been observed. The error variant is the upstream
/// item's `Err` type (`bollard::errors::Error` for downloads, `axum::Error`
/// for uploads), wrapped into a generic boxed error.
///
/// Sized as a real `Stream` impl (rather than `stream::unfold`) so the inner
/// stream does not need to be `Unpin`.
pub(crate) struct CountingStream<S, E> {
    inner: S,
    transferred: u64,
    max: u64,
    exceeded: bool,
    _marker: std::marker::PhantomData<fn() -> E>,
}

impl<S, E> CountingStream<S, E> {
    fn new(inner: S, max: u64) -> Self {
        Self {
            inner,
            transferred: 0,
            max,
            exceeded: false,
            _marker: std::marker::PhantomData,
        }
    }
}

/// Sentinel error type produced when the configured transfer size is
/// exceeded. We wrap it in `std::io::Error` so it can be carried by either
/// the bollard error type (for download) or by axum's `Body` (upload),
/// neither of which we want to depend on directly here.
#[derive(Debug, thiserror::Error)]
#[error("transfer exceeded configured maximum of {limit} bytes")]
pub(crate) struct SizeLimitExceeded {
    pub limit: u64,
}

impl<S, E> Stream for CountingStream<S, E>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: From<std::io::Error>,
{
    type Item = Result<Bytes, E>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.exceeded {
            return Poll::Ready(None);
        }
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                self.transferred = self.transferred.saturating_add(bytes.len() as u64);
                if self.transferred > self.max {
                    self.exceeded = true;
                    let err = std::io::Error::other(SizeLimitExceeded { limit: self.max });
                    Poll::Ready(Some(Err(E::from(err))))
                } else {
                    Poll::Ready(Some(Ok(bytes)))
                }
            }
            other => other,
        }
    }
}

/// Download a tar archive of a path inside a service's container.
#[utoipa::path(
    get,
    path = "/api/v1/apps/{app_id}/services/{service}/files",
    params(
        ("app_id" = String, Path, description = "App identifier"),
        ("service" = String, Path, description = "Service name within the app"),
        ("path" = String, Query, description = "Absolute container path to download"),
    ),
    responses(
        (status = 200, description = "Tar archive of the requested container path", content_type = "application/x-tar"),
        (status = 400, description = "Invalid path query parameter", body = FileTransferError),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Caller lacks the `view` permission", body = FileTransferError),
        (status = 404, description = "App, service, or path not found", body = FileTransferError),
        (status = 409, description = "Service container is not running", body = FileTransferError),
        (status = 413, description = "Configured maximum transfer size exceeded", body = FileTransferError),
        (status = 500, description = "Docker daemon error", body = FileTransferError),
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "scotty-service"
)]
pub async fn download_files_handler(
    Path((app_id, service)): Path<(String, String)>,
    Query(query): Query<FileTransferQuery>,
    State(state): State<SharedAppState>,
) -> Response {
    if let Err(resp) = validate_path(&query.path) {
        return resp;
    }

    let container_id = match resolve_container_id(&state, &app_id, &service).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let options = DownloadFromContainerOptionsBuilder::default()
        .path(&query.path)
        .build();

    let max = state.settings.files.max_transfer_size;
    let inner = state
        .docker
        .download_from_container(&container_id, Some(options));
    let counted = CountingStream::<_, bollard::errors::Error>::new(Box::pin(inner), max);

    // Map size-limit errors into a proper 413 response by wrapping the
    // stream. The HTTP status itself is already sent as 200 with chunked
    // transfer encoding by the time we know the size; we therefore surface
    // the limit through a stream error that aborts the response. Clients
    // observe a truncated stream + reset; this matches the spec which
    // states that downloads exceeding the limit are aborted.
    let body_stream = counted.map(|item| match item {
        Ok(bytes) => Ok::<_, std::io::Error>(bytes),
        Err(err) => {
            tracing::warn!(error = %err, "file download stream aborted");
            Err(std::io::Error::other(err.to_string()))
        }
    });

    let body = Body::from_stream(body_stream);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-tar")
        .body(body)
        .unwrap_or_else(|err| {
            error_response(
                FileTransferErrorCode::Internal,
                format!("failed to build response: {err}"),
            )
        })
}

/// Upload a tar archive and extract it into a path inside a service's
/// container.
#[utoipa::path(
    put,
    path = "/api/v1/apps/{app_id}/services/{service}/files",
    params(
        ("app_id" = String, Path, description = "App identifier"),
        ("service" = String, Path, description = "Service name within the app"),
        ("path" = String, Query, description = "Absolute container path to extract into"),
    ),
    request_body(
        content = Vec<u8>,
        content_type = "application/x-tar",
        description = "Tar archive to extract inside the container"
    ),
    responses(
        (status = 204, description = "Upload accepted and extracted"),
        (status = 400, description = "Invalid path query parameter", body = FileTransferError),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Caller lacks the `manage` permission", body = FileTransferError),
        (status = 404, description = "App, service, or path not found", body = FileTransferError),
        (status = 409, description = "Service container is not running", body = FileTransferError),
        (status = 413, description = "Configured maximum transfer size exceeded", body = FileTransferError),
        (status = 500, description = "Docker daemon error", body = FileTransferError),
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "scotty-service"
)]
pub async fn upload_files_handler(
    Path((app_id, service)): Path<(String, String)>,
    Query(query): Query<FileTransferQuery>,
    State(state): State<SharedAppState>,
    request: axum::extract::Request,
) -> Response {
    if let Err(resp) = validate_path(&query.path) {
        return resp;
    }

    let container_id = match resolve_container_id(&state, &app_id, &service).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let options = UploadToContainerOptionsBuilder::default()
        .path(&query.path)
        .build();

    let max = state.settings.files.max_transfer_size;

    // The incoming request body. Map the framework error type to
    // `std::io::Error` so the counting wrapper can synthesize a uniform
    // error variant when the size limit is exceeded.
    let body_stream =
        request
            .into_body()
            .into_data_stream()
            .map(|item| -> Result<Bytes, std::io::Error> {
                item.map_err(|e| std::io::Error::other(e.to_string()))
            });
    let counted = CountingStream::<_, std::io::Error>::new(Box::pin(body_stream), max);

    // Track whether we aborted for size before delegating, so we can return
    // the right error code if bollard surfaces an i/o failure that was
    // actually our limit kicking in. We use a shared flag updated as the
    // stream is consumed.
    let exceeded_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let exceeded_flag_inner = exceeded_flag.clone();
    let bollard_body = counted.map(move |item| match item {
        Ok(bytes) => Ok(bytes),
        Err(err) => {
            // Mark the size-limit case so we can convert the eventual
            // bollard error back into a 413 below.
            if err
                .get_ref()
                .and_then(|e| e.downcast_ref::<SizeLimitExceeded>())
                .is_some()
            {
                exceeded_flag_inner.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            Err(err)
        }
    });

    let body = bollard::body_try_stream(bollard_body);

    match state
        .docker
        .upload_to_container(&container_id, Some(options), body)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => {
            if exceeded_flag.load(std::sync::atomic::Ordering::SeqCst) {
                return error_response(
                    FileTransferErrorCode::PayloadTooLarge,
                    format!("upload exceeded configured maximum of {max} bytes"),
                );
            }
            let (code, message) = map_bollard_error(&err);
            tracing::warn!(error = %err, ?code, "file upload failed");
            error_response(code, message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_path_rejects_empty() {
        let resp = validate_path("").unwrap_err();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn validate_path_rejects_relative() {
        let resp = validate_path("etc/hostname").unwrap_err();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn validate_path_accepts_absolute() {
        assert!(validate_path("/etc/hostname").is_ok());
    }

    #[test]
    fn validate_path_rejects_traversal() {
        assert_eq!(
            validate_path("/etc/../etc/shadow").unwrap_err().status(),
            StatusCode::BAD_REQUEST
        );
        // A bare `.` component is rejected too.
        assert_eq!(
            validate_path("/./etc/passwd").unwrap_err().status(),
            StatusCode::BAD_REQUEST
        );
        // A path whose component merely contains dots (but is not a `.`/`..`
        // component itself) is still allowed.
        assert!(validate_path("/etc/..hidden").is_ok());
        assert!(validate_path("/var/log/app..log").is_ok());
        assert!(validate_path("/etc/.config").is_ok());
    }

    #[test]
    fn map_bollard_404_to_not_found() {
        let err = bollard::errors::Error::DockerResponseServerError {
            status_code: 404,
            message: "no such container".into(),
        };
        let (code, _) = map_bollard_error(&err);
        assert_eq!(code, FileTransferErrorCode::NotFound);
    }

    #[test]
    fn map_bollard_409_to_service_not_running() {
        let err = bollard::errors::Error::DockerResponseServerError {
            status_code: 409,
            message: "container not running".into(),
        };
        let (code, _) = map_bollard_error(&err);
        assert_eq!(code, FileTransferErrorCode::ServiceNotRunning);
    }

    #[test]
    fn map_bollard_other_to_internal() {
        let err = bollard::errors::Error::DockerResponseServerError {
            status_code: 500,
            message: "boom".into(),
        };
        let (code, _) = map_bollard_error(&err);
        assert_eq!(code, FileTransferErrorCode::Internal);
    }

    /// Drive a sequence of chunk sizes through a `CountingStream` with the
    /// given limit and return how many bytes were yielded successfully before
    /// (optionally) a `SizeLimitExceeded` error terminated the stream.
    async fn run_counting(chunks: &[usize], max: u64) -> (u64, bool) {
        let items: Vec<Result<Bytes, std::io::Error>> = chunks
            .iter()
            .map(|&n| Ok(Bytes::from(vec![0u8; n])))
            .collect();
        let inner = futures_util::stream::iter(items);
        let counted = CountingStream::<_, std::io::Error>::new(Box::pin(inner), max);

        let mut delivered = 0u64;
        let mut hit_limit = false;
        let collected: Vec<_> = counted.collect().await;
        for item in collected {
            match item {
                Ok(bytes) => delivered += bytes.len() as u64,
                Err(err) => {
                    assert!(
                        err.get_ref()
                            .and_then(|e| e.downcast_ref::<SizeLimitExceeded>())
                            .is_some(),
                        "error should be a SizeLimitExceeded sentinel"
                    );
                    hit_limit = true;
                }
            }
        }
        (delivered, hit_limit)
    }

    #[tokio::test]
    async fn counting_stream_passes_when_exactly_at_limit() {
        // Total of 10 bytes against a 10-byte limit must pass untouched.
        let (delivered, hit_limit) = run_counting(&[4, 6], 10).await;
        assert_eq!(delivered, 10);
        assert!(!hit_limit);
    }

    #[tokio::test]
    async fn counting_stream_aborts_one_byte_over() {
        // 11 bytes against a 10-byte limit: the chunk that crosses the limit
        // is replaced by the sentinel error and the stream stops.
        let (delivered, hit_limit) = run_counting(&[6, 5], 10).await;
        assert!(hit_limit);
        // The first 6-byte chunk is delivered; the second crosses the limit
        // and becomes an error rather than data.
        assert_eq!(delivered, 6);
    }

    #[tokio::test]
    async fn counting_stream_aborts_across_many_chunks() {
        // Several small chunks that only collectively exceed the limit.
        let (delivered, hit_limit) = run_counting(&[2, 2, 2, 2, 2, 2], 5).await;
        assert!(hit_limit);
        // 2 + 2 = 4 delivered; the third chunk takes the running total to 6
        // which is over the limit of 5.
        assert_eq!(delivered, 4);
    }

    #[tokio::test]
    async fn counting_stream_passes_under_limit() {
        let (delivered, hit_limit) = run_counting(&[1, 1, 1], 100).await;
        assert_eq!(delivered, 3);
        assert!(!hit_limit);
    }
}
