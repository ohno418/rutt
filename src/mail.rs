//! Fetching message headers over IMAP and parsing them into [`Message`]s.

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use mailparse::{MailHeaderMap, addrparse, dateparse, parse_headers};

use crate::config::Config;

/// Header summary of a single message, enough for the index view and threading.
#[derive(Debug, Clone)]
pub struct Message {
    /// IMAP UID within the mailbox.
    #[allow(dead_code)]
    pub uid: u32,
    /// `Date` header converted to local time, if parseable.
    pub date: Option<DateTime<Local>>,
    /// Display name of the `From` header, falling back to the address.
    pub sender: String,
    /// `Subject` with whitespace normalized.
    pub subject: String,
    /// `Message-ID` without angle brackets.
    pub message_id: Option<String>,
    /// Ancestors, oldest first (References + In-Reply-To).
    pub references: Vec<String>,
    /// True when the `\Seen` flag is absent.
    pub unread: bool,
}

/// Fetch only the headers needed for the index; `PEEK` keeps `\Seen` untouched.
const FETCH_QUERY: &str =
    "(UID FLAGS BODY.PEEK[HEADER.FIELDS (DATE FROM SUBJECT MESSAGE-ID IN-REPLY-TO REFERENCES)])";

/// Connect over TLS, log in, and fetch header summaries of every message in the configured mailbox.
pub fn fetch_messages(config: &Config) -> Result<Vec<Message>> {
    let client = imap::ClientBuilder::new(&config.imap.host, config.imap.port)
        .connect()
        .with_context(|| {
            format!("failed to connect to {}:{}", config.imap.host, config.imap.port)
        })?;

    let mut session = client
        .login(&config.user.email, &config.user.password)
        .map_err(|(e, _)| e)
        .context("IMAP login failed")?;

    let mailbox = session
        .select(&config.imap.mailbox)
        .with_context(|| format!("failed to select mailbox {}", config.imap.mailbox))?;

    let mut messages = Vec::new();
    if mailbox.exists > 0 {
        let fetches = session
            .fetch("1:*", FETCH_QUERY)
            .context("failed to fetch messages")?;
        for fetch in fetches.iter() {
            if let Some(msg) = parse_fetch(fetch) {
                messages.push(msg);
            }
        }
    }

    let _ = session.logout();
    Ok(messages)
}

/// Convert one IMAP FETCH response into a [`Message`]; `None` if it lacks a UID or headers.
fn parse_fetch(fetch: &imap::types::Fetch) -> Option<Message> {
    let uid = fetch.uid?;
    let unread = !fetch.flags().iter().any(|f| matches!(f, imap::types::Flag::Seen));
    let raw = fetch.header().unwrap_or(b"");
    let (headers, _) = parse_headers(raw).ok()?;

    let date = headers
        .get_first_value("Date")
        .and_then(|d| dateparse(&d).ok())
        .and_then(|ts| DateTime::from_timestamp(ts, 0))
        .map(|dt| dt.with_timezone(&Local));

    let sender = headers
        .get_first_value("From")
        .map(|from| parse_sender(&from))
        .unwrap_or_default();

    let subject = headers
        .get_first_value("Subject")
        .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
        .unwrap_or_default();

    let message_id = headers
        .get_first_value("Message-ID")
        .and_then(|v| extract_ids(&v).into_iter().next());

    let mut references = headers
        .get_first_value("References")
        .map(|v| extract_ids(&v))
        .unwrap_or_default();
    if let Some(irt) = headers
        .get_first_value("In-Reply-To")
        .and_then(|v| extract_ids(&v).into_iter().next())
    {
        if references.last() != Some(&irt) {
            references.retain(|r| r != &irt);
            references.push(irt);
        }
    }

    Some(Message {
        uid,
        date,
        sender,
        subject,
        message_id,
        references,
        unread,
    })
}

/// Display name if present, otherwise the address.
fn parse_sender(from: &str) -> String {
    if let Ok(list) = addrparse(from) {
        if let Some(addr) = list.into_inner().into_iter().next() {
            return match addr {
                mailparse::MailAddr::Single(s) => {
                    s.display_name.filter(|n| !n.trim().is_empty()).unwrap_or(s.addr)
                }
                mailparse::MailAddr::Group(g) => g.group_name,
            };
        }
    }
    from.trim().to_string()
}

/// Extract `<...>` message ids from a header value.
fn extract_ids(value: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = value;
    while let Some(start) = rest.find('<') {
        let after = &rest[start + 1..];
        match after.find('>') {
            Some(end) => {
                ids.push(after[..end].trim().to_string());
                rest = &after[end + 1..];
            }
            None => break,
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_ids() {
        assert_eq!(
            extract_ids("<a@x> <b@y>\r\n <c@z>"),
            vec!["a@x", "b@y", "c@z"]
        );
        assert!(extract_ids("nothing").is_empty());
    }

    #[test]
    fn sender_name_or_addr() {
        assert_eq!(parse_sender("Linus Torvalds <torvalds@linux-foundation.org>"), "Linus Torvalds");
        assert_eq!(parse_sender("torvalds@linux-foundation.org"), "torvalds@linux-foundation.org");
    }
}
