//! List Users Tool - Shows all configured users

use anyhow::Result;
use clap::Parser;
use smtp_tunnel::config::UsersConfig;
use std::path::PathBuf;

/// List all SMTP Tunnel users
#[derive(Parser, Debug)]
#[command(name = "smtp-tunnel-listusers")]
#[command(about = "List all SMTP Tunnel users")]
#[command(version)]
struct Args {
    /// Users file
    #[arg(short, long, default_value = "/etc/smtp-tunnel/users.yaml")]
    users_file: PathBuf,

    /// Show detailed information
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Get base directory
    let base_dir = std::env::current_dir()?;

    // Load users
    let users_file = if args.users_file.is_absolute() {
        args.users_file.clone()
    } else {
        base_dir.join(&args.users_file)
    };

    let users = if users_file.exists() {
        UsersConfig::from_file(&users_file)?
    } else {
        println!("No users configured");
        println!("Use smtp-tunnel-adduser to add users");
        return Ok(());
    };

    if users.users.is_empty() {
        println!("No users configured");
        println!("Use smtp-tunnel-adduser to add users");
        return Ok(());
    }

    println!("Users ({}):", users.users.len());
    println!("{}", "-".repeat(60));

    let mut user_list: Vec<_> = users.users.iter().collect();
    user_list.sort_by(|a, b| a.0.cmp(b.0));

    for (username, entry) in user_list {
        if args.verbose {
            println!("\n  {}:", username);
            let secret_preview = if entry.secret.len() > 12 {
                format!(
                    "{}...{}",
                    &entry.secret[..8],
                    &entry.secret[entry.secret.len() - 4..]
                )
            } else {
                entry.secret.clone()
            };
            println!("    Secret: {}", secret_preview);
            if entry.whitelist.is_empty() {
                println!("    Whitelist: (any IP)");
            } else {
                println!("    Whitelist: {}", entry.whitelist.join(", "));
            }
            println!(
                "    Logging: {}",
                if entry.logging { "enabled" } else { "disabled" }
            );
        } else {
            let whitelist_info = if entry.whitelist.is_empty() {
                String::new()
            } else {
                format!(" [{} IPs]", entry.whitelist.len())
            };
            let logging_info = if !entry.logging { " [no-log]" } else { "" };
            println!("  {}{}{}", username, whitelist_info, logging_info);
        }
    }

    if !args.verbose {
        println!();
        println!("Use -v for detailed information");
    }

    Ok(())
}
