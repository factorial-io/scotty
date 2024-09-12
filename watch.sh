#!/bin/bash

# Kill any previous instances of the server and CLI app
pkill -f scotty
pkill -f scottyctl

export RUST_LOG=info

# Start the server in the background
./target/debug/scotty &

sleep 3
# Run the CLI application
./target/debug/scottyctl list
