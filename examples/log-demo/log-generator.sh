#!/bin/bash

# Log generator script for testing scotty logs functionality
# This script outputs messages to both stdout and stderr at regular intervals

echo "Starting log generator..."
echo "Log generator started at $(date)" >&2

counter=1

while true; do
    # Generate timestamp
    timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    # Every 3rd message goes to stderr, others to stdout
    if [ $((counter % 3)) -eq 0 ]; then
        echo "[$timestamp] ERROR: This is error message #$counter from log-generator" >&2
    else
        echo "[$timestamp] INFO: This is info message #$counter from log-generator"
    fi

    # Every 5th message is a longer multi-line message
    if [ $((counter % 5)) -eq 0 ]; then
        echo "[$timestamp] INFO: Multi-line message #$counter"
        echo "  ├── This is line 2 of the multi-line message"
        echo "  ├── This is line 3 with some JSON: {\"counter\": $counter, \"type\": \"demo\"}"
        echo "  └── End of multi-line message #$counter"
    fi

    # Every 10th message is a warning
    if [ $((counter % 10)) -eq 0 ]; then
        echo "[$timestamp] WARN: This is warning message #$counter - something might be wrong!" >&2
    fi

    # Every 20th message simulates an exception
    if [ $((counter % 20)) -eq 0 ]; then
        echo "[$timestamp] ERROR: Exception occurred in log-generator!" >&2
        echo "  Stack trace simulation:" >&2
        echo "    at log-generator.sh:line_$counter" >&2
        echo "    at main_loop (log-generator.sh:$(($LINENO - 5)))" >&2
        echo "  Error details: Counter reached $counter" >&2
    fi

    # Progress indicator every 50 messages
    if [ $((counter % 50)) -eq 0 ]; then
        echo "[$timestamp] INFO: Log generator progress: $counter messages generated"
    fi

    counter=$((counter + 1))

    # Sleep for 2 seconds between messages
    sleep 2
done