#!/bin/sh
# TSP development script - unified entry point
# POSIX-compliant: works with sh, dash, bash, etc.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"

# Parse arguments
COMMAND="${1:-}"

# Get deno-tsp binary path
get_deno_bin() {
    target="${1:-debug}"
    os_type="$(get_os_type)"
    deno_bin="$PROJECT_ROOT/deno-tsp/$target/$os_type/deno-tsp"

    case "$(uname -s)" in
        CYGWIN*|MINGW*|MSYS*|Windows_NT)
        deno_bin="$deno_bin.exe"
        ;;
    esac

    echo "$deno_bin"
}

ensure_deno_bin() {
    deno_bin="$1"
    if [ ! -f "$deno_bin" ]; then
        echo "Error: $deno_bin does not exist"
        echo "Please run: sh ./tsp.sh build:denort:win && sh ./tsp.sh build:deno:win"
        exit 1
    fi
}

get_denort_bin() {
    target="${1:-debug}"
    os_type="$(get_os_type)"
    denort_bin="$PROJECT_ROOT/deno-tsp/$target/$os_type/denort-tsp"

    case "$(uname -s)" in
        CYGWIN*|MINGW*|MSYS*|Windows_NT)
        denort_bin="$denort_bin.exe"
        ;;
    esac

    echo "$denort_bin"
}

ensure_denort_bin() {
    denort_bin="$1"
    if [ ! -f "$denort_bin" ]; then
        echo "Error: $denort_bin does not exist"
        echo "Please run: sh ./tsp.sh build:denort:linux (or matching target)"
        exit 1
    fi
}

# Copy deno binaries to deno-tsp directory
copy_deno_bins() {
    target="${1:-debug}"

    # Get OS type for subdirectory
    os_type="$(get_os_type)"

    dest_dir="$PROJECT_ROOT/deno-tsp/$target/$os_type"
    src_dir="$PROJECT_ROOT/deno/target/$target"

    # Handle Windows executable extension
    denort_name="denort-tsp"
    deno_name="deno-tsp"
    case "$(uname -s)" in
        CYGWIN*|MINGW*|MSYS*|Windows_NT)
            denort_name="denort-tsp.exe"
            deno_name="deno-tsp.exe"
            ;;
    esac

    # Create destination directory
    mkdir -p "$dest_dir"

    # Copy binaries
    if [ -f "$src_dir/$denort_name" ]; then
        cp "$src_dir/$denort_name" "$dest_dir/"
        echo "Copied $denort_name to $dest_dir/"
    fi

    if [ -f "$src_dir/$deno_name" ]; then
        cp "$src_dir/$deno_name" "$dest_dir/"
        echo "Copied $deno_name to $dest_dir/"
    fi
}

# Build denort-tsp for Windows (default release)
build_denort_win() {
    do_clean="${1:-}"
    if [ "$do_clean" = "clean" ]; then
        cd "$PROJECT_ROOT/deno"
        cargo clean -p denort-tsp
    fi
    cd "$PROJECT_ROOT/deno"
    cargo build -p denort-tsp --release
    copy_deno_bins release
}

# Build denort-tsp for Windows (debug)
build_denort_win_dev() {
    do_clean="${1:-}"
    if [ "$do_clean" = "clean" ]; then
        cd "$PROJECT_ROOT/deno"
        cargo clean -p denort-tsp
    fi
    cd "$PROJECT_ROOT/deno"
    cargo build -p denort-tsp
    copy_deno_bins debug
}

# Get Linux arch for sysroot package naming
get_sysroot_arch() {
    case "$(uname -m)" in
        x86_64) echo "x86_64";;
        aarch64|arm64) echo "aarch64";;
        *)
            echo "Error: unsupported Linux architecture: $(uname -m)"
            exit 1
            ;;
    esac
}

# Download file with curl/wget fallback
download_file() {
    url="$1"
    output="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fL "$url" -o "$output"
        return $?
    fi

    if command -v wget >/dev/null 2>&1; then
        wget -O "$output" "$url"
        return $?
    fi

    echo "Error: neither curl nor wget is available"
    return 1
}

# Ensure sysroot exists locally (auto download + extract when missing)
ensure_sysroot_path() {
    sysroot_arch="$(get_sysroot_arch)"
    sysroot_version="${TSP_SYSROOT_VERSION:-20250207}"

    # Custom path override
    if [ -n "${SYSROOT_PATH:-}" ]; then
        if [ -f "$SYSROOT_PATH/.env" ]; then
            echo "$SYSROOT_PATH"
            return 0
        fi
        if [ -f "$SYSROOT_PATH/sysroot/.env" ]; then
            echo "$SYSROOT_PATH/sysroot"
            return 0
        fi
        echo "Error: SYSROOT_PATH is set but no .env found: $SYSROOT_PATH" >&2
        return 1
    fi

    sysroot_base="${TSP_SYSROOT_DIR:-$PROJECT_ROOT/.sysroot/$sysroot_arch}"
    sysroot_path="$sysroot_base/sysroot"
    archive_name="sysroot-${sysroot_arch}.tar.xz"
    archive_path="$sysroot_base/$archive_name"
    download_url="https://github.com/denoland/deno_sysroot_build/releases/download/sysroot-${sysroot_version}/${archive_name}"

    if [ -f "$sysroot_path/.env" ]; then
        echo "$sysroot_path"
        return 0
    fi

    mkdir -p "$sysroot_base"

    if [ ! -f "$archive_path" ]; then
        echo "Downloading sysroot: $download_url" >&2
        if ! download_file "$download_url" "$archive_path"; then
            echo "Error: failed to download sysroot archive" >&2
            return 1
        fi
    fi

    echo "Extracting sysroot to: $sysroot_base" >&2
    rm -rf "$sysroot_path"
    if ! tar -xJf "$archive_path" -C "$sysroot_base"; then
        echo "Error: failed to extract sysroot archive: $archive_path" >&2
        return 1
    fi

    if [ ! -f "$sysroot_path/.env" ]; then
        echo "Error: invalid sysroot package layout (missing $sysroot_path/.env)" >&2
        return 1
    fi

    echo "$sysroot_path"
}

# Apply sysroot .env to current shell (replace /sysroot with local path)
apply_sysroot_env() {
    sysroot_path="$1"
    env_file="$sysroot_path/.env"
    resolved_env="$sysroot_path/.env.resolved"
    prev_rustflags="${RUSTFLAGS:-}"
    prev_cflags="${CFLAGS:-}"

    if [ ! -f "$env_file" ]; then
        echo "Error: sysroot env file not found: $env_file"
        return 1
    fi

    # Replace /sysroot with actual path and strip trailing whitespace
    sed "s|/sysroot|$sysroot_path|g" "$env_file" | sed 's/[[:space:]]*$//' > "$resolved_env"

    # shellcheck disable=SC1090
    . "$resolved_env"

    rm -f "$resolved_env"

    # Merge previous flags if they exist and are non-empty
    if [ -n "$prev_rustflags" ] && [ -n "${RUSTFLAGS:-}" ]; then
        RUSTFLAGS="$RUSTFLAGS $prev_rustflags"
    elif [ -n "$prev_rustflags" ]; then
        RUSTFLAGS="$prev_rustflags"
    fi

    if [ -n "$prev_cflags" ] && [ -n "${CFLAGS:-}" ]; then
        CFLAGS="$CFLAGS $prev_cflags"
    elif [ -n "$prev_cflags" ]; then
        CFLAGS="$prev_cflags"
    fi

    # Only export if non-empty (avoid passing empty strings to cargo)
    if [ -n "${RUSTFLAGS:-}" ]; then
        export RUSTFLAGS
    else
        unset RUSTFLAGS
    fi

    if [ -n "${CFLAGS:-}" ]; then
        export CFLAGS
    else
        unset CFLAGS
    fi
}

# Run cargo build with auto sysroot
linux_cargo_build() {
    package_name="$1"
    shift

    # Get build type (release or debug)
    build_type=""
    if [ "$1" = "release" ] || [ "$1" = "debug" ]; then
        build_type="$1"
        shift
    fi

    # Check for clean parameter
    do_clean=""
    if [ "$1" = "clean" ]; then
        do_clean="clean"
        shift
    fi

    if [ "$(get_os_type)" != "linux" ]; then
        echo "Error: Linux sysroot build must run on Linux host"
        exit 1
    fi

    sysroot_path="$(ensure_sysroot_path)" || exit 1
    apply_sysroot_env "$sysroot_path" || exit 1
    echo "Using sysroot: $sysroot_path"

    cd "$PROJECT_ROOT/deno"
    if [ -n "$do_clean" ]; then
        cargo clean -p "$package_name"
    fi

    if [ "$build_type" = "release" ]; then
        cargo build -p "$package_name" --release
    else
        cargo build -p "$package_name"
    fi
}

# Build denort-tsp for Linux (auto sysroot)
build_denort_linux() {
    if [ "${1:-}" = "clean" ]; then
        linux_cargo_build denort-tsp release clean
    else
        linux_cargo_build denort-tsp release
    fi
    copy_deno_bins release
}

# Build denort-tsp for Linux (debug, auto sysroot)
build_denort_linux_dev() {
    if [ "${1:-}" = "clean" ]; then
        linux_cargo_build denort-tsp debug clean
    else
        linux_cargo_build denort-tsp debug
    fi
    copy_deno_bins debug
}

# Build denort-tsp for Linux (native, no sysroot)
build_denort_linux_native() {
    do_clean="${1:-}"
    cd "$PROJECT_ROOT/deno"
    if [ "$do_clean" = "clean" ]; then
        cargo clean -p denort-tsp
    fi
    cargo build -p denort-tsp --release
    copy_deno_bins release
}

# Build denort-tsp for Linux (native, debug, no sysroot)
build_denort_linux_native_dev() {
    do_clean="${1:-}"
    cd "$PROJECT_ROOT/deno"
    if [ "$do_clean" = "clean" ]; then
        cargo clean -p denort-tsp
    fi
    cargo build -p denort-tsp
    copy_deno_bins debug
}

# Build deno-tsp for Windows (default release)
build_deno_win() {
    do_clean="${1:-}"
    cd "$PROJECT_ROOT/deno"
    if [ "$do_clean" = "clean" ]; then
        cargo clean -p deno-tsp
    fi
    cargo build -p deno-tsp --release
    copy_deno_bins release
}

# Build deno-tsp for Windows (debug)
build_deno_win_dev() {
    do_clean="${1:-}"
    cd "$PROJECT_ROOT/deno"
    if [ "$do_clean" = "clean" ]; then
        cargo clean -p deno-tsp
    fi
    cargo build -p deno-tsp
    copy_deno_bins debug
}

# Build deno-tsp for Linux (auto sysroot)
build_deno_linux() {
    if [ "${1:-}" = "clean" ]; then
        linux_cargo_build deno-tsp release clean
    else
        linux_cargo_build deno-tsp release
    fi
    copy_deno_bins release
}

# Build deno-tsp for Linux (debug, auto sysroot)
build_deno_linux_dev() {
    if [ "${1:-}" = "clean" ]; then
        linux_cargo_build deno-tsp debug clean
    else
        linux_cargo_build deno-tsp debug
    fi
    copy_deno_bins debug
}

# Build deno-tsp for Linux (native, no sysroot)
build_deno_linux_native() {
    do_clean="${1:-}"
    cd "$PROJECT_ROOT/deno"
    if [ "$do_clean" = "clean" ]; then
        cargo clean -p deno-tsp
    fi
    cargo build -p deno-tsp --release
    copy_deno_bins release
}

# Build deno-tsp for Linux (native, debug, no sysroot)
build_deno_linux_native_dev() {
    do_clean="${1:-}"
    cd "$PROJECT_ROOT/deno"
    if [ "$do_clean" = "clean" ]; then
        cargo clean -p deno-tsp
    fi
    cargo build -p deno-tsp
    copy_deno_bins debug
}

# Run development server
run_dev() {
    deno_bin="$(get_deno_bin debug)"
    ensure_deno_bin "$deno_bin"

    "$deno_bin" run --watch --allow-all "$PROJECT_ROOT/src/main.ts" --dev
}

# Run production server
run_start() {
    deno_bin="$(get_deno_bin debug)"
    ensure_deno_bin "$deno_bin"

    "$deno_bin" run --allow-all "$PROJECT_ROOT/src/main.ts"
}

# Get OS type
get_os_type() {
    case "$(uname -s)" in
        Linux*)     echo "linux";;
        Darwin*)    echo "macos";;
        CYGWIN*|MINGW*|MSYS*) echo "win";;
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
    deno_json="$PROJECT_ROOT/deno.json"
    if [ -f "$deno_json" ]; then
        # Extract version using grep and sed
        grep -m1 '"version"' "$deno_json" | sed 's/.*"version": *"\([^"]*\)".*/\1/'
    else
        echo "0.0.0"
    fi
}

# Build TSP server
build_tspserver() {
    build_type="${1:-release}"
    target_os="${2:-}"  # Target OS: win, linux, or empty for current OS

    current_os="$(get_os_type)"
    arch="$(get_arch)"
    dist_base="$PROJECT_ROOT/dist"
    version="$(get_version)"

    # If target_os is not specified, use current OS
    if [ -z "$target_os" ]; then
        target_os="$current_os"
    fi

    # Determine output directory (target OS - suitable for GitHub releases)
    if [ "$build_type" = "release" ]; then
        output_dir="$dist_base/${target_os}-${arch}-v${version}"
    else
        output_dir="$dist_base/${target_os}-${arch}-v${version}-dev"
    fi

    # Determine deno-tsp path to use
    # Always use current OS deno-tsp for compilation
    if [ "$build_type" = "release" ]; then
        deno_bin="$(get_deno_bin release)"
    else
        deno_bin="$(get_deno_bin debug)"
    fi

    # Determine denort-bin: use target OS version
    if [ "$build_type" = "release" ]; then
        denort_bin="$PROJECT_ROOT/deno-tsp/release/$target_os/denort-tsp"
    else
        denort_bin="$PROJECT_ROOT/deno-tsp/debug/$target_os/denort-tsp"
    fi

    # Handle target OS denort-bin extension
    case "$target_os" in
        win)
            denort_bin="$denort_bin.exe"
            ;;
    esac

    ensure_deno_bin "$deno_bin"
    ensure_denort_bin "$denort_bin"

    echo "=== Building TSP server ($build_type for $target_os) ==="
    echo "Output directory: $output_dir"
    echo ""

    # Create output directory
    mkdir -p "$output_dir"

    # Compile tspserver
    tspserver_name="tspserver"

    # Determine target flag for cross-compilation
    target_flag=""
    case "$target_os" in
        win)
            tspserver_name="tspserver.exe"
            target_flag="--target x86_64-pc-windows-msvc"
            ;;
        linux)
            target_flag="--target x86_64-unknown-linux-gnu"
            ;;
        macos)
            target_flag="--target x86_64-apple-darwin"
            ;;
    esac

    echo "Compiling tspserver..."
    DENORT_BIN="$denort_bin" "$deno_bin" compile $target_flag --allow-all --dynamic-import-no-cache --output "$output_dir/$tspserver_name" "$PROJECT_ROOT/src/main.ts"

    # Copy config file
    echo "Copying config file..."

    # Copy config.example.jsonc (example config, not the actual config with sensitive data)
    if [ -f "$PROJECT_ROOT/config.example.jsonc" ]; then
        cp "$PROJECT_ROOT/config.example.jsonc" "$output_dir/"
    elif [ -f "$PROJECT_ROOT/config.example.json" ]; then
        cp "$PROJECT_ROOT/config.example.json" "$output_dir/"
    fi

    # Copy www directory
    cp -r "$PROJECT_ROOT/www" "$output_dir/"

    # Copy types.d.ts to www directory (overwrite if exists)
    if [ -f "$PROJECT_ROOT/types.d.ts" ]; then
        cp "$PROJECT_ROOT/types.d.ts" "$output_dir/www/"
    fi

    echo ""
    echo "✓ Build complete!"
    echo "  Output directory: $output_dir"
    echo "  Main binary: $output_dir/$tspserver_name"
}

# Run tests
run_test() {
    deno_bin
    test_file="${1:-run_all_tests.ts}"
    deno_bin="$(get_deno_bin debug)"
    ensure_deno_bin "$deno_bin"

    "$deno_bin" run --allow-all "$PROJECT_ROOT/tests/$test_file"
}

run_check() {
    deno_bin
    deno_bin="$(get_deno_bin debug)"
    ensure_deno_bin "$deno_bin"
    "$deno_bin" check "$PROJECT_ROOT/src/main.ts"
}

run_fmt() {
    deno_bin
    deno_bin="$(get_deno_bin debug)"
    ensure_deno_bin "$deno_bin"
    "$deno_bin" fmt --allow-write "$PROJECT_ROOT/src" "$PROJECT_ROOT/www" "$PROJECT_ROOT/tests"
}

run_lint() {
    deno_bin
    deno_bin="$(get_deno_bin debug)"
    ensure_deno_bin "$deno_bin"
    "$deno_bin" lint "$PROJECT_ROOT/src" "$PROJECT_ROOT/www" "$PROJECT_ROOT/tests"
}

run_e2e() {
    run_test run_e2e_tests.ts
}

# Package build output
package() {
    build_type="${1:-release}"  # release or debug
    os_type
    arch
    version
    dist_base="$PROJECT_ROOT/dist"

    os_type="$(get_os_type)"
    arch="$(get_arch)"
    version="$(get_version)"

    # Determine directory name
    if [ "$build_type" = "debug" ]; then
        dir_name="${os_type}-${arch}-v${version}-dev"
    else
        dir_name="${os_type}-${arch}-v${version}"
    fi

    source_dir="$dist_base/$dir_name"
    output_file="$dist_base/tsp-${dir_name}"

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
    if [ "$os_type" = "win" ]; then
        # Windows: try multiple methods
        if command -v powershell &> /dev/null; then
            # Use PowerShell's Compress-Archive (convert path to Windows style)
            cd "$dist_base"
            win_path=$(to_windows_path "$(pwd)")
            win_dir=$(to_windows_path "$dir_name")
            win_output=$(to_windows_path "${output_file}.zip")
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
    echo "  build:denort:win           Build denort-tsp for Windows"
    echo "  build:denort:win:dev      Build denort-tsp for Windows (debug)"
    echo "  build:denort:linux        Build denort-tsp for Linux (auto sysroot)"
    echo "  build:denort:linux:dev    Build denort-tsp for Linux (auto sysroot, debug)"
    echo "  build:denort:linux:native Build denort-tsp for Linux (native, no sysroot)"
    echo "  build:denort:linux:native:dev Build denort-tsp for Linux (native, debug)"
    echo "  build:deno:win            Build deno-tsp for Windows"
    echo "  build:deno:win:dev        Build deno-tsp for Windows (debug)"
    echo "  build:deno:linux          Build deno-tsp for Linux (auto sysroot)"
    echo "  build:deno:linux:dev     Build deno-tsp for Linux (auto sysroot, debug)"
    echo "  build:deno:linux:native   Build deno-tsp for Linux (native, no sysroot)"
    echo "  build:deno:linux:native:dev Build deno-tsp for Linux (native, debug)"
    echo "  build:tspserver           Build TSP server for current OS (release)"
    echo "  build:tspserver:dev      Build TSP server for current OS (debug)"
    echo "  build:tspserver:rel      Build TSP server for current OS (release)"
    echo "  build:tspserver:win      Build TSP server for Windows (release)"
    echo "  build:tspserver:win:dev  Build TSP server for Windows (debug)"
    echo "  build:tspserver:linux    Build TSP server for Linux (release)"
    echo "  build:tspserver:linux:dev Build TSP server for Linux (debug)"
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
    echo "Options:"
    echo "  clean                     Clean cargo cache before build"
    echo ""
    echo "Examples:"
    echo "  ./tsp.sh build:denort:win"
    echo "  ./tsp.sh build:denort:linux:native clean"
    echo "  ./tsp.sh build:deno:linux"
    echo "  ./tsp.sh build:tspserver:linux"
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
    build:denort:linux:native)
        build_denort_linux_native
        ;;
    build:denort:linux:native:dev)
        build_denort_linux_native_dev
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
    build:deno:linux:native)
        build_deno_linux_native
        ;;
    build:deno:linux:native:dev)
        build_deno_linux_native_dev
        ;;
    dev)
        run_dev
        ;;
    start)
        run_start
        ;;
    build:tspserver)
        # Default: build for current OS
        os_type="$(get_os_type)"
        build_tspserver release "$os_type"
        ;;
    build:tspserver:dev)
        os_type="$(get_os_type)"
        build_tspserver debug "$os_type"
        ;;
    build:tspserver:rel)
        os_type="$(get_os_type)"
        build_tspserver release "$os_type"
        ;;
    build:tspserver:win)
        build_tspserver release win
        ;;
    build:tspserver:win:dev)
        build_tspserver debug win
        ;;
    build:tspserver:linux)
        build_tspserver release linux
        ;;
    build:tspserver:linux:dev)
        build_tspserver debug linux
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
