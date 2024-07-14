#!/bin/bash

# Kill any previous instances of the server and CLI app
pkill -f yafbds
pkill -f yafbdsctl

export RUST_LOG=info

# Start the server in the background
./target/debug/yafbds &

sleep 3
# Run the CLI application
./target/debug/yafbdsctl list
