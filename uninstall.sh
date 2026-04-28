#!/bin/sh
# QSMxT.rs uninstaller for Linux/macOS
# Usage: curl -fsSL https://raw.githubusercontent.com/astewartau/qsmxt.rs/main/uninstall.sh | sh

set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARY="${INSTALL_DIR}/qsmxt"

if [ ! -f "$BINARY" ]; then
    echo "qsmxt not found at ${BINARY}"
    exit 0
fi

if [ -w "$INSTALL_DIR" ]; then
    rm "$BINARY"
elif command -v sudo >/dev/null 2>&1; then
    echo "Removing ${BINARY} (requires sudo)..."
    sudo rm "$BINARY"
else
    echo "Permission denied. Run manually: sudo rm ${BINARY}"
    exit 1
fi

echo "qsmxt has been removed from ${INSTALL_DIR}"
