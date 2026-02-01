//! Configuration management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;

/// Server configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// Listen address
    #[serde(default = "default_host")]
    pub host: String,
    /// Listen port (default: 587)
    #[serde(default = "default_port")]
    pub port: u16,
    /// SMTP hostname
    #[serde(default = "default_hostname")]
    pub hostname: String,
    /// TLS certificate file
    #[serde(default = "default_cert_file")]
    pub cert_file: String,
    /// TLS key file
    #[serde(default = "default_key_file")]
    pub key_file: String,
    /// Users file path
    #[serde(default = "default_users_file")]
    pub users_file: String,
    /// Global logging setting
    #[serde(default = "default_true")]
    pub log_users: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            hostname: default_hostname(),
            cert_file: default_cert_file(),
            key_file: default_key_file(),
            users_file: default_users_file(),
            log_users: true,
        }
    }
}

/// Client configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientConfig {
    /// Server hostname
    #[serde(default)]
    pub server_host: String,
    /// Server port
    #[serde(default = "default_port")]
    pub server_port: u16,
    /// Local SOCKS5 port
    #[serde(default = "default_socks_port")]
    pub socks_port: u16,
    /// Local SOCKS5 bind address
    #[serde(default = "default_socks_host")]
    pub socks_host: String,
    /// Username
    #[serde(default)]
    pub username: String,
    /// Secret
    #[serde(default)]
    pub secret: String,
    /// CA certificate file (optional but recommended)
    #[serde(default)]
    pub ca_cert: Option<String>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_host: String::new(),
            server_port: default_port(),
            socks_port: default_socks_port(),
            socks_host: default_socks_host(),
            username: String::new(),
            secret: String::new(),
            ca_cert: None,
        }
    }
}

/// User configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserEntry {
    /// Authentication secret
    pub secret: String,
    /// IP whitelist (empty = allow all)
    #[serde(default)]
    pub whitelist: Vec<String>,
    /// Enable logging for this user
    #[serde(default = "default_true")]
    pub logging: bool,
}

/// Users configuration file
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UsersConfig {
    pub users: HashMap<String, UserEntry>,
}

/// Full configuration file (server + client)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub client: ClientConfig,
}

// Default value functions
fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    587
}
fn default_socks_port() -> u16 {
    1080
}
fn default_socks_host() -> String {
    "127.0.0.1".to_string()
}
fn default_hostname() -> String {
    "mail.example.com".to_string()
}
fn default_cert_file() -> String {
    "server.crt".to_string()
}
fn default_key_file() -> String {
    "server.key".to_string()
}
fn default_users_file() -> String {
    "users.yaml".to_string()
}
fn default_true() -> bool {
    true
}

impl Config {
    /// Load configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Create default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            client: ClientConfig::default(),
        }
    }
}

impl Default for UsersConfig {
    fn default() -> Self {
        Self {
            users: HashMap::new(),
        }
    }
}

impl UsersConfig {
    /// Load users from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: UsersConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Save users to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get user by name
    pub fn get_user(&self, username: &str) -> Option<&UserEntry> {
        self.users.get(username)
    }

    /// Add or update user
    pub fn set_user(&mut self, username: impl Into<String>, entry: UserEntry) {
        self.users.insert(username.into(), entry);
    }

    /// Remove user
    pub fn remove_user(&mut self, username: &str) -> Option<UserEntry> {
        self.users.remove(username)
    }

    /// Check if IP is whitelisted for user
    pub fn is_ip_whitelisted(&self, username: &str, ip: &str) -> bool {
        let Some(user) = self.users.get(username) else {
            return false;
        };

        // Empty whitelist = allow all
        if user.whitelist.is_empty() {
            return true;
        }

        // Check each whitelist entry
        for entry in &user.whitelist {
            if entry == ip {
                return true;
            }
            // Try CIDR parsing
            if let Ok(network) = entry.parse::<ipnet::IpNet>() {
                if let Ok(addr) = ip.parse::<std::net::IpAddr>() {
                    if network.contains(&addr) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

impl ServerConfig {
    /// Get socket address to bind to
    pub fn bind_addr(&self) -> anyhow::Result<SocketAddr> {
        let addr = format!("{}:{}", self.host, self.port).parse()?;
        Ok(addr)
    }
}

impl ClientConfig {
    /// Get server socket address
    pub fn server_addr(&self) -> anyhow::Result<SocketAddr> {
        let addr = format!("{}:{}", self.server_host, self.server_port).parse()?;
        Ok(addr)
    }

    /// Get SOCKS5 bind address
    pub fn socks_bind_addr(&self) -> anyhow::Result<SocketAddr> {
        let addr = format!("{}:{}", self.socks_host, self.socks_port).parse()?;
        Ok(addr)
    }
}

/// Generate example configuration
pub fn generate_example_config() -> String {
    r#"# SMTP Tunnel Configuration
# Copy this file and customize for your setup

# ============================================================================
# Server Configuration (for smtp-tunnel-server)
# ============================================================================
server:
  # Listen address (0.0.0.0 for all interfaces)
  host: "0.0.0.0"

  # SMTP submission port (587 is standard)
  port: 587

  # Hostname to advertise in SMTP greeting
  # Use a realistic hostname that matches your server's DNS
  hostname: "mail.example.com"

  # TLS certificate and key files
  cert_file: "server.crt"
  key_file: "server.key"

  # Users configuration file
  users_file: "users.yaml"

  # Global logging setting
  log_users: true

# ============================================================================
# Client Configuration (for smtp-tunnel-client)
# ============================================================================
client:
  # Tunnel server domain name (FQDN required for certificate verification)
  server_host: "mail.example.com"

  # Tunnel server port
  server_port: 587

  # Local SOCKS5 proxy port
  socks_port: 1080

  # Local SOCKS5 bind address (127.0.0.1 = localhost only)
  socks_host: "127.0.0.1"

  # Username and secret (set per-user)
  username: "alice"
  secret: "your-secret-here"

  # CA certificate for server verification (RECOMMENDED for security)
  ca_cert: "ca.crt"
"#
    .to_string()
}

/// Generate example users file
pub fn generate_example_users() -> String {
    r#"# SMTP Tunnel Users
# Managed by smtp-tunnel-adduser

users:
  alice:
    secret: "auto-generated-secret-here"
    logging: true
    # whitelist:
    #   - 192.168.1.100
    #   - 10.0.0.0/8

  bob:
    secret: "another-secret-here"
    logging: true
    whitelist: []
"#
    .to_string()
}
