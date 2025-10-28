use axum::http::Request;
use tower_governor::key_extractor::KeyExtractor;
use tower_governor::GovernorError;

/// Extract rate limit key from bearer token in Authorization header
///
/// This extractor is used for authenticated API endpoints to rate limit
/// per user/token rather than per IP address.
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
                // Use first 32 chars as key (enough for uniqueness, avoids storing full token)
                token.chars().take(32).collect()
            })
            .ok_or(GovernorError::UnableToExtractKey)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

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

        // Should extract first 32 chars
        assert_eq!(key, "my-super-secret-token-1234567890");
        assert_eq!(key.len(), 32);
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
