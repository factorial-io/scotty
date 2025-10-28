# Log Demo Example

This example provides a simple logging service for testing Scotty's unified logs functionality.

## What it does

- **log-generator**: Continuously generates log messages to both stdout and stderr
- **web-logger**: A second instance of the same logger (simulates a multi-service app)

## Log Output Features

The log generator produces various types of messages:

- **Regular INFO messages**: Every message to stdout
- **ERROR messages**: Every 3rd message goes to stderr
- **Multi-line messages**: Every 5th message spans multiple lines
- **WARNING messages**: Every 10th message is a warning to stderr
- **Exception simulation**: Every 20th message simulates a stack trace
- **Progress indicators**: Every 50th message shows progress

## Usage with Scotty

### 1. Create the app

```bash
scottyctl app:create log-demo --folder examples/log-demo --service log-generator:80 --service web-logger:80
```

### 2. View logs

```bash
# View recent logs for the log-generator service
scottyctl app:logs log-demo log-generator

# Follow logs in real-time
scottyctl app:logs log-demo log-generator --follow

# View logs with timestamps
scottyctl app:logs log-demo log-generator --timestamps

# View logs from the web-logger service
scottyctl app:logs log-demo web-logger

# View specific number of lines
scottyctl app:logs log-demo log-generator --lines 200
```

### 3. Test error scenarios

```bash
# Test with wrong service name (should show available services)
scottyctl app:logs log-demo wrong-service

# Test with non-existent app
scottyctl app:logs non-existent wrong-service
```

## Manual Testing

You can also run the services manually for testing:

```bash
# Build and run
cd examples/log-demo
docker-compose up --build

# View logs
docker-compose logs -f log-generator
docker-compose logs -f web-logger

# Clean up
docker-compose down
```

## Expected Output

The log generator will produce output like:

```
[2024-01-01 10:00:00] INFO: This is info message #1 from log-generator
[2024-01-01 10:00:02] INFO: This is info message #2 from log-generator
[2024-01-01 10:00:04] ERROR: This is error message #3 from log-generator
[2024-01-01 10:00:06] INFO: This is info message #4 from log-generator
[2024-01-01 10:00:08] INFO: Multi-line message #5
  ├── This is line 2 of the multi-line message
  ├── This is line 3 with some JSON: {"counter": 5, "type": "demo"}
  └── End of multi-line message #5
```

This provides a realistic testing environment for the unified output system, with both stdout and stderr messages, timestamps, and various message types that help verify the logs functionality works correctly.