#!/bin/bash

# ============================================================
# Kill processes occupying configured port (Bash)
# Supports: Linux, macOS
# ============================================================

set -e

# Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${CYAN}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

# ============================================================
# Function: Read port number from config file
# ============================================================
get_config_port() {
    local filepath="$1"

    if [ ! -f "$filepath" ]; then
        return 1
    fi

    # Remove JSONC comments and extract port
    # Method: use sed to remove comments, then grep/awk to extract port
    local port=$(sed 's|//.*||g; s|/\*[\s\S]*?\*/||g' "$filepath" | grep -o '"port"[[:space:]]*:[[:space:]]*[0-9]*' | grep -o '[0-9]*$' | head -1)

    if [ -n "$port" ]; then
        echo "$port"
        return 0
    fi

    return 1
}

# ============================================================
# Function: Kill process occupying specified port
# ============================================================
kill_process_on_port() {
    local port=$1

    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  Checking and cleaning port $port"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""

    # Detect OS type
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS: use lsof
        if command -v lsof &> /dev/null; then
            local pids=$(lsof -ti ":$port" 2>/dev/null || true)

            if [ -n "$pids" ]; then
                echo "Found processes occupying port $port:"

                for pid in $pids; do
                    if kill -0 "$pid" 2>/dev/null; then
                        # Get process info
                        if ps -p "$pid" -o comm= > /dev/null 2>&1; then
                            local cmd=$(ps -p "$pid" -o comm= 2>/dev/null | tail -1)
                            echo "  - PID: $pid, command: $cmd" >&2
                        else
                            echo "  - PID: $pid" >&2
                        fi

                        # Try graceful termination (SIGTERM)
                        log_info "Terminating PID $pid..."
                        kill "$pid" 2>/dev/null || true

                        # Wait for process to exit
                        sleep 1

                        # Check if process is still running
                        if kill -0 "$pid" 2>/dev/null; then
                            log_warning "Process still running, forcing termination..."
                            kill -9 "$pid" 2>/dev/null || true
                            sleep 1
                        fi

                        log_success "PID $pid terminated"
                    fi
                done

                echo ""
                log_success "Port $port cleaned"
            else
                log_success "Port $port is not occupied"
            fi
        else
            log_warning "lsof command not available, skipping check"
        fi

    elif [[ "$OSTYPE" == "linux"* ]]; then
        # Linux: use ss or lsof or netstat
        local pids=""

        # Prefer ss (modern Linux)
        if command -v ss &> /dev/null; then
            pids=$(ss -tnlp ":$port" 2>/dev/null | awk 'NR>1 {print $NF}' | cut -d',' -f1 | cut -d'=' -f2 | sort -u)
        # Fallback: use lsof
        elif command -v lsof &> /dev/null; then
            pids=$(lsof -ti ":$port" 2>/dev/null || true)
        # Last fallback: use netstat (requires root)
        elif command -v netstat &> /dev/null; then
            # Extract PID column
            pids=$(netstat -tnlp 2>/dev/null | grep ":$port " | awk 'NR>1 {print $7}' | cut -d'/' -f2 | sort -u)
        fi

        if [ -n "$pids" ]; then
            echo "Found processes occupying port $port:"

            for pid in $pids; do
                if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
                    # Get process info
                    if ps -p "$pid" -o pid,comm,cmd > /dev/null 2>&1; then
                        local info=$(ps -p "$pid" -o pid,comm --no-headers 2>/dev/null | head -1)
                        echo "  - $info" >&2
                    else
                        echo "  - PID: $pid" >&2
                    fi

                    # Try graceful termination (SIGTERM)
                    log_info "Terminating PID $pid..."
                    kill "$pid" 2>/dev/null || true

                    # Wait for process to exit
                    sleep 1

                    # Check if process is still running
                    if kill -0 "$pid" 2>/dev/null; then
                        log_warning "Process still running, forcing termination..."
                        kill -9 "$pid" 2>/dev/null || true
                        sleep 1
                    fi

                    log_success "PID $pid terminated"
                fi
            done

            echo ""
            log_success "Port $port cleaned"
        else
            log_success "Port $port is not occupied"
        fi
    else
        log_warning "Unsupported OS: $OSTYPE"
    fi

    echo ""
}

# ============================================================
# Main function
# ============================================================

main() {
    # Get script directory
    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    cd "$script_dir"

    echo ""
    echo "╔════════════════════════════════════════╗"
    echo "║  Port Cleanup Tool                    ║"
    echo "╚════════════════════════════════════════╝"
    echo ""
    log_info "Working directory: $script_dir"
    echo ""

    # Find config file
    local port=""
    local found_config=false

    local config_files=("config.jsonc" "config.json")

    for config_file in "${config_files[@]}"; do
        port=$(get_config_port "$config_file" || true)

        if [ -n "$port" ]; then
            log_success "Found config file: $config_file"
            log_info "Port: $port"
            found_config=true
            break
        fi
    done

    if [ "$found_config" = false ]; then
        log_warning "No config file found, using default port 9000"
        port=9000
    fi

    # Kill processes occupying port
    if [ -n "$port" ]; then
        kill_process_on_port "$port"
    fi

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    log_success "Cleanup complete"
    echo ""
}

# Run main function
main "$@"
