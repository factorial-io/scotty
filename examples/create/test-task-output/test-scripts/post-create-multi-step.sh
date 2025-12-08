#!/bin/sh
# Post-create script with multiple steps and detailed output
# This tests that all output is captured during app creation

echo "=== POST-CREATE INITIALIZATION START ==="
echo ""
echo "[INFO] Testing task output streaming during app creation"
echo "[INFO] Script version: 1.0.0"
echo "[INFO] Timestamp: $(date -Iseconds)"
echo ""

echo "[STEP 1/5] Checking web root setup..."
sleep 0.3
if [ -f /usr/share/nginx/html/index.html ]; then
    echo "  ✓ Web root configured"
    echo "  ✓ index.html found"
else
    echo "  ✗ Web root not configured"
fi
echo ""

echo "[STEP 2/5] Verifying nginx configuration..."
sleep 0.3
if [ -f /etc/nginx/nginx.conf ]; then
    echo "  ✓ Nginx config exists"
else
    echo "  ✗ Nginx config missing"
fi
echo ""

echo "[STEP 3/5] Checking test scripts availability..."
sleep 0.3
if [ -d /test-scripts ]; then
    echo "  ✓ Test scripts volume mounted"
    echo "  ✓ Scripts available: $(ls /test-scripts | wc -l)"
else
    echo "  ✗ Test scripts not mounted"
fi
echo ""

echo "[STEP 4/5] Environment variable check..."
sleep 0.3
echo "  ✓ SCOTTY__APP_NAME: ${SCOTTY__APP_NAME:-not-set}"
echo "  ✓ SCOTTY__PUBLIC_URL__WEB: ${SCOTTY__PUBLIC_URL__WEB:-not-set}"
echo ""

echo "[STEP 5/5] Final initialization..."
sleep 0.3
echo "  ✓ All initialization steps completed"
echo ""

echo "=== POST-CREATE INITIALIZATION END ==="
echo "✓ Post-create action completed successfully"
echo "Exit code: 0"
exit 0
