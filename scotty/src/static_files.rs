//! This module provides functionality to serve embedded frontend files.
//!
//! The frontend files from the "frontend/build" directory are embedded into the executable
//! at compile time using the `include_dir` crate. This ensures the application
//! is self-contained and doesn't require external frontend files.

use axum::body::Body;
use axum::http::Response;
use include_dir::{include_dir, Dir};

// Include the frontend/build directory at compile time
static FRONTEND_DIR: Dir<'_> = include_dir!("frontend/build");

/// Handler for serving static files from the embedded directory
pub async fn serve_embedded_file(uri: axum::http::Uri) -> Response<Body> {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    // Try to find the file in the embedded directory
    match FRONTEND_DIR.get_file(path) {
        Some(file) => {
            let mime_type = mime_guess::from_path(path).first_or_text_plain();
            Response::builder()
                .header("content-type", mime_type.as_ref())
                .body(Body::from(file.contents()))
                .unwrap()
        }
        // Well-known URIs (RFC 8615) are machine-read, never SPA routes: an
        // HTML page with 200 makes clients (e.g. MCP OAuth discovery) choke.
        None if path.starts_with(".well-known/") => not_found(),
        None => {
            // If the file doesn't exist, try to serve index.html (for SPA routing)
            match FRONTEND_DIR.get_file("index.html") {
                Some(file) => Response::builder()
                    .header("content-type", "text/html")
                    .body(Body::from(file.contents()))
                    .unwrap(),
                None => not_found(),
            }
        }
    }
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(404)
        .body(Body::from("File not found"))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unknown_well_known_paths_are_404_not_the_spa() {
        for path in [
            "/.well-known/oauth-protected-resource/mcp",
            "/.well-known/oauth-authorization-server",
        ] {
            let res = serve_embedded_file(path.parse().unwrap()).await;
            assert_eq!(res.status(), 404, "{path}");
        }
    }
}
