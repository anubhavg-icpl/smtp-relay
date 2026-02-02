//! SMTP Tunnel Client Binary

use anyhow::Result;
use clap::Parser;
use smtp_tunnel::config::{ClientConfig, Config};
use std::path::PathBuf;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

/// SMTP Tunnel Client
#[derive(Parser, Debug)]
#[command(name = "smtp-tunnel-client")]
#[command(about = "SOCKS5 proxy that tunnels through SMTP")]
#[command(version = smtp_tunnel::VERSION)]
struct Args {
    /// Configuration file
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,

    /// Server hostname
    #[arg(long)]
    server: Option<String>,

    /// Server port
    #[arg(long)]
    server_port: Option<u16>,

    /// Local SOCKS port
    #[arg(short, long)]
    socks_port: Option<u16>,

    /// Username
    #[arg(short, long)]
    username: Option<String>,

    /// Secret
    #[arg(short, long)]
    secret: Option<String>,

    /// CA certificate file
    #[arg(long)]
    ca_cert: Option<String>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let level = if args.debug {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Load or create config
    let mut config = if args.config.exists() {
        let cfg = Config::from_file(&args.config)?;
        cfg.client
    } else {
        info!("No config file found, using defaults");
        ClientConfig::default()
    };

    // Apply command line overrides
    if let Some(server) = args.server {
        config.server_host = server;
    }
    if let Some(port) = args.server_port {
        config.server_port = port;
    }
    if let Some(port) = args.socks_port {
        config.socks_port = port;
    }
    if let Some(username) = args.username {
        config.username = username;
    }
    if let Some(secret) = args.secret {
        config.secret = secret;
    }
    if let Some(ca_cert) = args.ca_cert {
        config.ca_cert = Some(ca_cert);
    }

    // Validate config
    if config.server_host.is_empty() {
        eprintln!("Error: Server hostname is required");
        eprintln!("Use --server <hostname> or set in config file");
        std::process::exit(1);
    }

    if config.username.is_empty() {
        eprintln!("Error: Username is required");
        eprintln!("Use --username <name> or set in config file");
        std::process::exit(1);
    }

    if config.secret.is_empty() {
        eprintln!("Error: Secret is required");
        eprintln!("Use --secret <secret> or set in config file");
        std::process::exit(1);
    }

    info!("SMTP Tunnel Client {}", smtp_tunnel::VERSION);
    info!("Server: {}:{}", config.server_host, config.server_port);
    info!("SOCKS5: {}:{}", config.socks_host, config.socks_port);
    info!("Username: {}", config.username);

    // Run client
    smtp_tunnel::client::run_client(config).await?;

    Ok(())
}
