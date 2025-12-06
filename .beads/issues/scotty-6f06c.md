---
title: Implement automatic OAuth token refresh when expired
status: open
priority: 2
issue_type: feature
created_at: 2025-12-06T14:32:42.344071+00:00
updated_at: 2025-12-06T14:32:42.344071+00:00
---

# Description

Location: scottyctl/src/api.rs:133 (existing TODO comment)

Currently, when an OAuth token is expired, users must manually re-authenticate with auth:login. We should implement automatic token refresh using the refresh_token if available.

## Considerations

1. **When to refresh:**
   - On API call receiving 401 response?
   - Proactively before expiration (check expires_at)?
   - Both approaches?

2. **Refresh token storage:**
   - Currently StoredToken has refresh_token: Option<String>
   - Need to ensure refresh token is properly stored during login
   - Security implications of storing refresh tokens

3. **Refresh flow:**
   - Use OIDC token endpoint with grant_type=refresh_token
   - Handle refresh token expiration (fall back to re-auth)
   - Update stored token with new access_token and refresh_token

4. **User experience:**
   - Should refresh be silent or notify user?
   - What to do if refresh fails? (prompt for re-auth)

5. **API integration:**
   - Update get_auth_token() to attempt refresh on expired token
   - Add retry logic for 401 responses with token refresh

## Related Files

- scottyctl/src/auth/storage.rs - StoredToken definition
- scottyctl/src/auth/device_flow.rs - OAuth flow implementation
- scottyctl/src/api.rs - get_auth_token() function

## References

Related to GH#607, but separate concern. Token validation should work first, then add automatic refresh.
