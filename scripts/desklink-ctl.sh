#!/usr/bin/env bash
# DeskLink Control Helper Script

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN_PATH="$PROJECT_ROOT/desklink-daemon/target/release/desklink-daemon"

usage() {
    echo "Usage: $0 {start|build|test|simulate|help} [options]"
    echo ""
    echo "Commands:"
    echo "  start     - Run DeskLink Linux Host Daemon"
    echo "  build     - Build DeskLink Daemon in Release mode"
    echo "  test      - Run Rust unit tests for protocol and uinput"
    echo "  simulate  - Run end-to-end Python client simulator"
    echo "  help      - Show this help message"
    exit 1
}

case "$1" in
    start)
        shift
        if [ ! -f "$BIN_PATH" ]; then
            echo "[*] Building daemon binary..."
            cargo build --release --manifest-path "$PROJECT_ROOT/desklink-daemon/Cargo.toml"
        fi
        echo "[*] Launching DeskLink Host Daemon..."
        exec "$BIN_PATH" "$@"
        ;;
    build)
        echo "[*] Building DeskLink Host Daemon in Release mode..."
        cargo build --release --manifest-path "$PROJECT_ROOT/desklink-daemon/Cargo.toml"
        ;;
    test)
        echo "[*] Running Protocol and Input Tests..."
        cargo test --manifest-path "$PROJECT_ROOT/desklink-daemon/Cargo.toml"
        ;;
    simulate)
        echo "[*] Running Client Simulator..."
        python3 "$PROJECT_ROOT/scripts/test_client_sim.py"
        ;;
    *)
        usage
        ;;
esac
