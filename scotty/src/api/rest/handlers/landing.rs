use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode, Uri};
use url::Url;

use crate::app_state::SharedAppState;
use crate::static_files::serve_embedded_file;

/// Extract the hostname from the request's Host header.
fn extract_hostname(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|h| {
            // Strip port if present (e.g., "localhost:21342" -> "localhost")
            h.split(':').next().unwrap_or(h).to_string()
        })
}

/// Fallback handler that inspects the Host header to decide whether to:
/// 1. Serve the embedded SvelteKit frontend (request is for Scotty's own domain)
/// 2. Redirect to the landing page (request is for a stopped app's domain)
/// 3. Return 404 (unknown domain)
///
/// This enables the "default backend" flow where Traefik routes unmatched
/// domains to Scotty, and Scotty redirects to its landing page.
pub async fn landing_or_frontend_handler(
    State(state): State<SharedAppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response<Body> {
    let hostname = match extract_hostname(&headers) {
        Some(h) => h,
        None => return serve_embedded_file(uri).await,
    };

    // If this is a request for Scotty's own domain, serve the frontend
    if is_scotty_domain(&state, &hostname) {
        return serve_embedded_file(uri).await;
    }

    // Check if this domain belongs to a known app
    if let Some(app) = state.apps.find_app_by_domain(&hostname).await {
        let scheme = if state.settings.traefik.use_tls {
            "https"
        } else {
            "http"
        };
        let return_url = format!("{}://{}{}", scheme, hostname, uri.path());

        if let Some(base_url) = get_scotty_base_url(&state) {
            let redirect_url = format!(
                "{}/landing/{}?return_url={}",
                base_url.trim_end_matches('/'),
                urlencoding::encode(&app.name),
                urlencoding::encode(&return_url),
            );

            return Response::builder()
                .status(StatusCode::FOUND)
                .header("Location", redirect_url)
                .body(Body::empty())
                .unwrap();
        }

        // base_url not configured -- can't redirect, serve frontend as fallback
        tracing::warn!(
            "Received request for app domain '{}' (app: '{}') but api.base_url is not configured. \
             Cannot redirect to landing page.",
            hostname,
            app.name
        );
        return serve_embedded_file(uri).await;
    }

    // Unknown domain -- not Scotty, not a known app
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("content-type", "text/html; charset=utf-8")
        .body(Body::from(
            "<h1>Not Found</h1><p>No application is configured for this domain.</p>",
        ))
        .unwrap()
}

/// Check if the given hostname matches Scotty's own domain.
fn is_scotty_domain(state: &SharedAppState, hostname: &str) -> bool {
    // Check against configured base_url
    if let Some(base_url) = &state.settings.api.base_url {
        if let Ok(url) = Url::parse(base_url) {
            if let Some(host) = url.host_str() {
                return hostname == host;
            }
        }
    }

    // Fallback: check against oauth frontend_base_url
    if let Ok(url) = Url::parse(&state.settings.api.oauth.frontend_base_url) {
        if let Some(host) = url.host_str() {
            return hostname == host;
        }
    }

    // If nothing is configured, assume it's Scotty (don't redirect)
    true
}

/// Extract the base URL for constructing redirect targets.
fn get_scotty_base_url(state: &SharedAppState) -> Option<String> {
    if let Some(base_url) = &state.settings.api.base_url {
        if !base_url.is_empty() {
            return Some(base_url.clone());
        }
    }

    // Fallback to oauth frontend_base_url
    let frontend_url = &state.settings.api.oauth.frontend_base_url;
    if !frontend_url.is_empty() && frontend_url != "http://localhost:21342" {
        return Some(frontend_url.clone());
    }

    None
}
