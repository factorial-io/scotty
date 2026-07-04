use std::sync::Once;

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode, Uri};
use url::Url;

use crate::app_state::SharedAppState;
use crate::static_files::serve_embedded_file;

use scotty_core::apps::app_data::AppStatus;

/// Build a Response with no-cache headers to prevent browsers and proxies
/// from caching redirect or error responses for stopped apps.
fn no_cache_response() -> axum::http::response::Builder {
    Response::builder()
        .header("Cache-Control", "no-store, no-cache, must-revalidate")
        .header("Pragma", "no-cache")
        .header("Expires", "0")
}

/// Extract the hostname from the request's Host header.
/// Handles both plain hostnames, host:port pairs, and bracketed IPv6 addresses.
fn extract_hostname(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|h| {
            let h = h.trim();
            if let Some(bracket_end) = h.find(']') {
                // IPv6: "[::1]:port" or "[::1]" → "[::1]"
                h[..=bracket_end].to_string()
            } else if let Some(colon) = h.rfind(':') {
                // host:port → check if port part is numeric
                if h[colon + 1..].chars().all(|c| c.is_ascii_digit()) {
                    h[..colon].to_string()
                } else {
                    h.to_string()
                }
            } else {
                h.to_string()
            }
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
        // Only redirect to the landing page if the app is actually stopped.
        if app.status != AppStatus::Stopped {
            let message = match app.status {
                AppStatus::Running => {
                    tracing::warn!(
                        "App '{}' is Running but reached catch-all for domain '{}'. \
                         Possible load balancer routing misconfiguration.",
                        app.name,
                        hostname,
                    );
                    "<h1>Service Temporarily Unavailable</h1>\
                     <p>The application appears to be running but routing has not updated yet. \
                     This may indicate a load balancer configuration issue. \
                     Please refresh in a few seconds.</p>"
                }
                _ => {
                    tracing::info!(
                        "App '{}' matched domain '{}' but status is {} (not Stopped). Returning 503.",
                        app.name,
                        hostname,
                        app.status,
                    );
                    "<h1>Service Starting</h1>\
                     <p>The application is starting up. \
                     Please refresh in a few seconds.</p>"
                }
            };
            return no_cache_response()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("content-type", "text/html; charset=utf-8")
                .header("Retry-After", "5")
                .body(Body::from(message))
                .expect("BUG: response builder failed");
        }

        let scheme = if state.settings.traefik.use_tls {
            "https"
        } else {
            "http"
        };
        // Preserve the full path and query string so the user returns to the
        // exact page they originally requested. Note: if the original URL
        // contains sensitive query parameters (e.g. reset tokens), they will
        // appear in the redirect Location header and may be logged by proxies.
        // This is an accepted trade-off for deep-path preservation.
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
                .expect("BUG: response builder failed");
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
        .expect("BUG: response builder failed")
}

/// Check if the given hostname matches Scotty's own domain.
fn is_scotty_domain(state: &SharedAppState, hostname: &str) -> bool {
    let Some(base_url) = state.settings.api.configured_base_url() else {
        // Nothing configured: assume it's Scotty (don't redirect).
        // Log a warning once so operators notice the missing configuration.
        static WARN_UNCONFIGURED: Once = Once::new();
        WARN_UNCONFIGURED.call_once(|| {
            tracing::warn!(
                "api.base_url is not configured. All requests will be served as \
                 Scotty frontend — per-app domains will not redirect to the \
                 landing page."
            );
        });
        return true;
    };

    match Url::parse(&base_url).ok().and_then(|url| {
        url.host_str()
            .map(|host| hostname.eq_ignore_ascii_case(host))
    }) {
        Some(matches) => matches,
        None => {
            // Configured but not a parseable absolute URL: we cannot tell
            // Scotty's own domain apart from app domains. Serve everything as
            // Scotty rather than 404-ing its own UI; the startup warning from
            // base_url_config_warnings() points the operator at the broken
            // value.
            static WARN_MALFORMED: Once = Once::new();
            WARN_MALFORMED.call_once(|| {
                tracing::warn!(
                    "the configured public base URL ('{}') is not a valid absolute \
                     URL. All requests will be served as Scotty frontend — per-app \
                     domains will not redirect to the landing page.",
                    base_url
                );
            });
            true
        }
    }
}

/// Extract the base URL for constructing redirect targets. Returns None for
/// unparseable values so we never emit a broken Location header.
fn get_scotty_base_url(state: &SharedAppState) -> Option<String> {
    state.settings.api.configured_base_url().filter(|base_url| {
        Url::parse(base_url)
            .map(|url| url.host_str().is_some())
            .unwrap_or(false)
    })
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
    fn test_extract_hostname_ipv6_with_port() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, "[::1]:8080".parse().unwrap());
        assert_eq!(extract_hostname(&headers), Some("[::1]".to_string()));
    }

    #[test]
    fn test_extract_hostname_ipv6_without_port() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, "[::1]".parse().unwrap());
        assert_eq!(extract_hostname(&headers), Some("[::1]".to_string()));
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
