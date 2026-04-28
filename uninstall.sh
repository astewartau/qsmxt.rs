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

if [ -w "$BINARY" ]; then
    rm "$BINARY"
else
    echo "Removing ${BINARY} (requires sudo)..."
    sudo rm "$BINARY"
fi

echo "qsmxt has been removed from ${INSTALL_DIR}"
