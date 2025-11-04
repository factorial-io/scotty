# .scottyignore Example Application

This example demonstrates how to use `.scottyignore` files to control which files are uploaded when creating a new app with Scotty.

## What's in this example?

### Files that WILL be uploaded:
- `docker-compose.yml` - Docker Compose configuration
- `html/index.html` - Web content
- `.scottyignore` - The ignore file itself
- `README.md` - This file

### Files that WILL NOT be uploaded (filtered by .scottyignore):
- `debug.log` - Matches `*.log` pattern
- `.env` - Matches `.env` pattern (contains fake secrets)
- `cache.tmp` - Matches `*.tmp` pattern

## How to test this example

1. **Start the Scotty server:**
   ```bash
   cd /path/to/scotty
   SCOTTY__API__AUTH_MODE=dev cargo run --bin scotty
   ```

2. **Create the app with scottyctl:**
   ```bash
   cd examples/create/nginx-with-scottyignore
   scottyctl app:create scottyignore-demo ./docker-compose.yml --service web:80
   ```

3. **Verify files were filtered:**
   - The app should be created successfully
   - Check the server logs - you should see messages like:
     ```
     Ignoring file (scottyignore): "debug.log"
     Ignoring file (scottyignore): ".env"
     Ignoring file (scottyignore): "cache.tmp"
     ```

4. **Access the app:**
   - The app will be available at the URL shown after creation
   - You should see the HTML page explaining the .scottyignore feature

## Understanding .scottyignore patterns

The `.scottyignore` file in this directory contains comprehensive examples of different pattern types:

### Basic patterns:
- `*.log` - Ignore all files ending in .log
- `.env` - Ignore the .env file
- `*.tmp` - Ignore all temporary files

### Directory patterns:
- `node_modules/` - Ignore entire directory
- `target/` - Ignore build artifacts
- `.cache/` - Ignore cache directory

### Nested patterns:
- `**/*.bak` - Ignore .bak files in any subdirectory
- `**/.DS_Store` - Ignore .DS_Store in any location

### Negation (re-include):
- `!important.log` - Re-include this file even if *.log is ignored

### Comments:
- Lines starting with `#` are comments

## Automatic exclusions

Even without a `.scottyignore` file, Scotty automatically excludes:
- `.DS_Store` (macOS system files)
- `.git/` directory

## Learn more

- Full documentation: `docs/content/cli.md`
- Pattern syntax: Same as gitignore
- Implementation: `scottyctl/src/utils/files.rs`

## Testing the filtering

You can manually test the file collection without creating an app:

```bash
cd examples/create/nginx-with-scottyignore
RUST_LOG=info cargo run --bin scottyctl -- app:create test-app ./docker-compose.yml --service web:80
```

Watch the logs for "Ignoring file (scottyignore)" messages to see which files are filtered.

## Troubleshooting

**All files are being uploaded:**
- Make sure `.scottyignore` exists in the same directory as `docker-compose.yml`
- Check for syntax errors in `.scottyignore`
- Verify patterns match the files you want to ignore

**Files I need are being ignored:**
- Use negation patterns with `!` to re-include files
- Check if a parent directory pattern is matching
- Ensure the pattern is specific enough

**Pattern not working as expected:**
- Patterns are relative to the folder containing `docker-compose.yml`
- Use `**` for matching in subdirectories
- Check for typos or extra whitespace
