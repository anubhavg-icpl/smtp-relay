# 🦀 SMTP Tunnel Proxy - Rust Implementation

> A high-speed covert tunnel that disguises TCP traffic as SMTP email communication to bypass Deep Packet Inspection (DPI) firewalls.

**Rust rewrite** of the original Python SMTP Tunnel Proxy with improved performance, memory safety, and smaller binaries.

[![Author](https://img.shields.io/badge/Author-anubhavg--icpl-blue)](https://github.com/anubhavg-icpl)
[![Email](https://img.shields.io/badge/Email-anubhavg%40infopercept.com-red)](mailto:anubhavg@infopercept.com)
[![Version](https://img.shields.io/badge/Version-2.0.0-green)]()
[![License](https://img.shields.io/badge/License-MIT-yellow)]()

---

## 🎯 Features

| Feature | Description |
|---------|-------------|
| 🔒 **TLS 1.3** | Modern encryption with rustls |
| 🎭 **DPI Evasion** | Mimics real Postfix SMTP servers |
| ⚡ **Zero-Cost Async** | Handle thousands of connections with tokio |
| 👥 **Multi-User** | Per-user secrets and IP whitelists |
| 🔑 **HMAC Auth** | Time-based authentication tokens |
| 🌐 **SOCKS5 Proxy** | Standard proxy interface |
| 📡 **Multiplexing** | Multiple connections over single tunnel |
| 🦀 **Memory Safe** | No buffer overflows, no segfaults |
| 📦 **Static Binary** | Single executable, no runtime needed |

---

## 📊 Performance vs Python

| Metric | Python | Rust | Improvement |
|--------|--------|------|-------------|
| **Binary Size** | ~50MB + deps | ~1-2MB | **10x smaller** |
| **Memory Usage** | ~50MB base | ~5MB base | **10x less** |
| **Speed** | ~100 Mbps | ~1 Gbps | **10x faster** |
| **Latency** | GC pauses | Predictable | **No pauses** |
| **Safety** | Runtime errors | Compile-time | **Memory safe** |

---

## 🚀 Quick Start

### Installation

```bash
# One-liner installation
curl -sSL https://raw.githubusercontent.com/yourusername/smtp-tunnel-rs/main/install.sh | sudo bash

# Or download pre-built binaries from releases
wget https://github.com/yourusername/smtp-tunnel-rs/releases/latest/download/smtp-tunnel-server-linux-x86_64
wget https://github.com/yourusername/smtp-tunnel-rs/releases/latest/download/smtp-tunnel-client-linux-x86_64
chmod +x smtp-tunnel-*
```

### Server Setup (VPS)

```bash
# 1. Generate TLS certificates
smtp-tunnel-gen-certs --hostname mail.example.com

# 2. Add a user
smtp-tunnel-adduser alice

# 3. Start server
smtp-tunnel-server -c config.yaml

# Or use systemd
systemctl start smtp-tunnel
```

### Client Setup

```bash
# Get the client package from your server admin (alice.zip)
unzip alice.zip
cd alice

# Run client
./smtp-tunnel-client -c config.yaml

# Or use the launcher
./start.sh  # Linux/Mac
start.bat   # Windows

# Test
curl -x socks5h://127.0.0.1:1080 https://ifconfig.me
```

---

## 📦 Binaries

| Binary | Size | Purpose |
|--------|------|---------|
| `smtp-tunnel-server` | ~1.6M | Tunnel server (runs on VPS) |
| `smtp-tunnel-client` | ~1M | SOCKS5 proxy client |
| `smtp-tunnel-gen-certs` | ~900K | TLS certificate generator |
| `smtp-tunnel-adduser` | ~900K | Add users & create packages |
| `smtp-tunnel-deluser` | ~700K | Remove users |
| `smtp-tunnel-listusers` | ~700K | List all users |

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         CLIENT COMPUTER                             │
│  ┌──────────────┐      ┌──────────────┐      ┌─────────────────┐   │
│  │   Browser    │─────▶│  SOCKS5      │─────▶│   SMTP Client   │   │
│  │   curl, etc  │      │  127.0.0.1   │      │   (Rust/tokio)  │   │
│  └──────────────┘      │   :1080      │      └────────┬────────┘   │
│                        └──────────────┘               │            │
│                                                       │ TLS Tunnel │
│                                                       ▼            │
│                                               ┌──────────────┐     │
│                                               │  smtp-tunnel │─────┼────▶
│                                               │  -client     │     │
│                                               └──────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
                                     │
                                     │ Port 587 (looks like SMTP)
                                     ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         SERVER (VPS)                                │
│                                               ┌──────────────┐      │
│  ◀────────────────────────────────────────────│  SMTP Server │      │
│                                               │  (Rust/tokio)│      │
│                                               └──────┬───────┘      │
│                                                      │               │
│                                                      │ Forward       │
│                                                      ▼               │
│                                               ┌──────────────┐      │
│                                               │   Internet   │      │
│                                               │   (Any TCP)  │      │
│                                               └──────────────┘      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 🔧 Command Reference

### smtp-tunnel-server

```
SMTP Tunnel Server

Usage: smtp-tunnel-server [OPTIONS]

Options:
  -c, --config <FILE>     Configuration file [default: config.yaml]
  -u, --users <FILE>      Users file
  -d, --debug             Enable debug logging
  -h, --help              Print help
  -V, --version           Print version
```

### smtp-tunnel-client

```
SMTP Tunnel Client

Usage: smtp-tunnel-client [OPTIONS]

Options:
  -c, --config <FILE>     Configuration file [default: config.yaml]
      --server <HOST>     Server hostname
      --server-port <PORT> Server port
  -p, --socks-port <PORT> Local SOCKS port
  -u, --username <NAME>   Username
  -s, --secret <SECRET>   Secret
      --ca-cert <FILE>    CA certificate file
  -d, --debug             Enable debug logging
  -h, --help              Print help
  -V, --version           Print version
```

### smtp-tunnel-adduser

```
Add a new user and generate client package

Usage: smtp-tunnel-adduser [OPTIONS] <USERNAME>

Arguments:
  <USERNAME>  Username to add

Options:
  -s, --secret <SECRET>          Secret (auto-generated if not provided)
  -w, --whitelist <WHITELIST>    IP whitelist (can specify multiple)
      --no-logging               Disable logging for this user
  -u, --users-file <USERS_FILE>  Users file [default: /etc/smtp-tunnel/users.yaml]
  -c, --config <CONFIG>          Server config file [default: /etc/smtp-tunnel/config.yaml]
  -o, --output-dir <OUTPUT_DIR>  Output directory for ZIP file [default: .]
      --no-package               Do not generate client ZIP package
  -h, --help                     Print help
  -V, --version                  Print version
```

---

## 📁 Configuration

### Server Config (`config.yaml`)

```yaml
server:
  host: "0.0.0.0"
  port: 587
  hostname: "mail.example.com"
  cert_file: "/etc/smtp-tunnel/server.crt"
  key_file: "/etc/smtp-tunnel/server.key"
  users_file: "/etc/smtp-tunnel/users.yaml"
  log_users: true

client:
  server_host: "mail.example.com"
  server_port: 587
  socks_port: 1080
  socks_host: "127.0.0.1"
  ca_cert: "/etc/smtp-tunnel/ca.crt"
```

### Users File (`users.yaml`)

```yaml
users:
  alice:
    secret: "auto-generated-secret"
    logging: true
    whitelist:
      - "192.168.1.100"
      - "10.0.0.0/8"
  
  bob:
    secret: "another-secret"
    logging: false
    whitelist: []  # Allow from any IP
```

---

## 🛠️ Building from Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone repository
git clone https://github.com/yourusername/smtp-tunnel-rs
cd smtp-tunnel-rs

# Build release binaries
cargo build --release

# Binaries will be in target/release/
ls -la target/release/smtp-tunnel-*
```

### Cross-compilation

```bash
# Install cross
cargo install cross

# Build for different targets
cross build --release --target x86_64-unknown-linux-musl
cross build --release --target aarch64-unknown-linux-musl
cross build --release --target x86_64-pc-windows-gnu
```

---

## 🔐 Security Features

- **TLS 1.3** encryption for all traffic
- **HMAC-SHA256** authentication with time-based tokens (anti-replay)
- **Certificate pinning** support (ca_cert)
- **IP whitelisting** per user with CIDR support
- **Memory safety** guaranteed by Rust (no buffer overflows)
- **Constant-time** comparison for secrets

---

## 📋 Protocol

1. **SMTP Handshake**: Client connects, server responds with Postfix-style greeting
2. **STARTTLS**: Connection upgrades to TLS
3. **Authentication**: HMAC-SHA256 token with timestamp
4. **Binary Mode**: Switch to fast binary frame protocol
5. **Tunneling**: SOCKS5 requests forwarded through encrypted tunnel

---

## 📝 License

MIT License - See [LICENSE](LICENSE) file

---

## 👤 Author

**Anubhav Gain** ([@anubhavg-icpl](https://github.com/anubhavg-icpl))
- Email: [anubhavg@infopercept.com](mailto:anubhavg@infopercept.com)
- Company: [InfoPercept Consulting Pvt Ltd](https://www.infopercept.com)

---

*Made with 🦀 for internet freedom*
