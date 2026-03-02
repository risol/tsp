#!/bin/bash
# TSP development script - unified entry point

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"

# Parse arguments
COMMAND="${1:-}"

# Get deno-tsp binary path
get_deno_bin() {
    local target="${1:-debug}"
    local deno_bin="$PROJECT_ROOT/deno/target/$target/deno-tsp"

    case "$(uname -s)" in
        CYGWIN*|MINGW*|MSYS*|Windows_NT)
        deno_bin="$deno_bin.exe"
        ;;
    esac

    echo "$deno_bin"
}

ensure_deno_bin() {
    local deno_bin="$1"
    if [ ! -f "$deno_bin" ]; then
        echo "Error: $deno_bin does not exist"
        echo "Please run: sh ./tsp.sh build:denort:win && sh ./tsp.sh build:deno:win"
        exit 1
    fi
}

# Build denort-tsp for Windows (default release)
build_denort_win() {
    cd "$PROJECT_ROOT/deno"
    cargo build -p denort-tsp --release "${@}"
}

# Build denort-tsp for Windows (debug)
build_denort_win_dev() {
    cd "$PROJECT_ROOT/deno"
    cargo build -p denort-tsp "${@}"
}

# Build denort-tsp for Linux (static linking with sysroot)
build_denort_linux() {
    local sysroot_path="${SYSROOT_PATH:-$HOME/deno-sysroot/x86_64-unknown-linux-gnu}"

    if [ ! -d "$sysroot_path" ]; then
        echo "Error: Sysroot not found at $sysroot_path"
        echo "Please download and extract sysroot first:"
        echo "  wget https://github.com/denoland/deno_sysroot_build/releases/download/sysroot-20250207/sysroot-x86_64.tar.xz"
        echo "  tar -xf sysroot-x86_64.tar.xz -C ~"
        exit 1
    fi

    cd "$PROJECT_ROOT/deno"
    RUSTFLAGS="--sysroot=$sysroot_path -C target-feature=-crt-static" cargo build -p denort-tsp --release "${@}"
}

# Build denort-tsp for Linux (debug, static linking with sysroot)
build_denort_linux_dev() {
    local sysroot_path="${SYSROOT_PATH:-$HOME/deno-sysroot/x86_64-unknown-linux-gnu}"

    if [ ! -d "$sysroot_path" ]; then
        echo "Error: Sysroot not found at $sysroot_path"
        echo "Please download and extract sysroot first:"
        echo "  wget https://github.com/denoland/deno_sysroot_build/releases/download/sysroot-20250207/sysroot-x86_64.tar.xz"
        echo "  tar -xf sysroot-x86_64.tar.xz -C ~"
        exit 1
    fi

    cd "$PROJECT_ROOT/deno"
    RUSTFLAGS="--sysroot=$sysroot_path -C target-feature=-crt-static" cargo build -p denort-tsp "${@}"
}

# Build deno-tsp for Windows (default release)
build_deno_win() {
    cd "$PROJECT_ROOT/deno"
    cargo build -p deno-tsp --release "${@}"
}

# Build deno-tsp for Windows (debug)
build_deno_win_dev() {
    cd "$PROJECT_ROOT/deno"
    cargo build -p deno-tsp "${@}"
}

# Build deno-tsp for Linux (static linking with sysroot)
build_deno_linux() {
    local sysroot_path="${SYSROOT_PATH:-$HOME/deno-sysroot/x86_64-unknown-linux-gnu}"

    if [ ! -d "$sysroot_path" ]; then
        echo "Error: Sysroot not found at $sysroot_path"
        echo "Please download and extract sysroot first:"
        echo "  wget https://github.com/denoland/deno_sysroot_build/releases/download/sysroot-20250207/sysroot-x86_64.tar.xz"
        echo "  tar -xf sysroot-x86_64.tar.xz -C ~"
        exit 1
    fi

    cd "$PROJECT_ROOT/deno"
    RUSTFLAGS="--sysroot=$sysroot_path -C target-feature=-crt-static" cargo build -p deno-tsp --release "${@}"
}

# Build deno-tsp for Linux (debug, static linking with sysroot)
build_deno_linux_dev() {
    local sysroot_path="${SYSROOT_PATH:-$HOME/deno-sysroot/x86_64-unknown-linux-gnu}"

    if [ ! -d "$sysroot_path" ]; then
        echo "Error: Sysroot not found at $sysroot_path"
        echo "Please download and extract sysroot first:"
        echo "  wget https://github.com/denoland/deno_sysroot_build/releases/download/sysroot-20250207/sysroot-x86_64.tar.xz"
        echo "  tar -xf sysroot-x86_64.tar.xz -C ~"
        exit 1
    fi

    cd "$PROJECT_ROOT/deno"
    RUSTFLAGS="--sysroot=$sysroot_path -C target-feature=-crt-static" cargo build -p deno-tsp "${@}"
}

# Run development server
run_dev() {
    local deno_bin
    deno_bin="$(get_deno_bin debug)"
    ensure_deno_bin "$deno_bin"

    "$deno_bin" run --watch --dynamic-import-no-cache --allow-all "$PROJECT_ROOT/src/main.ts" --dev
}

# Run production server
run_start() {
    local deno_bin
    deno_bin="$(get_deno_bin debug)"
    ensure_deno_bin "$deno_bin"

    "$deno_bin" run --allow-all "$PROJECT_ROOT/src/main.ts"
}

# Get OS type
get_os_type() {
    case "$(uname -s)" in
        Linux*)     echo "linux";;
        Darwin*)    echo "macos";;
        CYGWIN*|MINGW*|MSYS*) echo "windows";;
        *)          echo "unknown";;
    esac
}

# Get CPU architecture
get_arch() {
    case "$(uname -m)" in
        x86_64)     echo "x64";;
        aarch64|arm64) echo "arm64";;
        *)          echo "x64";;
    esac
}

# Get version from deno.json
get_version() {
    local deno_json="$PROJECT_ROOT/deno.json"
    if [ -f "$deno_json" ]; then
        # Extract version using grep and sed
        grep -m1 '"version"' "$deno_json" | sed 's/.*"version": *"\([^"]*\)".*/\1/'
    else
        echo "0.0.0"
    fi
}

# Build TSP server
build_tspserver() {
    local build_type="${1:-debug}"
    local os_type
    local arch
    local output_dir
    local tspserver_bin
    local dist_base="$PROJECT_ROOT/dist"
    local version
    version="$(get_version)"

    os_type="$(get_os_type)"
    arch="$(get_arch)"

    # Determine output directory (combined os-arch-version - suitable for GitHub releases)
    if [ "$build_type" = "release" ]; then
        output_dir="$dist_base/${os_type}-${arch}-v${version}"
    else
        output_dir="$dist_base/${os_type}-${arch}-v${version}-dev"
    fi

    # Determine deno-tsp path to use
    local deno_bin
    if [ "$build_type" = "release" ]; then
        deno_bin="$(get_deno_bin release)"
    else
        deno_bin="$(get_deno_bin debug)"
    fi

    ensure_deno_bin "$deno_bin"

    echo "=== Building TSP server ($build_type) ==="
    echo "Output directory: $output_dir"
    echo ""

    # Create output directory
    mkdir -p "$output_dir"

    # Compile tspserver
    local tspserver_name="tspserver"
    if [ "$os_type" = "windows" ]; then
        tspserver_name="tspserver.exe"
    fi

    echo "Compiling tspserver..."
    "$deno_bin" compile --allow-all --dynamic-import-no-cache --output "$output_dir/$tspserver_name" "$PROJECT_ROOT/src/main.ts"

    # Copy config file
    echo "Copying config file..."

    # Copy config.jsonc (if exists)
    if [ -f "$PROJECT_ROOT/config.jsonc" ]; then
        cp "$PROJECT_ROOT/config.jsonc" "$output_dir/"
    elif [ -f "$PROJECT_ROOT/config.json" ]; then
        cp "$PROJECT_ROOT/config.json" "$output_dir/"
    fi

    # Copy types.d.ts
    if [ -f "$PROJECT_ROOT/types.d.ts" ]; then
        cp "$PROJECT_ROOT/types.d.ts" "$output_dir/"
    fi

    # Create www directory and copy CLAUDE_GUIDE_README.md
    mkdir -p "$output_dir/www"
    if [ -f "$PROJECT_ROOT/www/CLAUDE_GUIDE_README.md" ]; then
        cp "$PROJECT_ROOT/www/CLAUDE_GUIDE_README.md" "$output_dir/www/"
    fi

    echo ""
    echo "✓ Build complete!"
    echo "  Output directory: $output_dir"
    echo "  Main binary: $output_dir/$tspserver_name"
}

# Run tests
run_test() {
    local deno_bin
    local test_file="${1:-run_all_tests.ts}"
    deno_bin="$(get_deno_bin debug)"
    ensure_deno_bin "$deno_bin"

    "$deno_bin" run --allow-all "$PROJECT_ROOT/tests/$test_file"
}

run_check() {
    local deno_bin
    deno_bin="$(get_deno_bin debug)"
    ensure_deno_bin "$deno_bin"
    "$deno_bin" check "$PROJECT_ROOT/src/main.ts"
}

run_fmt() {
    local deno_bin
    deno_bin="$(get_deno_bin debug)"
    ensure_deno_bin "$deno_bin"
    "$deno_bin" fmt --allow-write "$PROJECT_ROOT/src" "$PROJECT_ROOT/www" "$PROJECT_ROOT/tests"
}

run_lint() {
    local deno_bin
    deno_bin="$(get_deno_bin debug)"
    ensure_deno_bin "$deno_bin"
    "$deno_bin" lint "$PROJECT_ROOT/src" "$PROJECT_ROOT/www" "$PROJECT_ROOT/tests"
}

run_e2e() {
    run_test run_e2e_tests.ts
}

# Package build output
package() {
    local build_type="${1:-release}"  # release or debug
    local os_type
    local arch
    local version
    local dist_base="$PROJECT_ROOT/dist"

    os_type="$(get_os_type)"
    arch="$(get_arch)"
    version="$(get_version)"

    # Determine directory name
    if [ "$build_type" = "debug" ]; then
        local dir_name="${os_type}-${arch}-v${version}-dev"
    else
        local dir_name="${os_type}-${arch}-v${version}"
    fi

    local source_dir="$dist_base/$dir_name"
    local output_file="$dist_base/tsp-${dir_name}"

    # Check if directory exists
    if [ ! -d "$source_dir" ]; then
        echo "Error: Directory not found: $source_dir"
        echo "Please run './tsp.sh build:tspserver' first"
        exit 1
    fi

    # Convert path for PowerShell (Unix to Windows)
    to_windows_path() {
        echo "$1" | sed 's|^/d/|D:/|;s|/|\\\\|g'
    }

    echo "=== Packaging TSP server ($build_type) ==="
    echo "Source: $source_dir"
    echo "Output: $output_file"
    echo ""

    # Create archive based on OS
    if [ "$os_type" = "windows" ]; then
        # Windows: try multiple methods
        if command -v powershell &> /dev/null; then
            # Use PowerShell's Compress-Archive (convert path to Windows style)
            cd "$dist_base"
            local win_path=$(to_windows_path "$(pwd)")
            local win_dir=$(to_windows_path "$dir_name")
            local win_output=$(to_windows_path "${output_file}.zip")
            powershell -Command "Compress-Archive -Path '$win_dir' -DestinationPath '$win_output' -Force"
            echo "✓ Package created: ${output_file}.zip"
        elif command -v 7z &> /dev/null; then
            # Use 7z if available
            cd "$dist_base"
            7z a -r "${output_file}.zip" "$dir_name"
            echo "✓ Package created: ${output_file}.zip"
        elif command -v zip &> /dev/null; then
            # Use zip if available (Git Bash with MSYS)
            cd "$dist_base"
            zip -r "${output_file}.zip" "$dir_name"
            echo "✓ Package created: ${output_file}.zip"
        else
            echo "Error: No compression tool found. Install 7zip or use PowerShell."
            exit 1
        fi
    else
        # Linux/macOS: tar.gz
        cd "$dist_base"
        tar -czf "${output_file}.tar.gz" "$dir_name"
        echo "✓ Package created: ${output_file}.tar.gz"
    fi
}

# Show help
show_help() {
    echo "TSP development script"
    echo ""
    echo "Usage: ./tsp.sh <command> [args]"
    echo ""
    echo "Commands:"
    echo "  build:denort:win      Build denort-tsp for Windows"
    echo "  build:denort:win:dev Build denort-tsp for Windows (debug)"
    echo "  build:denort:linux   Build denort-tsp for Linux (static)"
    echo "  build:denort:linux:dev Build denort-tsp for Linux (static, debug)"
    echo "  build:deno:win       Build deno-tsp for Windows"
    echo "  build:deno:win:dev   Build deno-tsp for Windows (debug)"
    echo "  build:deno:linux     Build deno-tsp for Linux (static)"
    echo "  build:deno:linux:dev Build deno-tsp for Linux (static, debug)"
    echo "  build:tspserver            Build TSP server (debug) -> dist/<os>-<arch>-v<version>-dev/"
    echo "  build:tspserver:rel       Build TSP server (release) -> dist/<os>-<arch>-v<version>/"
    echo "  package                   Package build output to zip/tar.gz"
    echo "  dev                       Run development server (hot reload)"
    echo "  start                     Run production server"
    echo "  test                      Run all tests"
    echo "  test:unit                 Run unit tests"
    echo "  test:e2e                  Run E2E tests"
    echo "  check                     Type check"
    echo "  fmt                       Format code"
    echo "  lint                      Lint code"
    echo ""
    echo "Examples:"
    echo "  ./tsp.sh build:denort:win"
    echo "  ./tsp.sh build:deno:linux"
    echo "  ./tsp.sh build:tspserver"
    echo "  ./tsp.sh dev"
}

# Main logic
case "$COMMAND" in
    build:denort:win)
        build_denort_win
        ;;
    build:denort:win:dev)
        build_denort_win_dev
        ;;
    build:denort:linux)
        build_denort_linux
        ;;
    build:denort:linux:dev)
        build_denort_linux_dev
        ;;
    build:deno:win)
        build_deno_win
        ;;
    build:deno:win:dev)
        build_deno_win_dev
        ;;
    build:deno:linux)
        build_deno_linux
        ;;
    build:deno:linux:dev)
        build_deno_linux_dev
        ;;
    dev)
        run_dev
        ;;
    start)
        run_start
        ;;
    build:tspserver)
        build_tspserver debug
        ;;
    build:tspserver:rel)
        build_tspserver release
        ;;
    test)
        run_test
        ;;
    test:unit)
        run_test run_unit_tests.ts
        ;;
    test:e2e)
        run_e2e
        ;;
    package)
        package "${2:-release}"
        ;;
    check)
        run_check
        ;;
    fmt)
        run_fmt
        ;;
    lint)
        run_lint
        ;;
    *)
        show_help
        ;;
esac
