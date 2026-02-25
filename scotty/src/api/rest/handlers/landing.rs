use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode, Uri};
use url::Url;

use crate::app_state::SharedAppState;
use crate::static_files::serve_embedded_file;

use scotty_core::settings::api_server::DEFAULT_FRONTEND_BASE_URL;

/// Build a Response with no-cache headers to prevent browsers and proxies
/// from caching redirect or error responses for stopped apps.
fn no_cache_response() -> axum::http::response::Builder {
    Response::builder()
        .header("Cache-Control", "no-store, no-cache, must-revalidate")
        .header("Pragma", "no-cache")
        .header("Expires", "0")
}

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
        let path_and_query = uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or(uri.path());
        let return_url = format!("{}://{}{}", scheme, hostname, path_and_query);

        if let Some(base_url) = get_scotty_base_url(&state) {
            let redirect_url = format!(
                "{}/landing/{}?return_url={}",
                base_url.trim_end_matches('/'),
                urlencoding::encode(&app.name),
                urlencoding::encode(&return_url),
            );

            return no_cache_response()
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
    no_cache_response()
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
                return hostname.eq_ignore_ascii_case(host);
            }
        }
    }

    // Fallback: check against oauth frontend_base_url
    if let Ok(url) = Url::parse(&state.settings.api.oauth.frontend_base_url) {
        if let Some(host) = url.host_str() {
            return hostname.eq_ignore_ascii_case(host);
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

    // Fallback to oauth frontend_base_url, but skip the default localhost value
    // since it indicates no explicit configuration was provided.
    let frontend_url = &state.settings.api.oauth.frontend_base_url;
    if !frontend_url.is_empty() && frontend_url != DEFAULT_FRONTEND_BASE_URL {
        return Some(frontend_url.clone());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header;

    #[test]
    fn test_extract_hostname_with_port() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, "example.com:8080".parse().unwrap());
        assert_eq!(extract_hostname(&headers), Some("example.com".to_string()));
    }

    #[test]
    fn test_extract_hostname_without_port() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, "example.com".parse().unwrap());
        assert_eq!(extract_hostname(&headers), Some("example.com".to_string()));
    }

    #[test]
    fn test_extract_hostname_missing() {
        let headers = HeaderMap::new();
        assert_eq!(extract_hostname(&headers), None);
    }

    #[test]
    fn test_no_cache_response_has_correct_headers() {
        let response = no_cache_response()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();

        assert_eq!(
            response.headers().get("Cache-Control").unwrap(),
            "no-store, no-cache, must-revalidate"
        );
        assert_eq!(response.headers().get("Pragma").unwrap(), "no-cache");
        assert_eq!(response.headers().get("Expires").unwrap(), "0");
    }
}
