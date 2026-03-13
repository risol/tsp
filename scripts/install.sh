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
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
INSTALL_DIR="/opt/tsp"
CONFIG_DIR="/etc/tsp"
SERVICE_NAME="tsp"
BINARY_NAME="tspserver"
PORT=9000
NUM_WORKERS=1

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_PATH="$SCRIPT_DIR/$BINARY_NAME"

# Read input with default
read_with_default() {
    local prompt="$1"
    local default="$2"
    local value

    if [ -n "$default" ]; then
        read -p "$prompt [$default]: " value
        echo "${value:-$default}"
    else
        read -p "$prompt: " value
        echo "$value"
    fi
}

# Yes/No question
ask_yes_no() {
    local prompt="$1"
    local default="$2"
    local yn

    while true; do
        if [ -n "$default" ]; then
            read -p "$prompt [$default]: " yn
            yn="${yn:-$default}"
        else
            read -p "$prompt [y/n]: " yn
        fi

        case "$yn" in
            [Yy]|[Yy][Ee][Ss]) return 0 ;;
            [Nn]|[Nn][Oo]) return 1 ;;
            *) echo "Please answer yes or no" ;;
        esac
    done
}

# Interactive mode
interactive_install() {
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}         TSP Server Interactive Installation${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
    echo ""

    # Welcome
    echo -e "${GREEN}Welcome to TSP Server installation!${NC}"
    echo ""
    echo "This script will install TSP as a systemd service."
    echo ""

    # Check if running as root
    if [ "$EUID" -ne 0 ]; then
        echo -e "${YELLOW}Note: Installation requires root privileges.${NC}"
        echo "You'll be prompted for sudo password when needed."
        echo ""
    fi

    # Installation directory
    echo -e "${BLUE}─────────────────────────────────────────────────────────${NC}"
    echo -e "${BLUE}Step 1: Installation Directory${NC}"
    echo ""
    INSTALL_DIR=$(read_with_default "Installation directory" "$INSTALL_DIR")
    echo ""

    # Service name
    echo -e "${BLUE}─────────────────────────────────────────────────────────${NC}"
    echo -e "${BLUE}Step 2: Service Configuration${NC}"
    echo ""
    SERVICE_NAME=$(read_with_default "Service name" "$SERVICE_NAME")
    echo ""

    # Port configuration
    echo -e "${BLUE}─────────────────────────────────────────────────────────${NC}"
    echo -e "${BLUE}Step 3: Port Configuration${NC}"
    echo ""
    PORT=$(read_with_default "Port number" "9000")
    echo ""

    # Workers configuration (Linux only)
    echo -e "${BLUE}─────────────────────────────────────────────────────────${NC}"
    echo -e "${BLUE}Step 4: Workers Configuration${NC}"
    echo ""
    echo "Workers use SO_REUSEPORT to share the same port (Linux/macOS)."
    echo "On Windows, workers mode is not supported."
    echo ""
    if ask_yes_no "Enable workers mode?" "n"; then
        NUM_WORKERS=$(read_with_default "Number of workers" "4")
        echo ""
        echo -e "${GREEN}Will enable workers mode with $NUM_WORKERS workers${NC}"
    else
        NUM_WORKERS=1
        echo ""
        echo -e "${GREEN}Will run in single-process mode${NC}"
    fi

    # Summary
    echo ""
    echo -e "${BLUE}─────────────────────────────────────────────────────────${NC}"
    echo -e "${BLUE}Installation Summary${NC}"
    echo ""
    echo -e "  Installation directory: ${GREEN}$INSTALL_DIR${NC}"
    echo -e "  Service name:          ${GREEN}$SERVICE_NAME${NC}"
    echo -e "  Port:                  ${GREEN}$PORT${NC}"
    echo -e "  Workers:               ${GREEN}$NUM_WORKERS${NC}"
    echo ""

    # Confirm
    if ! ask_yes_no "Start installation?" "y"; then
        echo ""
        echo -e "${YELLOW}Installation cancelled.${NC}"
        exit 0
    fi

    echo ""
    echo -e "${GREEN}Starting installation...${NC}"
    echo ""
}

# Uninstall function
uninstall() {
    echo ""
    echo -e "${YELLOW}Uninstalling TSP services...${NC}"
    echo ""

    # Ask for service name
    SERVICE_NAME=$(read_with_default "Service name to uninstall" "tsp")

    # Find all services matching the name
    echo "Finding services matching '$SERVICE_NAME'..."
    SERVICES=$(systemctl list-units --type=service --all --no-legend | grep "$SERVICE_NAME" | awk '{print $1}' || true)

    if [ -z "$SERVICES" ]; then
        echo -e "${YELLOW}No services found matching '$SERVICE_NAME'${NC}"
        exit 0
    fi

    echo "Found services:"
    echo "$SERVICES"
    echo ""

    if ! ask_yes_no "Uninstall these services?" "y"; then
        echo -e "${YELLOW}Uninstallation cancelled.${NC}"
        exit 0
    fi

    for service in $SERVICES; do
        echo "Stopping $service..."
        sudo systemctl stop "$service" 2>/dev/null || true
        sudo systemctl disable "$service" 2>/dev/null || true
        echo "Removing service file..."
        sudo rm -f "/etc/systemd/system/$service"
    done

    # Reload systemd
    echo "Reloading systemd..."
    sudo systemctl daemon-reload

    echo -e "${GREEN}Uninstallation complete!${NC}"
}

# Parse arguments
UNINSTALL=false
INTERACTIVE=false

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
            NUM_WORKERS="$2"
            shift 2
            ;;
        -i|--interactive)
            INTERACTIVE=true
            shift
            ;;
        --uninstall)
            UNINSTALL=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --install-dir DIR    Installation directory (default: /opt/tsp)"
            echo "  --config-dir DIR    Config directory (default: /etc/tsp)"
            echo "  --service-name NAME  Service name (default: tsp)"
            echo "  --port PORT         Port number (default: 9000)"
            echo "  --workers NUM        Number of workers (default: 1, use 0 to disable)"
            echo "  -i, --interactive  Interactive installation mode"
            echo "  --uninstall         Uninstall the service(s)"
            echo "  -h, --help         Show this help message"
            echo ""
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use -h or --help for usage information"
            exit 1
            ;;
    esac
done

# Run in interactive mode if -i flag is passed
if [ "$INTERACTIVE" = true ]; then
    interactive_install
fi

# If uninstall, just run uninstall
if [ "$UNINSTALL" = true ]; then
    uninstall
fi

# Check if binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}Error: tspserver binary not found in $SCRIPT_DIR${NC}"
    echo "Please run this script from the directory containing tspserver binary"
    exit 1
fi

# Installation
echo -e "${GREEN}Installing $SERVICE_NAME on port $PORT with $NUM_WORKERS worker(s)...${NC}"

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

# Build ExecStart command with workers flag
EXEC_START="$INSTALL_DIR/$BINARY_NAME --config $CONFIG_DIR/config.jsonc --port $PORT"
if [ "$NUM_WORKERS" -gt 1 ]; then
    EXEC_START="$EXEC_START --workers $NUM_WORKERS"
fi

# Create single systemd service file
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
echo "Creating systemd service file..."

sudo tee "$SERVICE_FILE" > /dev/null << EOF
[Unit]
Description=TSP (TypeScript Server Page) Server
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$INSTALL_DIR
ExecStart=$EXEC_START
Restart=always
RestartSec=10

# Environment
Environment="HOME=$HOME"
Environment="PATH=/usr/local/bin:/usr/bin:/bin"

# Logging - capture all output to files
StandardOutput=append:$SYSTEM_LOG_DIR/${SERVICE_NAME}-output.log
StandardError=append:$SYSTEM_LOG_DIR/${SERVICE_NAME}-error.log

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
echo "Enabling and starting $SERVICE_NAME..."
sudo systemctl daemon-reload
sudo systemctl enable "$SERVICE_NAME"
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
echo "  sudo systemctl restart $SERVICE_NAME   # Restart"
echo "  journalctl -u $SERVICE_NAME -f         # View logs"
echo ""
