#!/usr/bin/env bash
# Wrapper script to build and run im-deploy

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_PATH="$SCRIPT_DIR/im-deploy/target/release/im-deploy"
CARGO_TOML="$SCRIPT_DIR/im-deploy/Cargo.toml"

# Check if the binary exists and is up to date
if [ -f "$BINARY_PATH" ]; then
    # Check if any source files are newer than the binary
    if [ -n "$(find "$SCRIPT_DIR/im-deploy/src" -newer "$BINARY_PATH" 2>/dev/null)" ] || \
       [ "$CARGO_TOML" -nt "$BINARY_PATH" ]; then
        echo "Source files changed, rebuilding..."
        cd "$SCRIPT_DIR/im-deploy"
        cargo build --release
    fi
else
    echo "Binary not found, building release version..."
    cd "$SCRIPT_DIR/im-deploy"
    cargo build --release
fi

# Run the binary with all passed arguments
exec "$BINARY_PATH" "$@"

