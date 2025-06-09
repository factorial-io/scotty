# Middleware Examples

This directory contains examples of how to use the new `middlewares` feature in Scotty with Traefik.

## Overview

The `middlewares` field in your `.scotty.yml` file allows you to specify custom Traefik middlewares that will be applied to your services. These middlewares are applied in addition to any built-in middlewares (basic auth and robots protection).

## Middleware Order

Middlewares are applied in the following order:

1. Built-in basic auth middleware (if `basic_auth` is configured)
2. Built-in robots middleware (if `disallow_robots` is true)
3. Custom middlewares (in the order specified in the `middlewares` array)

## Examples

### Example 1: Rate Limiting and CORS

```yaml
# .scotty.yml
public_services:
  - service: web
    port: 80
domain: my-app.example.com
time_to_live: !Days 7
basic_auth: null
disallow_robots: true
middlewares:
  - rate-limit
  - cors-headers
environment:
  NODE_ENV: production
```

**Generated Traefik labels:**
```
traefik.http.routers.web--my-app-0.middlewares: web--my-app--robots,rate-limit,cors-headers
```

### Example 2: Authentication + Security Headers

```yaml
# .scotty.yml
public_services:
  - service: api
    port: 3000
domain: api.example.com
time_to_live: !Days 30
basic_auth:
  - "admin"
  - "secretpassword"
disallow_robots: true
middlewares:
  - security-headers
  - api-rate-limit
environment:
  API_KEY: "{{ .Env.API_KEY }}"
```

**Generated Traefik labels:**
```
traefik.http.routers.api--my-app-0.middlewares: api--my-app--basic-auth,api--my-app--robots,security-headers,api-rate-limit
```

### Example 3: Custom Middlewares Only

```yaml
# .scotty.yml
public_services:
  - service: frontend
    port: 8080
domain: frontend.example.com
time_to_live: !Days 7
basic_auth: null
disallow_robots: false  # No built-in middlewares
middlewares:
  - compress
  - custom-headers
  - redirect-scheme
environment:
  REACT_APP_API_URL: "https://api.example.com"
```

**Generated Traefik labels:**
```
traefik.http.routers.frontend--my-app-0.middlewares: compress,custom-headers,redirect-scheme
```

### Example 4: Multiple Services with Different Middlewares

```yaml
# .scotty.yml
public_services:
  - service: web
    port: 80
  - service: api
    port: 3000
domain: my-app.example.com
time_to_live: !Days 14
basic_auth: null
disallow_robots: true
middlewares:
  - global-rate-limit
  - security-headers
environment:
  DATABASE_URL: "postgres://user:pass@db:5432/myapp"
```

**Generated Traefik labels:**
```
# For web service:
traefik.http.routers.web--my-app-0.middlewares: web--my-app--robots,global-rate-limit,security-headers

# For api service:
traefik.http.routers.api--my-app-0.middlewares: api--my-app--robots,global-rate-limit,security-headers
```

## Common Middleware Use Cases

### Rate Limiting
- `rate-limit`: Basic rate limiting
- `api-rate-limit`: API-specific rate limiting
- `ddos-protection`: Advanced DDoS protection

### Security
- `security-headers`: Add security headers (HSTS, CSP, etc.)
- `cors-headers`: Configure CORS policies
- `ip-whitelist`: Restrict access by IP address

### Performance
- `compress`: Enable gzip compression
- `cache-headers`: Set appropriate cache headers
- `cdn-headers`: Configure CDN-specific headers

### Redirects & Rewrites
- `redirect-scheme`: HTTP to HTTPS redirects
- `www-redirect`: WWW to non-WWW redirects
- `path-rewrite`: URL path rewriting

## Prerequisites

Before using custom middlewares, you need to:

1. Define the middlewares in your Traefik configuration
2. Ensure the middleware names match exactly what you specify in `middlewares`
3. Make sure your Traefik instance can access the middleware definitions

## Traefik Middleware Configuration Example

Here's how you might define some of these middlewares in your Traefik configuration:

```yaml
# traefik.yml or docker-compose.yml
http:
  middlewares:
    rate-limit:
      rateLimit:
        burst: 100
        average: 50
    
    security-headers:
      headers:
        customRequestHeaders:
          X-Forwarded-Proto: "https"
        customResponseHeaders:
          X-Frame-Options: "DENY"
          X-Content-Type-Options: "nosniff"
    
    cors-headers:
      headers:
        accessControlAllowMethods:
          - "GET"
          - "POST"
          - "PUT"
          - "DELETE"
        accessControlAllowOriginList:
          - "https://example.com"
        accessControlMaxAge: 100
        addVaryHeader: true
```

## Notes

- Middleware names are case-sensitive
- The order of middlewares in the array matters
- Built-in middlewares (basic auth, robots) are always applied before custom middlewares
- If a middleware doesn't exist in Traefik, the service may fail to start
- You can use the same middleware across multiple apps by referencing the same name