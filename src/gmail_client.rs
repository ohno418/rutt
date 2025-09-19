use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use imap::Session;
use mailparse::parse_mail;
use native_tls::{TlsConnector, TlsStream};
use std::net::TcpStream;

#[derive(Debug, Clone)]
pub struct Email {
    pub _uid: u32,
    pub subject: String,
    pub from: String,
    pub date: DateTime<Local>,
    pub is_read: bool,
}

pub struct GmailClient {
    session: Session<TlsStream<TcpStream>>,
}

impl GmailClient {
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
                            name.to_string()
                        } else {
                            format!("{}@{}", mailbox, host)
                        }
                    })
                    .unwrap_or_else(|| "(unknown)".to_string());

                let date = if let Some(header) = msg.header() {
                    parse_date_from_header(header).unwrap_or_else(|| Local::now())
                } else {
                    Local::now()
                };

                emails.push(Email {
                    _uid,
                    subject,
                    from,
                    date,
                    is_read,
                });
            }
        }

        emails.sort_by(|a, b| b.date.cmp(&a.date));

        Ok(emails)
    }

    pub fn _logout(mut self) -> Result<()> {
        self.session.logout().context("Failed to logout")?;
        Ok(())
    }
}

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
            date,
            is_read: false,
        };

        assert_eq!(email._uid, 123);
        assert_eq!(email.subject, "Test Subject");
        assert_eq!(email.from, "test@example.com");
        assert!(!email.is_read);
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
            date: Local::now(),
            is_read: true,
        };

        let cloned = email.clone();
        assert_eq!(cloned._uid, email._uid);
        assert_eq!(cloned.subject, email.subject);
        assert_eq!(cloned.from, email.from);
        assert_eq!(cloned.is_read, email.is_read);
    }
}
