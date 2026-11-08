use axum::http::Request;
use sha2::{Digest, Sha256};
use tower_governor::key_extractor::KeyExtractor;
use tower_governor::GovernorError;

/// Extract rate limit key from bearer token in Authorization header
///
/// This extractor is used for authenticated API endpoints to rate limit
/// per user/token rather than per IP address.
///
/// Uses SHA256 hashing to avoid token collision attacks that could occur
/// with simple truncation.
#[derive(Clone, Copy, Debug)]
pub struct BearerTokenExtractor;

impl KeyExtractor for BearerTokenExtractor {
    type Key = String;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        req.headers()
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(|token| {
                // Hash the token with SHA256 to create a consistent rate limit key
                // This prevents collision attacks that could occur with truncation
                let mut hasher = Sha256::new();
                hasher.update(token.as_bytes());
                let hash = hasher.finalize();
                // Convert to hex string (64 characters)
                hash.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>()
            })
            .ok_or_else(|| {
                // Record extraction error metric
                super::metrics::record_extractor_error();
                GovernorError::UnableToExtractKey
            })
    }
}

/// Rate limit key of the client address for MCP client registration: the
/// first `X-Forwarded-For` entry, else `X-Real-IP`, else the peer address,
/// else one shared key. Never fails, so a request without any address still
/// counts against a (shared) budget instead of erroring.
#[derive(Clone, Copy, Debug)]
pub struct ClientAddressExtractor;

impl KeyExtractor for ClientAddressExtractor {
    type Key = String;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        let header = |name: &str| {
            req.headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split(',').next())
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
        };
        Ok(header("x-forwarded-for")
            .or_else(|| header("x-real-ip"))
            .or_else(|| {
                req.extensions()
                    .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                    .map(|info| info.0.ip().to_string())
            })
            .unwrap_or_else(|| "unknown".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    #[test]
    fn test_client_address_extractor_fallbacks() {
        let key = |headers: &[(&str, &str)]| {
            let mut req = Request::builder();
            for (k, v) in headers {
                req = req.header(*k, *v);
            }
            ClientAddressExtractor
                .extract(&req.body(Body::empty()).unwrap())
                .unwrap()
        };
        assert_eq!(
            key(&[
                ("x-forwarded-for", "203.0.113.5, 10.0.0.1"),
                ("x-real-ip", "1.1.1.1")
            ]),
            "203.0.113.5"
        );
        assert_eq!(key(&[("x-real-ip", "1.1.1.1")]), "1.1.1.1");
        assert_eq!(key(&[]), "unknown");
    }

    #[test]
    fn test_bearer_token_extractor_success() {
        let req = Request::builder()
            .header(
                "authorization",
                "Bearer my-super-secret-token-12345678901234567890",
            )
            .body(Body::empty())
            .unwrap();

        let extractor = BearerTokenExtractor;
        let key = extractor.extract(&req).unwrap();

        // Should produce a 64-character hex SHA256 hash
        assert_eq!(key.len(), 64);

        // Verify it's a valid hex string
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_bearer_token_extractor_consistent_hash() {
        // Same token should produce same hash
        let token = "Bearer test-token-123";

        let req1 = Request::builder()
            .header("authorization", token)
            .body(Body::empty())
            .unwrap();

        let req2 = Request::builder()
            .header("authorization", token)
            .body(Body::empty())
            .unwrap();

        let extractor = BearerTokenExtractor;
        let key1 = extractor.extract(&req1).unwrap();
        let key2 = extractor.extract(&req2).unwrap();

        assert_eq!(key1, key2, "Same token should produce same hash");
    }

    #[test]
    fn test_bearer_token_extractor_different_tokens() {
        // Different tokens should produce different hashes
        let req1 = Request::builder()
            .header("authorization", "Bearer token1")
            .body(Body::empty())
            .unwrap();

        let req2 = Request::builder()
            .header("authorization", "Bearer token2")
            .body(Body::empty())
            .unwrap();

        let extractor = BearerTokenExtractor;
        let key1 = extractor.extract(&req1).unwrap();
        let key2 = extractor.extract(&req2).unwrap();

        assert_ne!(
            key1, key2,
            "Different tokens should produce different hashes"
        );
    }

    #[test]
    fn test_bearer_token_extractor_no_header() {
        let req = Request::builder().body(Body::empty()).unwrap();

        let extractor = BearerTokenExtractor;
        let result = extractor.extract(&req);

        assert!(result.is_err());
    }

    #[test]
    fn test_bearer_token_extractor_wrong_format() {
        let req = Request::builder()
            .header("authorization", "Basic dXNlcjpwYXNz")
            .body(Body::empty())
            .unwrap();

        let extractor = BearerTokenExtractor;
        let result = extractor.extract(&req);

        assert!(result.is_err());
    }
}
