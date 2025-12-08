#!/bin/sh
# Post-run script with quick verification
# This tests output streaming for the run action

echo "=== POST-RUN QUICK VERIFICATION START ==="
echo ""
echo "[INFO] Quick health check after container start"
echo "[INFO] Timestamp: $(date -Iseconds)"
echo ""

echo "[CHECK 1/3] Nginx process..."
sleep 0.2
if pgrep nginx > /dev/null 2>&1; then
    echo "  ✓ Nginx is running"
else
    echo "  ✗ Nginx not running"
fi

echo "[CHECK 2/3] Web root..."
sleep 0.2
if [ -d /usr/share/nginx/html ]; then
    echo "  ✓ Web root accessible"
else
    echo "  ✗ Web root not found"
fi

echo "[CHECK 3/3] Configuration..."
sleep 0.2
if [ -f /etc/nginx/nginx.conf ]; then
    echo "  ✓ Nginx configured"
else
    echo "  ✗ Configuration missing"
fi

echo ""
echo "=== POST-RUN QUICK VERIFICATION END ==="
echo "✓ Post-run action completed"
echo "Exit code: 0"
exit 0
