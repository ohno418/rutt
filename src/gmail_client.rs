//! Gmail IMAP client implementation with SSL/TLS support.
//!
//! Provides secure connection to Gmail's IMAP server, email fetching, and
//! message parsing functionality.

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use imap::Session;
use mailparse::parse_mail;
use native_tls::{TlsConnector, TlsStream};
use std::{fmt, net::TcpStream};

/// Represents an email message with metadata.
#[derive(Debug, Clone)]
pub struct Email {
    /// Unique identifier for the email in the mailbox.
    pub _uid: u32,
    /// Subject line of the email.
    pub subject: String,
    /// Sender's name or email address.
    pub from: NameAddr,
    /// Primary recipients.
    pub to: Vec<NameAddr>,
    /// Carbon copy recipients.
    pub cc: Vec<NameAddr>,
    /// Blind carbon copy recipients.
    pub bcc: Vec<NameAddr>,
    /// Date and time the email was sent.
    pub date: DateTime<Local>,
    /// Whether the email has been read.
    pub is_read: bool,
    /// Email body content (lazily loaded).
    pub body: Option<String>,
}

/// Represents an email address with an optional display name.
///
/// This structure can represent email addresses in various formats:
/// - Name and email: "John Doe <john@example.com>"
/// - Email only: "john@example.com"
/// - Name only: "John Doe" (less common)
#[derive(Debug, Clone)]
pub struct NameAddr {
    pub name: Option<String>,
    pub email: Option<String>,
}

impl fmt::Display for NameAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self {
                name: Some(name),
                email: Some(email),
            } => write!(f, "{} <{}>", name, email),
            Self {
                name: None,
                email: Some(email),
            } => write!(f, "{}", email),
            Self {
                name: Some(name),
                email: None,
            } => write!(f, "{} <(unknown)>", name),
            Self {
                name: None,
                email: None,
            } => write!(f, "(unknown)"),
        }
    }
}

impl NameAddr {
    /// Returns the display name if available, otherwise the email address.
    ///
    /// This method prioritizes the display name over the email address for
    /// user-friendly display purposes. Returns `None` if both fields are empty.
    pub fn name_or_addr(&self) -> Option<&str> {
        match self {
            Self {
                name: Some(name),
                email: _,
            } => Some(name),
            Self {
                name: None,
                email: Some(email),
            } => Some(email),
            Self {
                name: None,
                email: None,
            } => None,
        }
    }
}

/// Gmail IMAP client for secure email access.
#[derive(Debug)]
pub struct GmailClient {
    session: Session<TlsStream<TcpStream>>,
}

impl GmailClient {
    /// Establishes a secure connection to Gmail's IMAP server.
    pub fn connect(username: &str, password: &str) -> Result<Self> {
        let tls = TlsConnector::builder()
            .build()
            .context("Failed to create TLS connector")?;

        let client = imap::connect(("imap.gmail.com", 993), "imap.gmail.com", &tls)
            .context("Failed to connect to Gmail IMAP")?;

        let session = client
            .login(username, password)
            .map_err(|(e, _)| e)
            .context("Failed to login to Gmail")?;

        Ok(GmailClient { session })
    }

    /// Fetches the most recent emails from the INBOX.
    pub fn fetch_emails(&mut self, limit: u32) -> Result<Vec<Email>> {
        // Get the number of messages in the mailbox
        let mailbox = self
            .session
            .examine("INBOX")
            .context("Failed to examine INBOX")?;

        let total = mailbox.exists;
        if total == 0 {
            return Ok(Vec::new());
        }

        // Calculate the sequence range for the most recent messages
        let start = if total > limit { total - limit + 1 } else { 1 };

        let sequence_set = format!("{}:{}", start, total);

        let messages = self
            .session
            .fetch(&sequence_set, "(UID FLAGS ENVELOPE RFC822.HEADER)")
            .context("Failed to fetch messages")?;

        let mut emails = Vec::new();

        for msg in messages.iter() {
            let _uid = msg.uid.unwrap_or(0);

            let is_read = msg.flags().iter().any(|f| f == &imap::types::Flag::Seen);

            if let Some(envelope) = msg.envelope() {
                let date = if let Some(header) = msg.header() {
                    parse_date_from_header(header).unwrap_or_else(|| Local::now())
                } else {
                    Local::now()
                };

                let subject = envelope
                    .subject
                    .as_ref()
                    .and_then(|s| std::str::from_utf8(s).ok())
                    .unwrap_or("(no subject)")
                    .to_string();

                let from = envelope
                    .from
                    .as_ref()
                    .and_then(|addrs| addrs.first())
                    .map(|addr| {
                        let name = addr
                            .name
                            .as_ref()
                            .and_then(|n| std::str::from_utf8(n).ok())
                            .unwrap_or("");
                        let mailbox = addr
                            .mailbox
                            .as_ref()
                            .and_then(|m| std::str::from_utf8(m).ok())
                            .unwrap_or("");
                        let host = addr
                            .host
                            .as_ref()
                            .and_then(|h| std::str::from_utf8(h).ok())
                            .unwrap_or("");
                        let name = if !name.is_empty() {
                            Some(name.to_string())
                        } else {
                            None
                        };
                        let email = Some(format!("{}@{}", mailbox, host));
                        NameAddr { name, email }
                    })
                    .unwrap_or_else(|| NameAddr {
                        name: None,
                        email: None,
                    });

                let to = envelope
                    .to
                    .as_ref()
                    .map(|addrs| {
                        addrs
                            .iter()
                            .map(|addr| {
                                let name = addr
                                    .name
                                    .as_ref()
                                    .and_then(|n| std::str::from_utf8(n).ok())
                                    .filter(|s| !s.is_empty())
                                    .map(|s| s.to_string());
                                let mailbox = addr
                                    .mailbox
                                    .as_ref()
                                    .and_then(|m| std::str::from_utf8(m).ok())
                                    .unwrap_or("");
                                let host = addr
                                    .host
                                    .as_ref()
                                    .and_then(|h| std::str::from_utf8(h).ok())
                                    .unwrap_or("");
                                let email = if !mailbox.is_empty() && !host.is_empty() {
                                    Some(format!("{}@{}", mailbox, host))
                                } else {
                                    None
                                };
                                NameAddr { name, email }
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(Vec::new);

                let cc = envelope
                    .cc
                    .as_ref()
                    .map(|addrs| {
                        addrs
                            .iter()
                            .map(|addr| {
                                let name = addr
                                    .name
                                    .as_ref()
                                    .and_then(|n| std::str::from_utf8(n).ok())
                                    .filter(|s| !s.is_empty())
                                    .map(|s| s.to_string());
                                let mailbox = addr
                                    .mailbox
                                    .as_ref()
                                    .and_then(|m| std::str::from_utf8(m).ok())
                                    .unwrap_or("");
                                let host = addr
                                    .host
                                    .as_ref()
                                    .and_then(|h| std::str::from_utf8(h).ok())
                                    .unwrap_or("");
                                let email = if !mailbox.is_empty() && !host.is_empty() {
                                    Some(format!("{}@{}", mailbox, host))
                                } else {
                                    None
                                };
                                NameAddr { name, email }
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(Vec::new);

                let bcc = envelope
                    .bcc
                    .as_ref()
                    .map(|addrs| {
                        addrs
                            .iter()
                            .map(|addr| {
                                let name = addr
                                    .name
                                    .as_ref()
                                    .and_then(|n| std::str::from_utf8(n).ok())
                                    .filter(|s| !s.is_empty())
                                    .map(|s| s.to_string());
                                let mailbox = addr
                                    .mailbox
                                    .as_ref()
                                    .and_then(|m| std::str::from_utf8(m).ok())
                                    .unwrap_or("");
                                let host = addr
                                    .host
                                    .as_ref()
                                    .and_then(|h| std::str::from_utf8(h).ok())
                                    .unwrap_or("");
                                let email = if !mailbox.is_empty() && !host.is_empty() {
                                    Some(format!("{}@{}", mailbox, host))
                                } else {
                                    None
                                };
                                NameAddr { name, email }
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(Vec::new);

                emails.push(Email {
                    _uid,
                    subject,
                    from,
                    to,
                    cc,
                    bcc,
                    date,
                    is_read,
                    body: None,
                });
            }
        }

        emails.sort_by(|a, b| b.date.cmp(&a.date));

        Ok(emails)
    }

    /// Fetches the body of a specific email by its UID.
    pub fn fetch_email_body(&mut self, uid: u32) -> Result<String> {
        self.session
            .select("INBOX")
            .context("Failed to select INBOX")?;

        let uid_set = format!("{}", uid);
        let messages = self
            .session
            .uid_fetch(&uid_set, "BODY[TEXT]")
            .context("Failed to fetch message body")?;

        if let Some(msg) = messages.iter().next() {
            if let Some(body) = msg.text() {
                let body_str = std::str::from_utf8(body)
                    .unwrap_or("(Unable to decode message body)")
                    .to_string();
                return Ok(body_str);
            }
        }

        Ok("(No body content)".to_string())
    }

    fn _logout(mut self) -> Result<()> {
        self.session.logout().context("Failed to logout")?;
        Ok(())
    }
}

/// Parses date from email header bytes using multiple date formats.
///
/// Attempts to parse RFC2822 format first, then falls back to a common
/// alternative format if that fails.
fn parse_date_from_header(header: &[u8]) -> Option<DateTime<Local>> {
    let mail = parse_mail(header).ok()?;

    for header in mail.headers {
        if header.get_key().eq_ignore_ascii_case("date") {
            let date_str = header.get_value();
            if let Ok(date) = DateTime::parse_from_rfc2822(&date_str) {
                return Some(date.with_timezone(&Local));
            }

            if let Ok(date) =
                chrono::DateTime::parse_from_str(&date_str, "%a, %d %b %Y %H:%M:%S %z")
            {
                return Some(date.with_timezone(&Local));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_date_from_header_rfc2822() {
        let header = b"Date: Wed, 15 Jan 2025 10:30:45 +0000\r\n\r\n";
        let result = parse_date_from_header(header);
        assert!(result.is_some());
        let date = result.unwrap();
        assert_eq!(date.format("%Y/%m/%d").to_string(), "2025/01/15");
    }

    #[test]
    fn test_parse_date_from_header_invalid() {
        let header = b"Date: Invalid Date Format\r\n\r\n";
        let result = parse_date_from_header(header);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_date_from_header_missing() {
        let header = b"Subject: Test Subject\r\n\r\n";
        let result = parse_date_from_header(header);
        assert!(result.is_none());
    }
}
