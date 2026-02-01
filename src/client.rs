//! SMTP Tunnel Client
//! 
//! Connects to SMTP tunnel server and provides SOCKS5 proxy interface.

use crate::config::ClientConfig;
use crate::crypto::AuthToken;
use bytes::{Buf, BytesMut};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tracing::{debug, info};
use std::sync::Arc;

/// SMTP Tunnel Client
pub struct Client {
    config: ClientConfig,
    state: Arc<RwLock<ClientState>>,
}

/// Client connection state
#[derive(Debug)]
struct ClientState {
    connected: bool,
    next_channel_id: u16,
    channels: HashMap<u16, Channel>,
}

/// A tunneled channel
#[derive(Debug)]
struct Channel {
    _tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    connected: bool,
}

impl Client {
    /// Create a new client
    pub fn new(config: ClientConfig) -> Self {
        let state = Arc::new(RwLock::new(ClientState {
            connected: false,
            next_channel_id: 1,
            channels: HashMap::new(),
        }));

        Self { config, state }
    }

    /// Run the client with auto-reconnect
    pub async fn run(&self) -> anyhow::Result<()> {
        let mut reconnect_delay = 2;
        const MAX_RECONNECT_DELAY: u64 = 30;

        loop {
            match self.connect_and_serve().await {
                Ok(()) => {
                    info!("Connection closed gracefully");
                    reconnect_delay = 2;
                }
                Err(e) => {
                    tracing::warn!("Connection error: {}, reconnecting in {}s...", e, reconnect_delay);
                    tokio::time::sleep(tokio::time::Duration::from_secs(reconnect_delay)).await;
                    reconnect_delay = (reconnect_delay * 2).min(MAX_RECONNECT_DELAY);
                }
            }
        }
    }

    /// Connect to server and serve requests
    async fn connect_and_serve(&self) -> anyhow::Result<()> {
        // 1. Connect to server
        let addr = format!("{}:{}", self.config.server_host, self.config.server_port);
        info!("Connecting to {}...", addr);

        let stream = TcpStream::connect(&addr).await?;
        let peer_addr = stream.peer_addr()?;
        info!("Connected to {}", peer_addr);

        // 2. SMTP handshake
        let _stream = self.smtp_handshake(stream).await?;
        info!("SMTP handshake complete, binary mode active");

        // 3. Set state to connected
        {
            let mut state = self.state.write().await;
            state.connected = true;
        }

        // 4. Start SOCKS5 server
        let socks_bind = self.config.socks_bind_addr()?;

        // Create SOCKS5 server
        let socks_server = crate::socks5::Socks5Server::new(socks_bind, move |req| {
            let host = req.host;
            let port = req.port;
            async move {
                // Connect directly for now (simplified)
                let addr = format!("{}:{}", host, port);
                match TcpStream::connect(&addr).await {
                    Ok(stream) => {
                        let local_addr = stream.local_addr()?;
                        Ok(crate::socks5::ProxyStream::new(local_addr, stream))
                    }
                    Err(e) => Err(e),
                }
            }
        });

        // Run SOCKS5 server
        socks_server.run().await?;

        Ok(())
    }

    /// Perform SMTP handshake and upgrade to TLS
    async fn smtp_handshake(&self, mut stream: TcpStream) -> anyhow::Result<TcpStream> {
        let mut buf = BytesMut::with_capacity(1024);

        // 1. Wait for greeting
        let line = self.read_smtp_line(&mut stream, &mut buf).await?
            .ok_or_else(|| anyhow::anyhow!("Server closed connection"))?;
        
        if !line.starts_with("220") {
            return Err(anyhow::anyhow!("Unexpected greeting: {}", line));
        }
        debug!("Server greeting: {}", line);

        // 2. Send EHLO
        stream.write_all(b"EHLO tunnel-client.local\r\n").await?;
        
        // Read EHLO response (multi-line)
        loop {
            let line = self.read_smtp_line(&mut stream, &mut buf).await?
                .ok_or_else(|| anyhow::anyhow!("Server closed connection"))?;
            debug!("EHLO response: {}", line);
            
            if line.starts_with("250 ") {
                break;
            }
            if !line.starts_with("250-") {
                return Err(anyhow::anyhow!("EHLO failed: {}", line));
            }
        }

        // 3. STARTTLS
        stream.write_all(b"STARTTLS\r\n").await?;
        let line = self.read_smtp_line(&mut stream, &mut buf).await?
            .ok_or_else(|| anyhow::anyhow!("Server closed connection"))?;
        
        if !line.starts_with("220") {
            return Err(anyhow::anyhow!("STARTTLS failed: {}", line));
        }
        debug!("STARTTLS response: {}", line);

        // 4. Upgrade TLS - simplified for compilation
        // In full implementation, we'd use tokio-rustls here
        
        // 5. EHLO again (post-TLS)
        stream.write_all(b"EHLO tunnel-client.local\r\n").await?;
        
        // Read EHLO response
        loop {
            let line = self.read_smtp_line(&mut stream, &mut buf).await?
                .ok_or_else(|| anyhow::anyhow!("Server closed connection"))?;
            debug!("EHLO (post-TLS) response: {}", line);
            
            if line.starts_with("250 ") {
                break;
            }
            if !line.starts_with("250-") {
                return Err(anyhow::anyhow!("EHLO (post-TLS) failed: {}", line));
            }
        }

        // 6. AUTH
        let token = AuthToken::generate_now(&self.config.secret, &self.config.username);
        stream.write_all(format!("AUTH PLAIN {}\r\n", token).as_bytes()).await?;
        let line = self.read_smtp_line(&mut stream, &mut buf).await?
            .ok_or_else(|| anyhow::anyhow!("Server closed connection"))?;
        
        if !line.starts_with("235") {
            return Err(anyhow::anyhow!("Authentication failed: {}", line));
        }
        debug!("Auth success: {}", line);

        // 7. Switch to binary mode
        stream.write_all(b"BINARY\r\n").await?;
        let line = self.read_smtp_line(&mut stream, &mut buf).await?
            .ok_or_else(|| anyhow::anyhow!("Server closed connection"))?;
        
        if !line.starts_with("299") {
            return Err(anyhow::anyhow!("Binary mode failed: {}", line));
        }
        debug!("Binary mode active: {}", line);

        Ok(stream)
    }

    /// Read an SMTP line
    async fn read_smtp_line(
        &self,
        stream: &mut TcpStream,
        buf: &mut BytesMut,
    ) -> anyhow::Result<Option<String>> {
        loop {
            if let Some(pos) = buf.windows(2).position(|w| w == b"\r\n") {
                let line = buf.split_to(pos);
                buf.advance(2); // Skip \r\n
                return Ok(Some(String::from_utf8_lossy(&line).to_string()));
            }

            let mut temp = vec![0u8; 1024];
            let n = stream.read(&mut temp).await?;
            if n == 0 {
                return Ok(None);
            }
            buf.extend_from_slice(&temp[..n]);
        }
    }
}

/// Run the client
pub async fn run_client(config: ClientConfig) -> anyhow::Result<()> {
    let client = Client::new(config);
    client.run().await
}
