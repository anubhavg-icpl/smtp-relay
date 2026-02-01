//! SOCKS5 Proxy Server
//!
//! Implements SOCKS5 protocol (RFC 1928) for local proxy interface.

use bytes::{BufMut, BytesMut};
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, info, trace, warn};

/// SOCKS5 protocol constants
pub const VERSION: u8 = 0x05;
pub const AUTH_NONE: u8 = 0x00;
pub const AUTH_GSSAPI: u8 = 0x01;
pub const AUTH_PASSWORD: u8 = 0x02;
pub const AUTH_NO_ACCEPTABLE: u8 = 0xFF;

/// SOCKS5 commands
pub const CMD_CONNECT: u8 = 0x01;
pub const CMD_BIND: u8 = 0x02;
pub const CMD_UDP_ASSOCIATE: u8 = 0x03;

/// SOCKS5 address types
pub const ATYP_IPV4: u8 = 0x01;
pub const ATYP_DOMAIN: u8 = 0x03;
pub const ATYP_IPV6: u8 = 0x04;

/// SOCKS5 reply codes
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Reply {
    Success = 0x00,
    GeneralFailure = 0x01,
    NotAllowed = 0x02,
    NetworkUnreachable = 0x03,
    HostUnreachable = 0x04,
    ConnectionRefused = 0x05,
    TtlExpired = 0x06,
    CommandNotSupported = 0x07,
    AddressNotSupported = 0x08,
}

/// SOCKS5 request info
#[derive(Debug, Clone)]
pub struct ConnectRequest {
    pub host: String,
    pub port: u16,
}

/// SOCKS5 server
pub struct Socks5Server<F> {
    bind_addr: SocketAddr,
    handler: F,
}

impl<F, Fut> Socks5Server<F>
where
    F: Fn(ConnectRequest) -> Fut + Clone + Send + 'static,
    Fut: std::future::Future<Output = io::Result<ProxyStream>> + Send,
{
    /// Create a new SOCKS5 server
    pub fn new(bind_addr: SocketAddr, handler: F) -> Self {
        Self { bind_addr, handler }
    }

    /// Start the server
    pub async fn run(self) -> io::Result<()> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        info!("SOCKS5 proxy listening on {}", self.bind_addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            trace!("SOCKS5 connection from {}", addr);

            let handler = self.handler.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_client(stream, handler).await {
                    debug!("SOCKS5 client error: {}", e);
                }
            });
        }
    }
}

/// Handle a SOCKS5 client connection
async fn handle_client<F, Fut>(mut stream: TcpStream, handler: F) -> io::Result<()>
where
    F: FnOnce(ConnectRequest) -> Fut + Send,
    Fut: std::future::Future<Output = io::Result<ProxyStream>> + Send,
{
    // 1. Greeting
    let mut buf = [0u8; 2];
    stream.read_exact(&mut buf).await?;

    if buf[0] != VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid SOCKS version",
        ));
    }

    let nmethods = buf[1] as usize;
    let mut methods = vec![0u8; nmethods];
    stream.read_exact(&mut methods).await?;

    // We only support no authentication
    if !methods.contains(&AUTH_NONE) {
        stream.write_all(&[VERSION, AUTH_NO_ACCEPTABLE]).await?;
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No acceptable auth method",
        ));
    }

    // Select no authentication
    stream.write_all(&[VERSION, AUTH_NONE]).await?;

    // 2. Request
    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).await?;

    if buf[0] != VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid SOCKS version in request",
        ));
    }

    let cmd = buf[1];
    let atyp = buf[3];

    if cmd != CMD_CONNECT {
        send_reply(&mut stream, Reply::CommandNotSupported, None).await?;
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Unsupported command",
        ));
    }

    // Parse destination address
    let (host, port) = match atyp {
        ATYP_IPV4 => {
            let mut addr = [0u8; 4];
            stream.read_exact(&mut addr).await?;
            let port = stream.read_u16().await?;
            let ip = Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3]);
            (ip.to_string(), port)
        }
        ATYP_DOMAIN => {
            let len = stream.read_u8().await?;
            let mut domain = vec![0u8; len as usize];
            stream.read_exact(&mut domain).await?;
            let port = stream.read_u16().await?;
            let host = String::from_utf8_lossy(&domain).to_string();
            (host, port)
        }
        ATYP_IPV6 => {
            let mut addr = [0u8; 16];
            stream.read_exact(&mut addr).await?;
            let port = stream.read_u16().await?;
            let ip = Ipv6Addr::new(
                u16::from_be_bytes([addr[0], addr[1]]),
                u16::from_be_bytes([addr[2], addr[3]]),
                u16::from_be_bytes([addr[4], addr[5]]),
                u16::from_be_bytes([addr[6], addr[7]]),
                u16::from_be_bytes([addr[8], addr[9]]),
                u16::from_be_bytes([addr[10], addr[11]]),
                u16::from_be_bytes([addr[12], addr[13]]),
                u16::from_be_bytes([addr[14], addr[15]]),
            );
            (ip.to_string(), port)
        }
        _ => {
            send_reply(&mut stream, Reply::AddressNotSupported, None).await?;
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported address type",
            ));
        }
    };

    info!("SOCKS5 CONNECT {}:{}", host, port);

    // Call handler to establish connection
    let request = ConnectRequest { host, port };
    match handler(request).await {
        Ok(proxy_stream) => {
            // Send success reply
            send_reply(&mut stream, Reply::Success, Some(proxy_stream.local_addr)).await?;

            // Start proxying
            proxy_stream.proxy(stream).await?;
            Ok(())
        }
        Err(e) => {
            warn!("Failed to establish tunnel: {}", e);
            send_reply(&mut stream, Reply::HostUnreachable, None).await?;
            Err(e)
        }
    }
}

/// Send SOCKS5 reply
async fn send_reply(
    stream: &mut TcpStream,
    reply: Reply,
    bound_addr: Option<SocketAddr>,
) -> io::Result<()> {
    let mut buf = BytesMut::with_capacity(10);
    buf.put_u8(VERSION);
    buf.put_u8(reply as u8);
    buf.put_u8(0); // Reserved

    if let Some(addr) = bound_addr {
        match addr.ip() {
            IpAddr::V4(ip) => {
                buf.put_u8(ATYP_IPV4);
                buf.extend_from_slice(&ip.octets());
            }
            IpAddr::V6(ip) => {
                buf.put_u8(ATYP_IPV6);
                buf.extend_from_slice(&ip.octets());
            }
        }
        buf.put_u16(addr.port());
    } else {
        // Bind address 0.0.0.0:0
        buf.put_u8(ATYP_IPV4);
        buf.put_u32(0);
        buf.put_u16(0);
    }

    stream.write_all(&buf).await?;
    stream.flush().await?;
    Ok(())
}

/// A stream that can be used for proxying
pub struct ProxyStream {
    local_addr: SocketAddr,
    stream: TcpStream,
}

impl ProxyStream {
    /// Create a new proxy stream
    pub fn new(local_addr: SocketAddr, stream: TcpStream) -> Self {
        Self { local_addr, stream }
    }

    /// Get the local address
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Start bidirectional proxying between the SOCKS5 client and the tunneled connection
    pub async fn proxy(mut self, mut client: TcpStream) -> io::Result<()> {
        let (mut client_read, mut client_write) = client.split();
        let (mut stream_read, mut stream_write) = self.stream.split();

        // Bidirectional copy
        let client_to_stream = tokio::io::copy(&mut client_read, &mut stream_write);
        let stream_to_client = tokio::io::copy(&mut stream_read, &mut client_write);

        tokio::select! {
            result = client_to_stream => {
                debug!("Client to stream finished: {:?}", result);
            }
            result = stream_to_client => {
                debug!("Stream to client finished: {:?}", result);
            }
        }

        Ok(())
    }
}

/// Request to open a tunnel connection
#[derive(Debug)]
pub struct TunnelRequest {
    pub host: String,
    pub port: u16,
    pub response_tx: tokio::sync::oneshot::Sender<io::Result<TunnelStream>>,
}

/// A stream through the tunnel
pub struct TunnelStream {
    pub reader: tokio::sync::mpsc::Receiver<Vec<u8>>,
    pub writer: tokio::sync::mpsc::Sender<Vec<u8>>,
}

impl std::fmt::Debug for TunnelStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TunnelStream").finish()
    }
}
