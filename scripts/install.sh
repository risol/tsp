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

# Usage function
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --install-dir DIR    Installation directory (default: /opt/tsp)"
    echo "  --config-dir DIR    Config directory (default: /etc/tsp)"
    echo "  --service-name NAME  Service name (default: tsp)"
    echo "  --uninstall         Uninstall the service"
    echo "  -h, --help         Show this help message"
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
    echo -e "${YELLOW}Uninstalling $SERVICE_NAME service...${NC}"

    # Stop and disable service
    if systemctl is-active --quiet "$SERVICE_NAME"; then
        echo "Stopping $SERVICE_NAME service..."
        sudo systemctl stop "$SERVICE_NAME" || true
    fi

    if systemctl is-enabled --quiet "$SERVICE_NAME"; then
        echo "Disabling $SERVICE_NAME service..."
        sudo systemctl disable "$SERVICE_NAME" || true
    fi

    # Remove service file
    if [ -f "/etc/systemd/system/$SERVICE_NAME.service" ]; then
        echo "Removing service file..."
        sudo rm -f /etc/systemd/system/$SERVICE_NAME.service
    fi

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
echo -e "${GREEN}Installing $SERVICE_NAME...${NC}"

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

# Copy config file if it doesn't exist
if [ ! -f "$CONFIG_DIR/config.jsonc" ]; then
    echo "Creating default config file..."
    if [ -f "$SCRIPT_DIR/config.example.jsonc" ]; then
        # Replace default root path "./www" with actual install path
        sed "s|\"./www\"|\"$INSTALL_DIR/www\"|g" "$SCRIPT_DIR/config.example.jsonc" | sudo tee "$CONFIG_DIR/config.jsonc" > /dev/null
        echo -e "${GREEN}Config file created at $CONFIG_DIR/config.jsonc${NC}"
    else
        echo -e "${YELLOW}Warning: config.example.jsonc not found, creating empty config...${NC}"
        sudo tee "$CONFIG_DIR/config.jsonc" > /dev/null << EOF
{
  "root": "$INSTALL_DIR/www",
  "port": 9000,
  "dev": false,
  "hotReload": true
}
EOF
    fi
else
    echo "Config file already exists at $CONFIG_DIR/config.jsonc"
fi

# Create log directory
LOG_DIR="/var/log/tsp"
echo "Creating log directory: $LOG_DIR"
sudo mkdir -p "$LOG_DIR"
sudo chown "$USER" "$LOG_DIR" 2>/dev/null || true

# Create systemd service file
SERVICE_FILE="/etc/systemd/system/$SERVICE_NAME.service"
echo "Creating systemd service file..."

sudo tee "$SERVICE_FILE" > /dev/null << EOF
[Unit]
Description=TSP (TypeScript Server Page) Server
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/$BINARY_NAME --config $CONFIG_DIR/config.jsonc
Restart=always
RestartSec=10

# Environment
Environment="HOME=$HOME"
Environment="PATH=/usr/local/bin:/usr/bin:/bin"

# Logging
StandardOutput=append:$LOG_DIR/access.log
StandardError=append:$LOG_DIR/error.log

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd
echo "Reloading systemd..."
sudo systemctl daemon-reload

# Enable and start service
echo "Enabling service..."
sudo systemctl enable "$SERVICE_NAME"

echo "Starting service..."
sudo systemctl start "$SERVICE_NAME"

# Check status
echo ""
echo -e "${GREEN}Installation complete!${NC}"
echo ""
echo "Service status:"
sudo systemctl status "$SERVICE_NAME" --no-pager || true
echo ""
echo -e "${GREEN}Service installed successfully!${NC}"
echo ""
echo "Useful commands:"
echo "  sudo systemctl status $SERVICE_NAME   # Check status"
echo "  sudo systemctl restart $SERVICE_NAME # Restart"
echo "  sudo systemctl stop $SERVICE_NAME    # Stop"
echo "  journalctl -u $SERVICE_NAME -f       # View logs"
