#!/bin/bash
#
# SMTP Tunnel Proxy - Installation Script
#
# One-liner installation:
#   curl -sSL https://raw.githubusercontent.com/yourusername/smtp-tunnel-rs/main/install.sh | sudo bash
#
# Version: 2.0.0

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
INSTALL_DIR="/opt/smtp-tunnel"
CONFIG_DIR="/etc/smtp-tunnel"
BIN_DIR="/usr/local/bin"
GITHUB_RAW="https://raw.githubusercontent.com/yourusername/smtp-tunnel-rs/main"

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_banner() {
    echo ""
    echo -e "${CYAN}"
    echo "  ╔═══════════════════════════════════════════════════════════╗"
    echo "  ║                                                           ║"
    echo "  ║   ███████╗███╗   ███╗████████╗██████╗                     ║"
    echo "  ║   ██╔════╝████╗ ████║╚══██╔══╝██╔══██╗                    ║"
    echo "  ║   ███████╗██╔████╔██║   ██║   ██████╔╝                    ║"
    echo "  ║   ╚════██║██║╚██╔╝██║   ██║   ██╔═══╝                     ║"
    echo "  ║   ███████║██║ ╚═╝ ██║   ██║   ██║                         ║"
    echo "  ║   ╚══════╝╚═╝     ╚═╝   ╚═╝   ╚═╝                         ║"
    echo "  ║                                                           ║"
    echo "  ║   SMTP Tunnel Proxy - Rust Edition v2.0.0                 ║"
    echo "  ║                                                           ║"
    echo "  ╚═══════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    echo ""
}

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    print_error "Please run as root (use sudo)"
    exit 1
fi

print_banner

# Detect architecture
ARCH=$(uname -m)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')

print_info "Detected: $OS $ARCH"

# Map architecture to release binary name
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
        print_error "Please build from source: cargo build --release"
        exit 1
        ;;
esac

# Check for existing installation
if [ -d "$INSTALL_DIR" ]; then
    print_warn "SMTP Tunnel is already installed at $INSTALL_DIR"
    read -p "Reinstall? [y/N]: " response
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        print_info "Installation cancelled"
        exit 0
    fi
    # Backup existing config
    if [ -d "$CONFIG_DIR" ]; then
        BACKUP_DIR="/tmp/smtp-tunnel-backup-$(date +%Y%m%d-%H%M%S)"
        print_info "Backing up configuration to $BACKUP_DIR"
        cp -r "$CONFIG_DIR" "$BACKUP_DIR"
    fi
fi

# Create directories
print_info "Creating directories..."
mkdir -p "$INSTALL_DIR"
mkdir -p "$CONFIG_DIR"

# Download binaries
print_info "Downloading binaries..."

BINARIES=(
    "smtp-tunnel-server"
    "smtp-tunnel-client"
    "smtp-tunnel-gen-certs"
    "smtp-tunnel-adduser"
    "smtp-tunnel-deluser"
    "smtp-tunnel-listusers"
)

# Try to download from GitHub releases
RELEASE_URL="https://github.com/yourusername/smtp-tunnel-rs/releases/latest/download"
DOWNLOAD_SUCCESS=true

for binary in "${BINARIES[@]}"; do
    binary_name="${binary}-${OS}-${BINARY_ARCH}"
    
    print_info "Downloading $binary..."
    
    if curl -sSL -f "$RELEASE_URL/$binary_name" -o "$INSTALL_DIR/$binary" 2>/dev/null; then
        chmod +x "$INSTALL_DIR/$binary"
        print_info "  ✓ $binary downloaded"
    else
        print_warn "  ✗ Failed to download $binary"
        DOWNLOAD_SUCCESS=false
    fi
done

if [ "$DOWNLOAD_SUCCESS" = false ]; then
    print_error "Some binaries failed to download"
    print_info "You can build from source:"
    print_info "  git clone https://github.com/yourusername/smtp-tunnel-rs"
    print_info "  cd smtp-tunnel-rs && cargo build --release"
    print_info "  sudo cp target/release/smtp-tunnel-* /opt/smtp-tunnel/"
    exit 1
fi

# Create symlinks
print_info "Creating command symlinks..."
for binary in "${BINARIES[@]}"; do
    ln -sf "$INSTALL_DIR/$binary" "$BIN_DIR/$binary"
done

# Generate certificates if not exist
if [ ! -f "$CONFIG_DIR/server.crt" ]; then
    print_info "Generating TLS certificates..."
    
    # Ask for hostname
    read -p "Enter your server's domain name [mail.example.com]: " hostname
    hostname=${hostname:-mail.example.com}
    
    cd "$CONFIG_DIR"
    "$INSTALL_DIR/smtp-tunnel-gen-certs" --hostname "$hostname" --output "$CONFIG_DIR"
    
    print_info "Certificates generated in $CONFIG_DIR"
else
    print_info "TLS certificates already exist"
fi

# Create default config if not exist
if [ ! -f "$CONFIG_DIR/config.yaml" ]; then
    print_info "Creating default configuration..."
    
    # Get hostname from certificate or ask
    if [ -f "$CONFIG_DIR/server.crt" ]; then
        HOSTNAME=$(openssl x509 -in "$CONFIG_DIR/server.crt" -noout -subject 2>/dev/null | sed -n 's/.*CN = \([^,]*\).*/\1/p' || echo "mail.example.com")
    else
        HOSTNAME="mail.example.com"
    fi
    
    cat > "$CONFIG_DIR/config.yaml" << EOF
# SMTP Tunnel Configuration

server:
  host: "0.0.0.0"
  port: 587
  hostname: "$HOSTNAME"
  cert_file: "$CONFIG_DIR/server.crt"
  key_file: "$CONFIG_DIR/server.key"
  users_file: "$CONFIG_DIR/users.yaml"
  log_users: true

client:
  server_host: "$HOSTNAME"
  server_port: 587
  socks_port: 1080
  socks_host: "127.0.0.1"
  ca_cert: "$CONFIG_DIR/ca.crt"
EOF
    
    chmod 600 "$CONFIG_DIR/config.yaml"
    print_info "Configuration created: $CONFIG_DIR/config.yaml"
fi

# Create default users file if not exist
if [ ! -f "$CONFIG_DIR/users.yaml" ]; then
    print_info "Creating users file..."
    cat > "$CONFIG_DIR/users.yaml" << 'EOF'
# SMTP Tunnel Users
# Use smtp-tunnel-adduser to add users

users:
  # Example user:
  # alice:
  #   secret: "auto-generated-secret"
  #   logging: true
  #   whitelist:
  #     - "192.168.1.100"
EOF
    chmod 600 "$CONFIG_DIR/users.yaml"
    print_info "Users file created: $CONFIG_DIR/users.yaml"
fi

# Install systemd service
if command -v systemctl &> /dev/null; then
    print_info "Installing systemd service..."
    
    cat > /etc/systemd/system/smtp-tunnel.service << EOF
[Unit]
Description=SMTP Tunnel Server (Rust)
Documentation=https://github.com/yourusername/smtp-tunnel-rs
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=$CONFIG_DIR
ExecStart=$INSTALL_DIR/smtp-tunnel-server -c $CONFIG_DIR/config.yaml
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF
    
    systemctl daemon-reload
    print_info "Systemd service installed"
fi

# Setup firewall (ufw)
if command -v ufw &> /dev/null; then
    print_info "Configuring firewall (UFW)..."
    ufw allow 587/tcp comment 'SMTP Tunnel' 2>/dev/null || true
    print_info "Firewall rule added for port 587"
fi

# Create first user
print_info ""
read -p "Would you like to create a user now? [Y/n]: " create_user
create_user=${create_user:-Y}

if [[ "$create_user" =~ ^[Yy]$ ]]; then
    read -p "Enter username: " username
    if [ -n "$username" ]; then
        "$BIN_DIR/smtp-tunnel-adduser" "$username" --users-file "$CONFIG_DIR/users.yaml" --config "$CONFIG_DIR/config.yaml"
    fi
fi

# Final instructions
print_banner
print_info "Installation complete!"
echo ""
echo -e "${BLUE}Configuration files:${NC}"
echo "  Config:  $CONFIG_DIR/config.yaml"
echo "  Users:   $CONFIG_DIR/users.yaml"
echo "  Certs:   $CONFIG_DIR/"
echo ""
echo -e "${BLUE}Management commands:${NC}"
echo "  smtp-tunnel-adduser <name>     - Add a new user"
echo "  smtp-tunnel-deluser <name>     - Remove a user"
echo "  smtp-tunnel-listusers          - List all users"
echo ""
echo -e "${BLUE}Service management:${NC}"
echo "  systemctl start smtp-tunnel    - Start server"
echo "  systemctl stop smtp-tunnel     - Stop server"
echo "  systemctl status smtp-tunnel   - Check status"
echo "  systemctl enable smtp-tunnel   - Enable auto-start"
echo ""
echo -e "${BLUE}To start the server now:${NC}"
echo "  systemctl start smtp-tunnel"
echo ""
echo -e "${GREEN}Thank you for using SMTP Tunnel!${NC}"
echo ""
