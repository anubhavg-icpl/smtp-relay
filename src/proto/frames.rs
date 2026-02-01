use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::io;
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder};

/// Protocol version
pub const PROTOCOL_VERSION: u8 = 1;

/// Maximum payload size (64KB)
pub const MAX_PAYLOAD_SIZE: usize = 65535;

/// Frame header size: type(1) + channel_id(2) + length(2)
pub const FRAME_HEADER_SIZE: usize = 5;

/// Frame types for binary protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    /// Tunnel data
    Data = 0x01,
    /// Open new channel
    Connect = 0x02,
    /// Connection successful
    ConnectOk = 0x03,
    /// Connection failed
    ConnectFail = 0x04,
    /// Close channel
    Close = 0x05,
    /// Keepalive
    Keepalive = 0x06,
    /// Keepalive ACK
    KeepaliveAck = 0x07,
}

impl FrameType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::Data),
            0x02 => Some(Self::Connect),
            0x03 => Some(Self::ConnectOk),
            0x04 => Some(Self::ConnectFail),
            0x05 => Some(Self::Close),
            0x06 => Some(Self::Keepalive),
            0x07 => Some(Self::KeepaliveAck),
            _ => None,
        }
    }
}

/// Binary protocol frame
/// Wire format: type(1) + channel_id(2) + length(2) + payload(N)
#[derive(Debug, Clone)]
pub struct Frame {
    pub frame_type: FrameType,
    pub channel_id: u16,
    pub payload: Bytes,
}

impl Frame {
    /// Create a new frame
    pub fn new(frame_type: FrameType, channel_id: u16, payload: impl Into<Bytes>) -> Self {
        Self {
            frame_type,
            channel_id,
            payload: payload.into(),
        }
    }

    /// Create a DATA frame
    pub fn data(channel_id: u16, data: impl Into<Bytes>) -> Self {
        Self::new(FrameType::Data, channel_id, data)
    }

    /// Create a CONNECT frame
    pub fn connect(channel_id: u16, host: &str, port: u16) -> Self {
        let host_bytes = host.as_bytes();
        let mut payload = BytesMut::with_capacity(1 + host_bytes.len() + 2);
        payload.put_u8(host_bytes.len() as u8);
        payload.extend_from_slice(host_bytes);
        payload.put_u16(port);
        Self::new(FrameType::Connect, channel_id, payload.freeze())
    }

    /// Create a CONNECT_OK frame
    pub fn connect_ok(channel_id: u16) -> Self {
        Self::new(FrameType::ConnectOk, channel_id, Bytes::new())
    }

    /// Create a CONNECT_FAIL frame
    pub fn connect_fail(channel_id: u16, reason: &str) -> Self {
        Self::new(FrameType::ConnectFail, channel_id, Bytes::copy_from_slice(reason.as_bytes()))
    }

    /// Create a CLOSE frame
    pub fn close(channel_id: u16) -> Self {
        Self::new(FrameType::Close, channel_id, Bytes::new())
    }

    /// Serialize frame to bytes
    pub fn serialize(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(FRAME_HEADER_SIZE + self.payload.len());
        buf.put_u8(self.frame_type as u8);
        buf.put_u16(self.channel_id);
        buf.put_u16(self.payload.len() as u16);
        buf.extend_from_slice(&self.payload);
        buf.freeze()
    }

    /// Parse a CONNECT payload to extract host and port
    pub fn parse_connect(&self) -> Option<(String, u16)> {
        if self.frame_type != FrameType::Connect {
            return None;
        }
        let mut buf = &self.payload[..];
        if buf.remaining() < 1 {
            return None;
        }
        let host_len = buf.get_u8() as usize;
        if buf.remaining() < host_len + 2 {
            return None;
        }
        let host_bytes = &buf[..host_len];
        let host = String::from_utf8_lossy(host_bytes).to_string();
        buf.advance(host_len);
        let port = buf.get_u16();
        Some((host, port))
    }
}

/// Frame parsing error
#[derive(Debug, Error)]
pub enum FrameError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Invalid frame type: {0}")]
    InvalidType(u8),
    #[error("Payload too large: {0}")]
    PayloadTooLarge(usize),
    #[error("Incomplete frame")]
    Incomplete,
}

/// Tokio codec for encoding/decoding frames
pub struct FrameCodec;

impl Encoder<Frame> for FrameCodec {
    type Error = FrameError;

    fn encode(&mut self, item: Frame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.extend_from_slice(&item.serialize());
        Ok(())
    }
}

impl Decoder for FrameCodec {
    type Item = Frame;
    type Error = FrameError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Need at least header
        if src.len() < FRAME_HEADER_SIZE {
            return Ok(None);
        }

        // Peek at header to get payload length
        let frame_type = src[0];
        let payload_len = u16::from_be_bytes([src[3], src[4]]) as usize;

        // Validate frame type
        let frame_type = FrameType::from_u8(frame_type)
            .ok_or(FrameError::InvalidType(frame_type))?;

        // Check payload size
        if payload_len > MAX_PAYLOAD_SIZE {
            return Err(FrameError::PayloadTooLarge(payload_len));
        }

        // Check if we have complete frame
        let total_len = FRAME_HEADER_SIZE + payload_len;
        if src.len() < total_len {
            // Reserve space for the full frame
            src.reserve(total_len - src.len());
            return Ok(None);
        }

        // Extract frame data
        let mut buf = src.split_to(total_len);
        buf.advance(1); // Skip type
        let channel_id = buf.get_u16();
        buf.advance(2); // Skip length (we already know it)
        let payload = buf.freeze();

        Ok(Some(Frame {
            frame_type,
            channel_id,
            payload,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_serialize_parse() {
        let frame = Frame::connect(42, "example.com", 443);
        let serialized = frame.serialize();
        
        let mut codec = FrameCodec;
        let mut buf = BytesMut::from(&serialized[..]);
        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        
        assert_eq!(decoded.frame_type, FrameType::Connect);
        assert_eq!(decoded.channel_id, 42);
        let (host, port) = decoded.parse_connect().unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 443);
    }

    #[test]
    fn test_frame_codec_partial() {
        let mut codec = FrameCodec;
        let mut buf = BytesMut::from(&[0x01, 0x00, 0x01, 0x00, 0x05][..]); // Incomplete
        
        assert!(codec.decode(&mut buf).unwrap().is_none());
        
        // Add rest of payload
        buf.extend_from_slice(b"hello");
        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        
        assert_eq!(decoded.frame_type, FrameType::Data);
        assert_eq!(decoded.channel_id, 1);
        assert_eq!(&decoded.payload[..], b"hello");
    }
}
