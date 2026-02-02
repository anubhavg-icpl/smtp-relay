//! SMTP Tunnel Proxy - Rust Implementation
//!
//! A high-speed covert tunnel that disguises TCP traffic as SMTP email communication
//! to bypass Deep Packet Inspection (DPI) firewalls.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐      ┌─────────────┐      ┌─────────────┐      ┌──────────────┐
//! │ Application │─────▶│   Client    │─────▶│   Server    │─────▶│  Internet    │
//! │  (Browser)  │ TCP  │ SOCKS5:1080 │ SMTP │  Port 587   │ TCP  │              │
//! │             │◀─────│             │◀─────│             │◀─────│              │
//! └─────────────┘      └─────────────┘      └─────────────┘      └──────────────┘
//! ```

pub mod client;
pub mod config;
pub mod crypto;
pub mod proto;
pub mod server;
pub mod socks5;

// Re-export commonly used items
pub use config::{ClientConfig, Config, ServerConfig, UserEntry, UsersConfig};
pub use crypto::{AuthToken, generate_secret};
pub use proto::{Frame, FrameType};

use thiserror::Error;

/// Error types for SMTP Tunnel
#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("Authentication failed")]
    AuthFailed,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Protocol error: {0}")]
    Protocol(String),
}

/// Result type for SMTP Tunnel
pub type Result<T> = std::result::Result<T, Error>;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
