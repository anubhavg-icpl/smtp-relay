//! Add User Tool - Creates users and generates client packages

use anyhow::Result;
use clap::Parser;
use smtp_tunnel::config::{Config, UsersConfig, UserEntry};
use smtp_tunnel::crypto::generate_secret;
use std::fs;
use std::path::{Path, PathBuf};

/// Add a new user to SMTP Tunnel
#[derive(Parser, Debug)]
#[command(name = "smtp-tunnel-adduser")]
#[command(about = "Add a new user and generate client package")]
#[command(version)]
struct Args {
    /// Username to add
    username: String,

    /// Secret (auto-generated if not provided)
    #[arg(short, long)]
    secret: Option<String>,

    /// IP whitelist (can specify multiple)
    #[arg(short, long)]
    whitelist: Vec<String>,

    /// Disable logging for this user
    #[arg(long)]
    no_logging: bool,

    /// Users file
    #[arg(short, long, default_value = "/etc/smtp-tunnel/users.yaml")]
    users_file: PathBuf,

    /// Server config file
    #[arg(short, long, default_value = "/etc/smtp-tunnel/config.yaml")]
    config: PathBuf,

    /// Output directory for ZIP file
    #[arg(short, long, default_value = ".")]
    output_dir: PathBuf,

    /// Do not generate client ZIP package
    #[arg(long)]
    no_package: bool,
}

fn create_client_config(server_host: &str, server_port: u16, username: &str, secret: &str) -> String {
    format!(r#"# SMTP Tunnel Client Configuration
# Generated for user: {username}

client:
  # Server connection
  server_host: "{server_host}"
  server_port: {server_port}

  # Authentication
  username: "{username}"
  secret: "{secret}"

  # Local SOCKS5 proxy
  socks_port: 1080
  socks_host: "127.0.0.1"

  # CA certificate for server verification
  ca_cert: "ca.crt"
"#)
}

fn create_readme(username: &str) -> String {
    format!(r#"# SMTP Tunnel Client - {username}

## Quick Start

1. Install the client binary:
   - Download `smtp-tunnel-client` for your platform
   - Make it executable: chmod +x smtp-tunnel-client

2. Run the client:
   ./smtp-tunnel-client -c config.yaml

3. Configure your browser/apps to use SOCKS5 proxy:
   Host: 127.0.0.1
   Port: 1080

## Files

- config.yaml    - Your configuration (pre-configured)
- ca.crt         - Server certificate for verification
- README.txt     - This file

## Test Connection

curl -x socks5h://127.0.0.1:1080 https://ifconfig.me

## Configuration

Edit config.yaml to change settings:
- server_host: Your server's domain name
- server_port: 587 (default SMTP submission port)
- socks_port: 1080 (local proxy port)
"#)
}

fn create_start_sh(username: &str) -> String {
    format!(r#"#!/bin/bash
#
# SMTP Tunnel Client Launcher
# User: {username}
#

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

clear
echo ""
echo -e "${{CYAN}}"
echo "  ╔═══════════════════════════════════════════════════════════╗"
echo "  ║                                                           ║"
echo "  ║   SMTP Tunnel Proxy Client                                ║"
echo "  ║   User: {username:50}║"
echo "  ║                                                           ║"
echo "  ╚═══════════════════════════════════════════════════════════╝"
echo -e "${{NC}}"
echo ""

# Find binary
if [ -f "./smtp-tunnel-client" ]; then
    BINARY="./smtp-tunnel-client"
elif command -v smtp-tunnel-client &> /dev/null; then
    BINARY="smtp-tunnel-client"
else
    echo -e "${{RED}}[ERROR]${{NC}} smtp-tunnel-client binary not found!"
    echo ""
    echo "Please download the client binary from your server."
    exit 1
fi

echo -e "${{GREEN}}[INFO]${{NC}} Found binary: $BINARY"
echo ""
echo -e "${{GREEN}}[INFO]${{NC}} Starting SMTP Tunnel..."
echo -e "${{GREEN}}[INFO]${{NC}} SOCKS5 proxy will be available at 127.0.0.1:1080"
echo ""
echo -e "Press ${{YELLOW}}Ctrl+C${{NC}} to stop"
echo "─────────────────────────────────────────────────────────────"
echo ""

$BINARY -c config.yaml

echo ""
echo -e "${{YELLOW}}Connection closed.${{NC}}"
"#)
}

fn create_start_bat(username: &str) -> String {
    format!(r#"@echo off
title SMTP Tunnel - {username}

echo.
echo  ╔═══════════════════════════════════════════════════════════╗
echo  ║                                                           ║
echo  ║   SMTP Tunnel Proxy Client                                ║
echo  ║   User: {username:50}║
echo  ║                                                           ║
echo  ╚═══════════════════════════════════════════════════════════╝
echo.

:: Find binary
if exist "smtp-tunnel-client.exe" (
    set BINARY=smtp-tunnel-client.exe
) else if exist "smtp-tunnel-client" (
    set BINARY=smtp-tunnel-client
) else (
    echo [ERROR] smtp-tunnel-client binary not found!
    echo.
    echo Please download the client binary from your server.
    pause
    exit /b 1
)

echo [INFO] Found binary: %BINARY%
echo.
echo [INFO] Starting SMTP Tunnel...
echo [INFO] SOCKS5 proxy will be available at 127.0.0.1:1080
echo.
echo Press Ctrl+C to stop
echo ─────────────────────────────────────────────────────────────
echo.

%BINARY% -c config.yaml

echo.
echo Connection closed.
pause
"#)
}

fn create_client_package(
    username: &str,
    secret: &str,
    server_host: &str,
    server_port: u16,
    base_dir: &Path,
    output_dir: &Path,
) -> Result<PathBuf> {
    use std::io::Write;

    // Create temp directory
    let temp_dir = tempfile::tempdir()?;
    let pkg_dir = temp_dir.path().join(username);
    fs::create_dir_all(&pkg_dir)?;

    // Copy CA cert if exists
    let ca_cert_src = base_dir.join("ca.crt");
    let ca_cert_dst = pkg_dir.join("ca.crt");
    if ca_cert_src.exists() {
        fs::copy(&ca_cert_src, &ca_cert_dst)?;
    } else {
        println!("Warning: ca.crt not found - client will not be able to verify server");
    }

    // Generate client config
    let config_content = create_client_config(server_host, server_port, username, secret);
    let config_path = pkg_dir.join("config.yaml");
    fs::write(&config_path, config_content)?;

    // Create README
    let readme_content = create_readme(username);
    let readme_path = pkg_dir.join("README.txt");
    fs::write(&readme_path, readme_content)?;

    // Create start scripts
    let start_sh = create_start_sh(username);
    let start_sh_path = pkg_dir.join("start.sh");
    fs::write(&start_sh_path, start_sh)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&start_sh_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&start_sh_path, perms)?;
    }

    let start_bat = create_start_bat(username);
    let start_bat_path = pkg_dir.join("start.bat");
    fs::write(&start_bat_path, start_bat)?;

    // Create ZIP file
    let zip_filename = format!("{}.zip", username);
    let zip_path = output_dir.join(&zip_filename);

    let file = fs::File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(file);

    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    for entry in walkdir::WalkDir::new(&pkg_dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let name = path.strip_prefix(&temp_dir)?;
            zip.start_file(name.to_string_lossy(), options)?;
            let content = fs::read(path)?;
            zip.write_all(&content)?;
        }
    }

    zip.finish()?;

    Ok(zip_path)
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Get base directory
    let base_dir = std::env::current_dir()?;

    // Load existing users
    let users_file = if args.users_file.is_absolute() {
        args.users_file.clone()
    } else {
        base_dir.join(&args.users_file)
    };

    let mut users = if users_file.exists() {
        UsersConfig::from_file(&users_file)?
    } else {
        UsersConfig::default()
    };

    // Check if user already exists
    if users.users.contains_key(&args.username) {
        eprintln!("Error: User '{}' already exists", args.username);
        std::process::exit(1);
    }

    // Generate secret if not provided
    let secret = args.secret.unwrap_or_else(generate_secret);

    // Create user entry
    let entry = UserEntry {
        secret: secret.clone(),
        whitelist: if args.whitelist.is_empty() { vec![] } else { args.whitelist },
        logging: !args.no_logging,
    };

    // Add user
    users.users.insert(args.username.clone(), entry);

    // Save users file
    users.save_to_file(&users_file)?;
    println!("User '{}' added to {}", args.username, users_file.display());

    // Generate client package
    if !args.no_package {
        // Load server config to get hostname and port
        let config_file = if args.config.is_absolute() {
            args.config.clone()
        } else {
            base_dir.join(&args.config)
        };

        let (server_host, server_port) = if config_file.exists() {
            let config = Config::from_file(&config_file)?;
            (
                config.server.hostname,
                config.server.port,
            )
        } else {
            println!("Warning: Config file {} not found, using defaults", config_file.display());
            ("localhost".to_string(), 587)
        };

        let output_dir = if args.output_dir.is_absolute() {
            args.output_dir.clone()
        } else {
            std::env::current_dir()?.join(&args.output_dir)
        };

        let zip_path = create_client_package(
            &args.username,
            &secret,
            &server_host,
            server_port,
            &base_dir,
            &output_dir,
        )?;

        println!("Client package created: {}", zip_path.display());
        println!();
        println!("Send this ZIP file to the user. They need to:");
        println!("  1. Extract the ZIP");
        println!("  2. Download smtp-tunnel-client binary for their platform");
        println!("  3. Run ./start.sh (Linux/Mac) or start.bat (Windows)");
    }

    Ok(())
}
