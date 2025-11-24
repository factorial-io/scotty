# File Upload Status

This document shows which files will be uploaded vs. ignored when running:
```bash
scottyctl app:create demo ./docker-compose.yml --service web:80
```

## ✅ Files that WILL be uploaded:

```
docker-compose.yml          # Required Docker Compose configuration
html/                       # Web content directory
html/index.html            # HTML content
.scottyignore              # The ignore file itself is uploaded
README.md                  # Documentation
FILES.md                   # This file
```

**Total files uploaded: 5 files + .scottyignore**

## ❌ Files that WILL NOT be uploaded:

```
.env                       # Matches: .env
debug.log                  # Matches: *.log
cache.tmp                  # Matches: *.tmp
node_modules/              # Matches: node_modules/
node_modules/example-package.js  # Inside ignored directory
```

**Total files ignored: 4 files (+ 1 inside ignored dir)**

## Pattern Matching Details

| File | Pattern | Reason |
|------|---------|--------|
| `.env` | `.env` | Exact match - prevents uploading secrets |
| `debug.log` | `*.log` | Extension match - excludes log files |
| `cache.tmp` | `*.tmp` | Extension match - excludes temporary files |
| `node_modules/` | `node_modules/` | Directory match - excludes dependencies |

## Test This Example

Run with logging to see which files are ignored:

```bash
cd /path/to/scotty
SCOTTY__API__AUTH_MODE=dev cargo run --bin scotty

# In another terminal:
cd examples/create/nginx-with-scottyignore
RUST_LOG=info cargo run --bin scottyctl -- app:create scottyignore-demo \
  ./docker-compose.yml --service web:80
```

Look for these log messages:
```
INFO scottyctl::utils::files: Ignoring file (scottyignore): ".env"
INFO scottyctl::utils::files: Ignoring file (scottyignore): "debug.log"
INFO scottyctl::utils::files: Ignoring file (scottyignore): "cache.tmp"
INFO scottyctl::utils::files: Ignoring file (scottyignore parent): "node_modules/example-package.js"
```

## Size Comparison

Without `.scottyignore`:
- All files would be uploaded (~5KB)

With `.scottyignore`:
- Only needed files are uploaded (~3KB)
- Faster upload
- No sensitive data (.env) uploaded
- Cleaner deployment
