#!/usr/bin/env bash
#
# TSP Server Installation Script
# This script installs tspserver as a systemd service
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
INSTALL_DIR="/opt/tsp"
CONFIG_DIR="/etc/tsp"
SERVICE_NAME="tsp"
BINARY_NAME="tspserver"
PORT=9000
NUM_INSTANCES=1

# Usage function
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --install-dir DIR    Installation directory (default: /opt/tsp)"
    echo "  --config-dir DIR    Config directory (default: /etc/tsp)"
    echo "  --service-name NAME  Service name (default: tsp)"
    echo "  --port PORT          Port for this instance (default: 9000)"
    echo "  --workers NUM        Number of instances to install (default: 1)"
    echo "                       Installs instances on ports: PORT, PORT+1, ..."
    echo "  --uninstall         Uninstall the service(s)"
    echo "  -h, --help          Show this help message"
    echo ""
    exit 0
}

# Parse arguments
UNINSTALL=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --install-dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        --config-dir)
            CONFIG_DIR="$2"
            shift 2
            ;;
        --service-name)
            SERVICE_NAME="$2"
            shift 2
            ;;
        --port)
            PORT="$2"
            shift 2
            ;;
        --workers)
            NUM_INSTANCES="$2"
            shift 2
            ;;
        --uninstall)
            UNINSTALL=true
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_PATH="$SCRIPT_DIR/$BINARY_NAME"

# Uninstall function
uninstall() {
    echo -e "${YELLOW}Uninstalling $SERVICE_NAME services...${NC}"

    # Uninstall all instances
    for i in $(seq 0 $((NUM_INSTANCES - 1))); do
        INSTANCE_PORT=$((PORT + i))
        INSTANCE_NAME="${SERVICE_NAME}-${INSTANCE_PORT}"

        echo "Stopping $INSTANCE_NAME service..."
        sudo systemctl stop "$INSTANCE_NAME" || true
        sudo systemctl disable "$INSTANCE_NAME" || true

        # Remove service file
        if [ -f "/etc/systemd/system/$INSTANCE_NAME.service" ]; then
            echo "Removing service file for port $INSTANCE_PORT..."
            sudo rm -f /etc/systemd/system/$INSTANCE_NAME.service
        fi
    done

    # Reload systemd
    echo "Reloading systemd..."
    sudo systemctl daemon-reload

    echo -e "${GREEN}Uninstallation complete!${NC}"
    exit 0
}

# Check if running as root for installation
if [ "$UNINSTALL" = false ] && [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Please run as root or with sudo for installation${NC}"
    exit 1
fi

# Check if binary exists
if [ "$UNINSTALL" = false ] && [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}Error: tspserver binary not found in $SCRIPT_DIR${NC}"
    echo "Please run this script from the directory containing tspserver binary"
    exit 1
fi

# If uninstall, just run uninstall
if [ "$UNINSTALL" = true ]; then
    uninstall
fi

# Installation
echo -e "${GREEN}Installing $SERVICE_NAME with $NUM_INSTANCES instance(s)...${NC}"

# Create installation directory
echo "Creating installation directory: $INSTALL_DIR"
sudo mkdir -p "$INSTALL_DIR"

# Copy binary
echo "Copying binary..."
sudo cp "$BINARY_PATH" "$INSTALL_DIR/"
sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"

# Create config directory
echo "Creating config directory: $CONFIG_DIR"
sudo mkdir -p "$CONFIG_DIR"

# Copy config template if it doesn't exist
if [ ! -f "$CONFIG_DIR/config.jsonc" ]; then
    echo "Creating default config file..."
    if [ -f "$SCRIPT_DIR/config.example.jsonc" ]; then
        # Replace default paths with actual install path
        sed "s|\./www|$INSTALL_DIR/www|g; s|\.logs/|$INSTALL_DIR/logs/|g" \
            "$SCRIPT_DIR/config.example.jsonc" | sudo tee "$CONFIG_DIR/config.jsonc" > /dev/null
        echo -e "${GREEN}Config file created at $CONFIG_DIR/config.jsonc${NC}"
    else
        echo -e "${YELLOW}Warning: config.example.jsonc not found, creating empty config...${NC}"
        sudo tee "$CONFIG_DIR/config.jsonc" > /dev/null << EOF
{
  "root": "$INSTALL_DIR/www",
  "port": $PORT,
  "dev": false
}
EOF
    fi
else
    echo "Config file already exists at $CONFIG_DIR/config.jsonc"
fi

# Create log directory
LOG_DIR="$INSTALL_DIR/logs"
echo "Creating log directory: $LOG_DIR"
mkdir -p "$LOG_DIR"

# Create system log directory for systemd output
SYSTEM_LOG_DIR="/var/log/tsp"
echo "Creating system log directory: $SYSTEM_LOG_DIR"
sudo mkdir -p "$SYSTEM_LOG_DIR"

# Install multiple instances
for i in $(seq 0 $((NUM_INSTANCES - 1))); do
    INSTANCE_PORT=$((PORT + i))
    INSTANCE_NAME="${SERVICE_NAME}-${INSTANCE_PORT}"
    INSTANCE_CONFIG="$CONFIG_DIR/config-${INSTANCE_PORT}.jsonc"

    echo ""
    echo "--- Installing instance $INSTANCE_NAME on port $INSTANCE_PORT ---"

    # Create instance config
    echo "Creating config for port $INSTANCE_PORT..."
    if [ -f "$SCRIPT_DIR/config.example.jsonc" ]; then
        sed "s|\./www|$INSTALL_DIR/www|g; s|\.logs/|$INSTALL_DIR/logs/|g; s|\"port\": [0-9000]*|\"port\": $INSTANCE_PORT|g" \
            "$SCRIPT_DIR/config.example.jsonc" | sudo tee "$INSTANCE_CONFIG" > /dev/null
    else
        sudo tee "$INSTANCE_CONFIG" > /dev/null << EOF
{
  "root": "$INSTALL_DIR/www",
  "port": $INSTANCE_PORT,
  "dev": false
}
EOF
    fi

    # Create systemd service file
    SERVICE_FILE="/etc/systemd/system/$INSTANCE_NAME.service"
    echo "Creating systemd service file for port $INSTANCE_PORT..."

    sudo tee "$SERVICE_FILE" > /dev/null << EOF
[Unit]
Description=TSP (TypeScript Server Page) Server - Port $INSTANCE_PORT
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/$BINARY_NAME --config $INSTANCE_CONFIG
Restart=always
RestartSec=10

# Environment
Environment="HOME=$HOME"
Environment="PATH=/usr/local/bin:/usr/bin:/bin"

# Logging - capture all output to files
StandardOutput=append:$SYSTEM_LOG_DIR/${INSTANCE_NAME}-output.log
StandardError=append:$SYSTEM_LOG_DIR/${INSTANCE_NAME}-error.log

[Install]
WantedBy=multi-user.target
EOF

    # Enable and start service
    echo "Enabling and starting $INSTANCE_NAME..."
    sudo systemctl daemon-reload
    sudo systemctl enable "$INSTANCE_NAME"
    sudo systemctl start "$INSTANCE_NAME"
done

# Check status
echo ""
echo -e "${GREEN}Installation complete!${NC}"
echo ""
echo "Service status:"
for i in $(seq 0 $((NUM_INSTANCES - 1))); do
    INSTANCE_PORT=$((PORT + i))
    INSTANCE_NAME="${SERVICE_NAME}-${INSTANCE_PORT}"
    echo ""
    echo "=== $INSTANCE_NAME (port $INSTANCE_PORT) ==="
    sudo systemctl status "$INSTANCE_NAME" --no-pager || true
done
echo ""
echo -e "${GREEN}All services installed successfully!${NC}"
echo ""
echo "Useful commands:"
for i in $(seq 0 $((NUM_INSTANCES - 1))); do
    INSTANCE_PORT=$((PORT + i))
    INSTANCE_NAME="${SERVICE_NAME}-${INSTANCE_PORT}"
    echo "  sudo systemctl status $INSTANCE_NAME   # Check status"
    echo "  sudo systemctl restart $INSTANCE_NAME # Restart"
    echo "  journalctl -u $INSTANCE_NAME -f       # View logs"
    echo ""
done
