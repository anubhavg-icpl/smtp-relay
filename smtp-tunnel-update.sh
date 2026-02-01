#!/bin/bash
#
# SMTP Tunnel Proxy - Update Script (Rust Edition)
#
# Updates server files without touching config, certs, or users.
#
# Version: 2.0.0

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Configuration
INSTALL_DIR="/opt/smtp-tunnel"
CONFIG_DIR="/etc/smtp-tunnel"
BIN_DIR="/usr/local/bin"
GITHUB_RELEASES="https://github.com/yourusername/smtp-tunnel-rs/releases/latest/download"

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check root
if [ "$EUID" -ne 0 ]; then
    print_error "Please run as root (use sudo)"
    exit 1
fi

# Check installation exists
if [ ! -d "$INSTALL_DIR" ]; then
    print_error "SMTP Tunnel not installed at $INSTALL_DIR"
    print_error "Run the installer first"
    exit 1
fi

# Detect architecture
ARCH=$(uname -m)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')

case $ARCH in
    x86_64)
        BINARY_ARCH="x86_64"
        ;;
    aarch64|arm64)
        BINARY_ARCH="aarch64"
        ;;
    armv7l)
        BINARY_ARCH="armv7"
        ;;
    *)
        print_error "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  SMTP Tunnel Update (Rust Edition)${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

# Show current version
if [ -f "$INSTALL_DIR/smtp-tunnel-server" ]; then
    CURRENT_VERSION=$($INSTALL_DIR/smtp-tunnel-server --version 2>/dev/null | awk '{print $2}' || echo "unknown")
    print_info "Current version: $CURRENT_VERSION"
fi

# Binaries to update
BINARIES=(
    "smtp-tunnel-server"
    "smtp-tunnel-client"
    "smtp-tunnel-gen-certs"
    "smtp-tunnel-adduser"
    "smtp-tunnel-deluser"
    "smtp-tunnel-listusers"
)

# Download new binaries
print_info "Downloading updates..."
cd "$INSTALL_DIR"

FAILED=0
for binary in "${BINARIES[@]}"; do
    binary_name="${binary}-${OS}-${BINARY_ARCH}"
    
    if curl -sSL -f "$GITHUB_RELEASES/$binary_name" -o "$binary.new" 2>/dev/null; then
        mv "$binary.new" "$binary"
        chmod +x "$binary"
        echo "  ✓ Updated: $binary"
    else
        print_warn "  ✗ Failed: $binary"
        rm -f "$binary.new"
        FAILED=1
    fi
done

# Update symlinks
print_info "Updating symlinks..."
for binary in "${BINARIES[@]}"; do
    ln -sf "$INSTALL_DIR/$binary" "$BIN_DIR/$binary"
done

if [ $FAILED -eq 1 ]; then
    print_warn "Some binaries failed to update"
fi

# Show new version
if [ -f "$INSTALL_DIR/smtp-tunnel-server" ]; then
    NEW_VERSION=$($INSTALL_DIR/smtp-tunnel-server --version 2>/dev/null | awk '{print $2}' || echo "unknown")
    print_info "New version: $NEW_VERSION"
fi

# Restart service
print_info "Restarting service..."
if systemctl restart smtp-tunnel 2>/dev/null; then
    if systemctl is-active --quiet smtp-tunnel; then
        print_info "Service restarted successfully"
    else
        print_warn "Service may not have started. Check: systemctl status smtp-tunnel"
    fi
else
    print_warn "Failed to restart service. You may need to start it manually."
fi

echo ""
print_info "Update complete!"
echo ""
echo "Your config, certificates, and users were NOT modified."
echo ""
