#!/bin/sh
# QSMxT.rs installer — downloads the latest release binary for your platform.
# Usage: curl -fsSL https://raw.githubusercontent.com/astewartau/qsmxt.rs/main/install.sh | sh

set -e

REPO="astewartau/qsmxt.rs"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        case "$ARCH" in
            x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
            aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
            *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
        esac
        ;;
    Darwin)
        case "$ARCH" in
            x86_64)  TARGET="x86_64-apple-darwin" ;;
            arm64)   TARGET="aarch64-apple-darwin" ;;
            *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
        esac
        ;;
    *)
        echo "Unsupported OS: $OS (use install.ps1 for Windows)"
        exit 1
        ;;
esac

# Get latest release tag
echo "Fetching latest release..."
TAG=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | cut -d'"' -f4)

if [ -z "$TAG" ]; then
    echo "Error: could not determine latest release"
    exit 1
fi

echo "Installing qsmxt ${TAG} for ${TARGET}..."

# Download and extract
URL="https://github.com/${REPO}/releases/download/${TAG}/qsmxt-${TAG}-${TARGET}.tar.gz"
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

curl -fsSL "$URL" -o "${TMPDIR}/qsmxt.tar.gz"
tar xzf "${TMPDIR}/qsmxt.tar.gz" -C "$TMPDIR"

# Install
mkdir -p "$INSTALL_DIR"
if [ -w "$INSTALL_DIR" ]; then
    mv "${TMPDIR}/qsmxt" "${INSTALL_DIR}/qsmxt"
else
    echo "Installing to ${INSTALL_DIR} (requires sudo)..."
    sudo mv "${TMPDIR}/qsmxt" "${INSTALL_DIR}/qsmxt"
fi

chmod +x "${INSTALL_DIR}/qsmxt"
echo "Installed qsmxt ${TAG} to ${INSTALL_DIR}/qsmxt"
echo ""
echo "Run 'qsmxt --version' to verify, or 'qsmxt tui' to get started."
