#!/bin/sh
# Post-rebuild script that SUCCEEDS
# This script outputs predictable messages to verify streaming

echo "=== POST-REBUILD SUCCESS SCRIPT START ==="
echo ""
echo "[INFO] Testing task output streaming during rebuild"
echo "[INFO] Script version: 1.0.0"
echo "[INFO] Timestamp: $(date -Iseconds)"
echo "[INFO] Hostname: $(hostname)"
echo ""

echo "[STEP 1/5] Checking environment variables..."
sleep 0.5
echo "  ✓ SCOTTY__APP_NAME: ${SCOTTY__APP_NAME:-not-set}"
echo "  ✓ SCOTTY__PUBLIC_URL__WEB: ${SCOTTY__PUBLIC_URL__WEB:-not-set}"
echo "  ✓ Environment variables available"
echo ""

echo "[STEP 2/5] Verifying nginx is running..."
sleep 0.5
if pgrep nginx > /dev/null 2>&1; then
    echo "  ✓ Nginx process found"
    echo "  ✓ Process count: $(pgrep nginx | wc -l)"
else
    echo "  ⚠ Nginx process not found (container might still be starting)"
fi
echo ""

echo "[STEP 3/5] Checking web root..."
sleep 0.5
if [ -f /usr/share/nginx/html/index.html ]; then
    echo "  ✓ index.html exists"
    echo "  ✓ Size: $(wc -c < /usr/share/nginx/html/index.html) bytes"
    echo "  ✓ Last modified: $(stat -c %y /usr/share/nginx/html/index.html 2>/dev/null || stat -f %Sm /usr/share/nginx/html/index.html)"
else
    echo "  ✗ index.html not found"
fi
echo ""

echo "[STEP 4/5] Testing internal connectivity..."
sleep 0.5
if command -v wget > /dev/null 2>&1; then
    echo "  ✓ wget available, testing localhost..."
    if wget -q -O /dev/null --timeout=2 http://localhost:80/; then
        echo "  ✓ HTTP request to localhost successful"
    else
        echo "  ⚠ HTTP request failed (service might still be starting)"
    fi
elif command -v curl > /dev/null 2>&1; then
    echo "  ✓ curl available, testing localhost..."
    if curl -s -f -o /dev/null --max-time 2 http://localhost:80/; then
        echo "  ✓ HTTP request to localhost successful"
    else
        echo "  ⚠ HTTP request failed (service might still be starting)"
    fi
else
    echo "  ⚠ Neither curl nor wget available in nginx:alpine"
    echo "  ℹ Skipping connectivity test"
fi
echo ""

echo "[STEP 5/5] Final checks..."
sleep 0.5
echo "  ✓ All post-rebuild checks passed"
echo "  ✓ Application is ready"
echo ""

echo "=== POST-REBUILD SUCCESS SCRIPT END ==="
echo "✓ Post-rebuild action completed successfully"
echo "Exit code: 0"
exit 0
