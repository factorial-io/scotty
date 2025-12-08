# Task Output Streaming Test App

This is a minimal test application designed to verify that **all task output** (including post-action results) is properly streamed via WebSocket and displayed in scottyctl.

## Purpose

Tests the fix for the race condition bug where:
- Final status messages were cut off
- Post-action output wasn't visible in scottyctl
- Exit codes and error details were lost

## Structure

```
test-task-output/
├── docker-compose.yml           # Simple nginx:alpine service
├── .scotty.yml                  # Config using test-task-output blueprint
├── html/
│   └── index.html              # Static test page
├── test-scripts/
│   ├── post-create-multi-step.sh   # Multi-step initialization (5 steps)
│   ├── post-rebuild-success.sh     # Detailed rebuild checks (5 steps) ✓
│   ├── post-rebuild-fail.sh        # Intentional failure scenario ✗
│   └── post-run-quick.sh           # Quick health check (3 steps)
└── README.md                    # This file
```

## Blueprint Configuration

The blueprint is defined in `config/local.yaml` (git-ignored, not distributed):

```yaml
apps:
  blueprints:
    test-task-output:
      name: "Task Output Streaming Test"
      required_services: [web]
      public_services:
        web: 80
      actions:
        post_create:    # → post-create-multi-step.sh
        post_rebuild:   # → post-rebuild-success.sh
        post_run:       # → post-run-quick.sh
        test:fail:      # → post-rebuild-fail.sh (custom action)
```

## Manual Testing Scenarios

### Scenario 1: Create with Post-Action
Tests that post-create output is fully visible.

```bash
scottyctl app:create test-task-output --path examples/create/test-task-output
```

**Expected output:**
- Docker operations (pull, create, start)
- `=== POST-CREATE INITIALIZATION START ===`
- All 5 initialization steps with checkmarks
- `=== POST-CREATE INITIALIZATION END ===`
- Success message

### Scenario 2: Rebuild with Post-Action (Success)
Tests that post-rebuild output is fully visible, including all intermediate steps.

```bash
scottyctl app:rebuild test-task-output
```

**Expected output:**
- Docker operations (pull, build, stop, up)
- Container status changes
- `=== POST-REBUILD SUCCESS SCRIPT START ===`
- All 5 steps with detailed checks
- Environment variable values
- Nginx process verification
- `=== POST-REBUILD SUCCESS SCRIPT END ===`
- `✓ rebuild action for app test-task-output has been successfully completed!`

### Scenario 3: Run with Post-Action
Tests post-run quick verification.

```bash
scottyctl app:run test-task-output
```

**Expected output:**
- Docker operations (start)
- `=== POST-RUN QUICK VERIFICATION START ===`
- 3 quick health checks
- `=== POST-RUN QUICK VERIFICATION END ===`
- Success message

### Scenario 4: Custom Action - Intentional Failure
Tests that failure output and exit codes are properly displayed.

```bash
scottyctl app:action test-task-output test:fail
```

**Expected output:**
- Action execution starts
- `=== POST-REBUILD FAIL SCRIPT START ===`
- Steps 1-2 passing
- Step 3 failing with error details
- `✗ CRITICAL ERROR: Simulated failure for testing`
- `=== POST-REBUILD FAIL SCRIPT END ===`
- **`Failed: action test:fail on service web (exit code 1)`** ← This is the critical line!
- Error status in scottyctl

## Verification Checklist

When testing, verify that scottyctl output includes:

- ✅ All docker-compose output (pull, build, stop, up)
- ✅ All container status changes (Starting, Started, Stopping, Stopped)
- ✅ Post-action script start markers (`===`)
- ✅ All intermediate step output (including sleep delays)
- ✅ Environment variable values
- ✅ Post-action script end markers
- ✅ Final status message (success or failure)
- ✅ Exit code information on failure
- ✅ **No truncated output** - compare with web UI to ensure they match exactly

## What This Tests

### The Bug (Before Fix)
1. Post-action runs and fails
2. Error message added to task output buffer
3. `output_collection_active` set to `false` **before** message is sent
4. WebSocket stream ends immediately
5. Client aborts handler
6. Final error message lost → user doesn't see why it failed!

### The Fix (After Fix)
1. Post-action runs and fails
2. Error message added to task output buffer
3. Small yield to ensure write completes
4. WebSocket stream polls one more time and captures the message
5. **Then** `output_collection_active` set to `false`
6. Stream sends buffered message
7. Stream ends gracefully
8. Client receives end signal and cleans up
9. ✓ User sees complete output including the error!

## Comparison with Web UI

The scottyctl output should **exactly match** what you see in the web UI task output. If scottyctl is missing any lines that appear in the web UI, the bug is not fully fixed.

## Script Output Format

All scripts follow a consistent format for easy verification:

```
=== [SCRIPT TYPE] START ===

[INFO] Metadata about the script
[INFO] Timestamp, version, etc.

[STEP X/Y] Description...
  ✓ Success message
  ✗ Failure message
  ⚠ Warning message

=== [SCRIPT TYPE] END ===
✓/✗ Final status
Exit code: 0 or 1
```

## Troubleshooting

**If output is still cut off:**
1. Check server logs for timing issues
2. Compare scottyctl output with web UI output line-by-line
3. Verify WebSocket connection isn't closing early
4. Check that `output_collection_active` isn't being set too early

**If scripts don't execute:**
1. Verify scripts are executable: `ls -la test-scripts/`
2. Check docker volume mount is working
3. Verify blueprint is loaded in config/local.yaml
4. Check app_blueprint value in .scotty.yml matches blueprint name

## Development Notes

- Scripts use `/bin/sh` (not bash) for alpine compatibility
- Sleep delays simulate real-world processing time
- Each script outputs ~15-30 lines to test buffering
- Scripts are idempotent and safe to run multiple times
- No external dependencies required (only alpine busybox utilities)

## Related Files

- **Fix 1 (Client):** `scottyctl/src/api.rs` - WebSocket handler wait for stream end
- **Fix 2 (Server):** `scotty/src/docker/state_machine_handlers/context.rs` - Message before deactivation
- **Fix 3 (Server):** `scotty/src/tasks/output_streaming.rs` - Check output before ending stream
- **Blueprint:** `config/local.yaml` - Test blueprint definition (git-ignored)
