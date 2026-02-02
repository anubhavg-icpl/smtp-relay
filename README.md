<div align="center">

# 🦀 SMTP Tunnel Proxy

[![Author](https://img.shields.io/badge/Author-anubhavg--icpl-blue?style=flat-square)](https://github.com/anubhavg-icpl)
[![Email](https://img.shields.io/badge/Email-anubhavg%40infopercept.com-red?style=flat-square)](mailto:anubhavg@infopercept.com)
[![Version](https://img.shields.io/badge/Version-2.0.0-green?style=flat-square)]()
[![License](https://img.shields.io/badge/License-MIT-yellow?style=flat-square)]()

**High-speed covert tunnel disguising TCP traffic as SMTP to bypass DPI firewalls**

</div>

---

## Overview

SMTP Tunnel Proxy is a high-performance network tunnel that masks your TCP traffic as legitimate SMTP email communication. It provides a secure SOCKS5 proxy interface that tunnels through port 587 (standard SMTP submission port), making it nearly impossible for Deep Packet Inspection (DPI) systems to detect or block.

Built with **Rust** for maximum performance, memory safety, and minimal resource usage.

---

## Features

<table>
<tr>
<td>

🔒 **TLS 1.3 Encryption**  
Modern encryption powered by rustls

⚡ **Zero-Cost Async**  
Handle thousands of connections with tokio

👥 **Multi-User Support**  
Per-user secrets and IP whitelists

🌐 **SOCKS5 Proxy**  
Standard proxy interface (RFC 1928)

</td>
<td>

🎭 **DPI Evasion**  
Mimics real Postfix SMTP servers

🔑 **HMAC-SHA256 Auth**  
Time-based tokens with anti-replay

📡 **Connection Multiplexing**  
Multiple connections over single tunnel

🦀 **Memory Safe**  
No buffer overflows, no segfaults

</td>
</tr>
</table>

---

## Architecture

```
┌──────────────────┐      ┌──────────────────┐      ┌──────────────────┐
│   Browser/curl   │─────▶│  SOCKS5 Proxy    │─────▶│   SMTP Client    │
│                  │ TCP  │  127.0.0.1:1080  │      │   (Rust/tokio)   │
└──────────────────┘      └──────────────────┘      └────────┬─────────┘
                                                             │
                                                             │ TLS + SMTP
                                                             │ on Port 587
                                                             ▼
┌──────────────────┐      ┌──────────────────┐      ┌──────────────────┐
│    Internet      │◀─────│   SMTP Server    │◀─────│   Your Server    │
│   (Any TCP)      │      │   (Rust/tokio)   │      │   (VPS/Dedi)     │
└──────────────────┘      └──────────────────┘      └──────────────────┘
```

---

## Quick Start

### Server Installation (VPS)

```bash
# Download and install
curl -sSL https://raw.githubusercontent.com/anubhavg-icpl/smtp-relay/main/install.sh | sudo bash

# Generate certificates
smtp-tunnel-gen-certs --hostname mail.example.com

# Create a user
smtp-tunnel-adduser alice

# Start the server
systemctl start smtp-tunnel
```

### Client Usage

```bash
# Extract the client package (alice.zip from server)
unzip alice.zip && cd alice

# Run the client
./start.sh

# Test the connection
curl -x socks5h://127.0.0.1:1080 https://ifconfig.me
```

---

## Binaries

| Binary | Size | Description |
|--------|------|-------------|
| `smtp-tunnel-server` | ~1.6 MB | Tunnel server (runs on VPS) |
| `smtp-tunnel-client` | ~1.0 MB | SOCKS5 proxy client |
| `smtp-tunnel-gen-certs` | ~0.9 MB | TLS certificate generator |
| `smtp-tunnel-adduser` | ~0.9 MB | User management tool |
| `smtp-tunnel-deluser` | ~0.7 MB | Remove users |
| `smtp-tunnel-listusers` | ~0.7 MB | List all users |

---

## Performance

<table>
<tr><td>Binary Size</td><td>~1-2 MB</td><td>Minimal footprint</td></tr>
<tr><td>Memory Usage</td><td>~5 MB base</td><td>Efficient resource usage</td></tr>
<tr><td>Throughput</td><td>~1 Gbps</td><td>Limited by network</td></tr>
<tr><td>Latency</td><td>Predictable</td><td>No GC pauses</td></tr>
<tr><td>Safety</td><td>Compile-time</td><td>Memory safe (Rust)</td></tr>
</table>

---

## Configuration

### Server (`/etc/smtp-tunnel/config.yaml`)

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

### Users (`/etc/smtp-tunnel/users.yaml`)

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
    whitelist: []  # Allow any IP
```

---

## Building from Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/anubhavg-icpl/smtp-relay
cd smtp-relay
cargo build --release

# Binaries in target/release/
```

---

## How It Works

1. **Connection**: Client connects to server on port 587 (standard SMTP submission)
2. **SMTP Handshake**: Server presents itself as Postfix mail server
3. **STARTTLS**: Connection upgrades to TLS 1.3 encryption
4. **Authentication**: Client authenticates with HMAC-SHA256 token (time-based, anti-replay)
5. **Binary Mode**: After auth, switches to fast binary frame protocol
6. **Tunneling**: SOCKS5 requests forwarded through encrypted tunnel to destination

---

## Security

- **TLS 1.3** for transport encryption
- **HMAC-SHA256** authentication with 5-minute token expiration
- **Certificate pinning** support
- **IP whitelisting** per user with CIDR notation
- **Memory safety** guaranteed by Rust's ownership model
- **Constant-time** secret comparison

---

## License

MIT License - See [LICENSE](LICENSE) file

---

## Author

**Anubhav Gain** ([@anubhavg-icpl](https://github.com/anubhavg-icpl))

- Email: [anubhavg@infopercept.com](mailto:anubhavg@infopercept.com)
- Company: [InfoPercept Consulting Pvt Ltd](https://www.infopercept.com)

---

<div align="center">

*Made with 🦀 for internet freedom*

</div>
