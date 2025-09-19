//! Gmail IMAP client implementation with SSL/TLS support.
//!
//! Provides secure connection to Gmail's IMAP server, email fetching, and
//! message parsing functionality.

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use imap::Session;
use mailparse::parse_mail;
use native_tls::{TlsConnector, TlsStream};
use std::net::TcpStream;

/// Represents an email message with metadata.
#[derive(Debug, Clone)]
pub struct Email {
    /// Unique identifier for the email in the mailbox.
    pub _uid: u32,
    /// Subject line of the email.
    pub subject: String,
    /// Sender's name or email address.
    pub from: String,
    /// Primary recipients.
    pub to: Option<String>,
    /// Carbon copy recipients.
    pub cc: Option<String>,
    /// Blind carbon copy recipients.
    pub bcc: Option<String>,
    /// Date and time the email was sent.
    pub date: DateTime<Local>,
    /// Whether the email has been read.
    pub is_read: bool,
    /// Email body content (lazily loaded).
    pub body: Option<String>,
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
        self.session
            .select("INBOX")
            .context("Failed to select INBOX")?;

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

                        if !name.is_empty() {
                            format!("{} <{}@{}>", name, mailbox, host)
                        } else {
                            format!("{}@{}", mailbox, host)
                        }
                    })
                    .unwrap_or_else(|| "(unknown)".to_string());

                let cc = envelope.cc.as_ref().map(|addrs| {
                    addrs
                        .iter()
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

                            if !name.is_empty() {
                                format!("{} <{}@{}>", name, mailbox, host)
                            } else {
                                format!("{}@{}", mailbox, host)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                });

                let bcc = envelope.bcc.as_ref().map(|addrs| {
                    addrs
                        .iter()
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

                            if !name.is_empty() {
                                format!("{} <{}@{}>", name, mailbox, host)
                            } else {
                                format!("{}@{}", mailbox, host)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                });

                let to = envelope.to.as_ref().map(|addrs| {
                    addrs
                        .iter()
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

                            if !name.is_empty() {
                                format!("{} <{}@{}>", name, mailbox, host)
                            } else {
                                format!("{}@{}", mailbox, host)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                });

                let date = if let Some(header) = msg.header() {
                    parse_date_from_header(header).unwrap_or_else(|| Local::now())
                } else {
                    Local::now()
                };

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
    fn test_email_struct() {
        let date = Local::now();
        let email = Email {
            _uid: 123,
            subject: "Test Subject".to_string(),
            from: "test@example.com".to_string(),
            to: Some("to@example.com".to_string()),
            cc: Some("cc@example.com".to_string()),
            bcc: None,
            date,
            is_read: false,
            body: None,
        };

        assert_eq!(email._uid, 123);
        assert_eq!(email.subject, "Test Subject");
        assert_eq!(email.from, "test@example.com");
        assert_eq!(email.to, Some("to@example.com".to_string()));
        assert_eq!(email.cc, Some("cc@example.com".to_string()));
        assert_eq!(email.bcc, None);
        assert!(!email.is_read);
        assert!(email.body.is_none());
    }

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

    #[test]
    fn test_email_clone() {
        let email = Email {
            _uid: 456,
            subject: "Clone Test".to_string(),
            from: "clone@test.com".to_string(),
            to: Some("to@test.com".to_string()),
            cc: Some("cc@test.com".to_string()),
            bcc: Some("bcc@test.com".to_string()),
            date: Local::now(),
            is_read: true,
            body: Some("Test body".to_string()),
        };

        let cloned = email.clone();
        assert_eq!(cloned._uid, email._uid);
        assert_eq!(cloned.subject, email.subject);
        assert_eq!(cloned.from, email.from);
        assert_eq!(cloned.to, email.to);
        assert_eq!(cloned.cc, email.cc);
        assert_eq!(cloned.bcc, email.bcc);
        assert_eq!(cloned.is_read, email.is_read);
        assert_eq!(cloned.body, email.body);
    }
}
