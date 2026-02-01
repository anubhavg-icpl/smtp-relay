/// SMTP Protocol Constants and State Machine
use std::fmt;

/// SMTP response codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseCode(pub u16);

impl ResponseCode {
    pub const READY: Self = Self(220);
    pub const CLOSING: Self = Self(221);
    pub const AUTH_SUCCESS: Self = Self(235);
    pub const OK: Self = Self(250);
    pub const OK_CONTINUING: Self = Self(250); // Multi-line responses start with 250-
    pub const START_INPUT: Self = Self(354);
    pub const AUTH_CONTINUE: Self = Self(334);
    pub const TEMP_FAIL: Self = Self(421);
    pub const SYNTAX_ERROR: Self = Self(500);
    pub const COMMAND_UNRECOGNIZED: Self = Self(502);
    pub const BAD_SEQUENCE: Self = Self(503);
    pub const AUTH_REQUIRED: Self = Self(530);
    pub const AUTH_FAILED: Self = Self(535);
    pub const BINARY_MODE: Self = Self(299);
}

impl fmt::Display for ResponseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// SMTP commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Ehlo,
    Helo,
    StartTls,
    Auth,
    Mail,
    Rcpt,
    Data,
    Quit,
    Binary, // Custom command to switch to binary mode
    Unknown,
}

impl Command {
    pub fn parse(s: &str) -> (Self, &str) {
        let s = s.trim();
        let (cmd, rest) = s.split_once(' ').unwrap_or((s, ""));
        let cmd = cmd.to_uppercase();

        let command = match cmd.as_str() {
            "EHLO" => Self::Ehlo,
            "HELO" => Self::Helo,
            "STARTTLS" => Self::StartTls,
            "AUTH" => Self::Auth,
            "MAIL" => Self::Mail,
            "RCPT" => Self::Rcpt,
            "DATA" => Self::Data,
            "QUIT" => Self::Quit,
            "BINARY" => Self::Binary,
            _ => Self::Unknown,
        };

        (command, rest.trim())
    }
}

/// SMTP State Machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Initial,
    Greeted,
    TlsStarted,
    Authenticated,
    BinaryMode,
    Quit,
}

/// SMTP response builder
pub struct Response;

impl Response {
    /// Create a simple response
    pub fn new(code: ResponseCode, message: &str) -> String {
        format!("{} {}\r\n", code, message)
    }

    /// Create a multi-line response (last line has space after code)
    pub fn multi_line(code: ResponseCode, lines: &[&str]) -> String {
        if lines.is_empty() {
            return Self::new(code, "");
        }
        if lines.len() == 1 {
            return Self::new(code, lines[0]);
        }

        let mut result = String::new();
        for (i, line) in lines.iter().enumerate() {
            if i < lines.len() - 1 {
                result.push_str(&format!("{}-{line}\r\n", code));
            } else {
                result.push_str(&format!("{} {line}\r\n", code));
            }
        }
        result
    }

    /// Greeting response
    pub fn greeting(hostname: &str) -> String {
        Self::new(
            ResponseCode::READY,
            &format!("{hostname} ESMTP Postfix (Ubuntu)"),
        )
    }

    /// EHLO response
    pub fn ehlo(hostname: &str, starttls: bool) -> String {
        let mut lines = vec![hostname];
        if starttls {
            lines.push("STARTTLS");
        }
        lines.push("AUTH PLAIN LOGIN");
        lines.push("8BITMIME");
        Self::multi_line(ResponseCode::OK, &lines)
    }

    /// STARTTLS response
    pub fn starttls() -> String {
        Self::new(ResponseCode::READY, "2.0.0 Ready to start TLS")
    }

    /// Auth success
    pub fn auth_success() -> String {
        Self::new(
            ResponseCode::AUTH_SUCCESS,
            "2.7.0 Authentication successful",
        )
    }

    /// Auth failed
    pub fn auth_failed() -> String {
        Self::new(ResponseCode::AUTH_FAILED, "5.7.8 Authentication failed")
    }

    /// Binary mode activated
    pub fn binary_mode() -> String {
        Self::new(ResponseCode::BINARY_MODE, "Binary mode activated")
    }

    /// Goodbye
    pub fn goodbye() -> String {
        Self::new(ResponseCode::CLOSING, "Bye")
    }

    /// Syntax error
    pub fn syntax_error() -> String {
        Self::new(ResponseCode::SYNTAX_ERROR, "Syntax error")
    }

    /// Command not recognized
    pub fn command_unrecognized() -> String {
        Self::new(ResponseCode::COMMAND_UNRECOGNIZED, "Command not recognized")
    }

    /// Bad sequence
    pub fn bad_sequence() -> String {
        Self::new(ResponseCode::BAD_SEQUENCE, "Bad sequence of commands")
    }

    /// Auth required
    pub fn auth_required() -> String {
        Self::new(ResponseCode::AUTH_REQUIRED, "Authentication required")
    }
}

/// Parse an SMTP line, returning (command, arg) or None if empty
pub fn parse_line(line: &str) -> Option<(Command, String)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let (cmd, arg) = Command::parse(line);
    Some((cmd, arg.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_parse() {
        assert_eq!(Command::parse("EHLO client.local").0, Command::Ehlo);
        assert_eq!(Command::parse("ehlo client.local").0, Command::Ehlo);
        assert_eq!(Command::parse("STARTTLS").0, Command::StartTls);
        assert_eq!(Command::parse("AUTH PLAIN token").0, Command::Auth);
        assert_eq!(Command::parse("BINARY").0, Command::Binary);
    }

    #[test]
    fn test_response_greeting() {
        let resp = Response::greeting("mail.example.com");
        assert!(resp.contains("220"));
        assert!(resp.contains("mail.example.com"));
        assert!(resp.contains("Postfix"));
    }

    #[test]
    fn test_response_multiline() {
        let resp = Response::ehlo("mail.example.com", true);
        assert!(resp.contains("250-mail.example.com"));
        assert!(resp.contains("250-STARTTLS"));
        assert!(resp.contains("250 8BITMIME"));
    }
}
