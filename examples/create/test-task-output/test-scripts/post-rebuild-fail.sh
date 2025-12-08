#!/bin/sh
# Post-rebuild script that FAILS
# This script outputs predictable messages then exits with error code
# Used to test error handling and output streaming on failure

echo "=== POST-REBUILD FAIL SCRIPT START ==="
echo ""
echo "[INFO] This script intentionally fails to test error handling"
echo "[INFO] Script version: 1.0.0"
echo "[INFO] Timestamp: $(date -Iseconds)"
echo ""

echo "[STEP 1/3] Starting validation..."
sleep 0.5
echo "  ✓ Initial checks passed"
echo "  ✓ Environment loaded"
echo ""

echo "[STEP 2/3] Running critical checks..."
sleep 0.5
echo "  ✓ Check 2.1: Configuration file readable"
echo "  ✓ Check 2.2: Dependencies available"
echo "  ✓ Check 2.3: Network accessible"
echo ""

echo "[STEP 3/3] Final validation (this will fail)..."
sleep 0.5
echo "  ✗ CRITICAL ERROR: Simulated failure for testing"
echo "  ✗ Configuration validation failed"
echo "  ✗ Expected value: 'success', Got: 'failure'"
echo ""
echo "Error details:"
echo "  - Test case: Intentional failure"
echo "  - Purpose: Verify error output is streamed correctly"
echo "  - Expected behavior: scottyctl should show this message"
echo ""

echo "=== POST-REBUILD FAIL SCRIPT END ==="
echo "✗ Post-rebuild action failed"
echo "Exit code: 1"
exit 1
