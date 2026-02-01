//! Delete User Tool - Removes users from configuration

use anyhow::Result;
use clap::Parser;
use smtp_tunnel::config::UsersConfig;
use std::path::PathBuf;

/// Remove a user from SMTP Tunnel
#[derive(Parser, Debug)]
#[command(name = "smtp-tunnel-deluser")]
#[command(about = "Remove a user from SMTP Tunnel")]
#[command(version)]
struct Args {
    /// Username to remove
    username: String,

    /// Users file
    #[arg(short, long, default_value = "/etc/smtp-tunnel/users.yaml")]
    users_file: PathBuf,

    /// Do not ask for confirmation
    #[arg(short, long)]
    force: bool,
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
        eprintln!("Error: Users file not found: {}", users_file.display());
        std::process::exit(1);
    };

    // Check if user exists
    if !users.users.contains_key(&args.username) {
        eprintln!("Error: User '{}' not found", args.username);
        std::process::exit(1);
    }

    // Confirm deletion
    if !args.force {
        print!("Delete user '{}'? [y/N]: ", args.username);
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut response = String::new();
        std::io::stdin().read_line(&mut response)?;
        if response.trim().to_lowercase() != "y" {
            println!("Cancelled");
            return Ok(());
        }
    }

    // Remove user
    users.users.remove(&args.username);

    // Save users file
    users.save_to_file(&users_file)?;
    println!("User '{}' removed", args.username);

    // Remind about ZIP files
    let zip_file = format!("{}.zip", args.username);
    if std::path::Path::new(&zip_file).exists() {
        println!(
            "Note: Client package '{zip_file}' still exists - delete manually if needed"
        );
    }

    Ok(())
}
