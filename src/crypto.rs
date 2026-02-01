//! Cryptography utilities for SMTP Tunnel

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Type alias for HMAC-SHA256
type HmacSha256 = Hmac<Sha256>;

/// Authentication token manager
pub struct AuthToken;

impl AuthToken {
    /// Generate an authentication token
    /// Format: base64(username:timestamp:hmac)
    pub fn generate(secret: &str, username: &str, timestamp: u64) -> String {
        let message = format!("smtp-tunnel-auth:{username}:{timestamp}");
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        let result = mac.finalize();
        let hmac_bytes = result.into_bytes();
        let hmac_b64 = BASE64.encode(hmac_bytes);

        let token = format!("{username}:{timestamp}:{hmac_b64}");
        BASE64.encode(token.as_bytes())
    }

    /// Generate with current timestamp
    pub fn generate_now(secret: &str, username: &str) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self::generate(secret, username, timestamp)
    }

    /// Verify an authentication token
    /// Returns (valid, username) if valid
    pub fn verify(token_b64: &str, secret: &str, max_age_secs: u64) -> (bool, Option<String>) {
        let decoded = match BASE64.decode(token_b64.as_bytes()) {
            Ok(d) => match String::from_utf8(d) {
                Ok(s) => s,
                Err(_) => return (false, None),
            },
            Err(_) => return (false, None),
        };

        let parts: Vec<&str> = decoded.split(':').collect();
        if parts.len() != 3 {
            return (false, None);
        }

        let username = parts[0];
        let timestamp: u64 = match parts[1].parse() {
            Ok(t) => t,
            Err(_) => return (false, None),
        };

        // Check timestamp freshness
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now.saturating_sub(timestamp) > max_age_secs {
            return (false, None);
        }

        // Verify HMAC
        let expected = Self::generate(secret, username, timestamp);
        let valid = expected.len() == token_b64.len()
            && expected
                .as_bytes()
                .iter()
                .zip(token_b64.as_bytes().iter())
                .all(|(a, b)| a == b);
        if valid {
            (true, Some(username.to_string()))
        } else {
            (false, None)
        }
    }

    /// Verify against multiple users
    pub fn verify_multi_user(
        token_b64: &str,
        users: &HashMap<String, UserSecret>,
        max_age_secs: u64,
    ) -> (bool, Option<String>) {
        let decoded = match BASE64.decode(token_b64.as_bytes()) {
            Ok(d) => match String::from_utf8(d) {
                Ok(s) => s,
                Err(_) => return (false, None),
            },
            Err(_) => return (false, None),
        };

        let parts: Vec<&str> = decoded.split(':').collect();
        if parts.len() != 3 {
            return (false, None);
        }

        let username = parts[0];
        let timestamp: u64 = match parts[1].parse() {
            Ok(t) => t,
            Err(_) => return (false, None),
        };

        // Check timestamp freshness first
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now.saturating_sub(timestamp) > max_age_secs {
            return (false, None);
        }

        // Look up user
        let user = match users.get(username) {
            Some(u) => u,
            None => return (false, None),
        };

        // Verify HMAC
        let expected = Self::generate(&user.secret, username, timestamp);
        let valid = expected.len() == token_b64.len()
            && expected
                .as_bytes()
                .iter()
                .zip(token_b64.as_bytes().iter())
                .all(|(a, b)| a == b);
        if valid {
            (true, Some(username.to_string()))
        } else {
            (false, None)
        }
    }
}

/// User secret for authentication
#[derive(Debug, Clone)]
pub struct UserSecret {
    pub secret: String,
}

impl UserSecret {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
        }
    }
}

/// Generate a random secret
pub fn generate_secret() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";
    let mut rng = rand::thread_rng();
    let secret: String = (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    secret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generate_verify() {
        let secret = "test-secret-123";
        let username = "alice";
        // Use a recent timestamp (within last 5 minutes)
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let token = AuthToken::generate(secret, username, timestamp);
        let (valid, user) = AuthToken::verify(&token, secret, 300);

        assert!(valid);
        assert_eq!(user, Some(username.to_string()));
    }

    #[test]
    fn test_token_wrong_secret() {
        let token = AuthToken::generate("correct-secret", "alice", 1234567890);
        let (valid, _) = AuthToken::verify(&token, "wrong-secret", 300);

        assert!(!valid);
    }

    #[test]
    fn test_token_expired() {
        let old_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 1000;

        let token = AuthToken::generate("secret", "alice", old_timestamp);
        let (valid, _) = AuthToken::verify(&token, "secret", 300);

        assert!(!valid);
    }
}
