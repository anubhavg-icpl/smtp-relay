//! SMTP Tunnel Server Binary

use anyhow::Result;
use clap::Parser;
use smtp_tunnel::config::{Config, UsersConfig};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// SMTP Tunnel Server
#[derive(Parser, Debug)]
#[command(name = "smtp-tunnel-server")]
#[command(about = "SMTP tunnel server that forwards traffic")]
#[command(version = smtp_tunnel::VERSION)]
struct Args {
    /// Configuration file
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,

    /// Users file
    #[arg(short, long)]
    users: Option<PathBuf>,

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

    // Load config
    let config = if args.config.exists() {
        Config::from_file(&args.config)?
    } else {
        info!("No config file found, using defaults");
        Config::default()
    };

    // Load users
    let users_file = args
        .users
        .unwrap_or_else(|| PathBuf::from(&config.server.users_file));

    let users = if users_file.exists() {
        UsersConfig::from_file(&users_file)?
    } else {
        eprintln!("Error: Users file not found: {}", users_file.display());
        eprintln!("Create a users file with:");
        eprintln!();
        eprintln!("users:");
        eprintln!("  alice:");
        eprintln!("    secret: 'your-secret-here'");
        eprintln!("    logging: true");
        std::process::exit(1);
    };

    if users.users.is_empty() {
        eprintln!("Error: No users configured in {}", users_file.display());
        std::process::exit(1);
    }

    // Check TLS certificates
    if !std::path::Path::new(&config.server.cert_file).exists() {
        eprintln!(
            "Error: Certificate file not found: {}",
            config.server.cert_file
        );
        eprintln!("Generate certificates with: smtp-tunnel-gen-certs");
        std::process::exit(1);
    }

    if !std::path::Path::new(&config.server.key_file).exists() {
        eprintln!("Error: Key file not found: {}", config.server.key_file);
        eprintln!("Generate certificates with: smtp-tunnel-gen-certs");
        std::process::exit(1);
    }

    info!("SMTP Tunnel Server {}", smtp_tunnel::VERSION);
    info!("Loaded {} users", users.users.len());

    // Run server
    smtp_tunnel::server::run_server(config.server, users).await?;

    Ok(())
}
