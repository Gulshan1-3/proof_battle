#!/bin/bash

# Kill previous cargo run (if needed)
pkill -f "target/debug/your-binary-name" 2>/dev/null

# Start the server
gnome-terminal -- bash -c "cargo run; exec bash"

# Wait for server to start
sleep 2

# Open two clients in two separate terminals
gnome-terminal -- bash -c "wscat -c ws://localhost:3000/ws; exec bash"
gnome-terminal -- bash -c "wscat -c ws://localhost:3000/ws; exec bash"
