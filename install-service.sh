#!/bin/sh
# Install systemd service for currents
# This script installs the service but doesn't start or enable it

set -e

# Check if we're in the project directory
if [ ! -f "currents.service" ]; then
    echo "Error: currents.service file not found"
    echo "Make sure you're running this script from the project root directory"
    exit 1
fi

# Get the binary path from cargo
BINARY_PATH=$(which currents 2>/dev/null || echo "")
if [ -z "$BINARY_PATH" ]; then
    echo "Error: currents binary not found in PATH"
    echo "Make sure to run 'cargo install --path .' or 'cargo install currents' first"
    exit 1
fi

# Get the directory containing the binary
BINARY_DIR=$(dirname "$BINARY_PATH")

# Create systemd service file
SERVICE_FILE="/etc/systemd/system/currents@.service"

echo "Installing systemd service to $SERVICE_FILE"
echo "Binary location: $BINARY_PATH"

# Copy the service file and update the ExecStart path
sed "s|ExecStart=.*|ExecStart=$BINARY_PATH|" currents.service | sudo tee "$SERVICE_FILE" > /dev/null

echo "Service installed successfully!"
echo ""
echo "To use the service:"
echo "1. Create your config: ~/.config/currents/config.toml"
echo "2. Enable for your user: sudo systemctl enable currents@\$USER"
echo "3. Start the service: sudo systemctl start currents@\$USER"
echo "4. Check status: sudo systemctl status currents@\$USER"
echo ""
echo "Note: The service is installed but not started or enabled."
echo "You need to manually enable and start it for your user."
