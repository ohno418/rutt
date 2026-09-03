//! IMAP access: fetching header summaries for the index and full bodies for the pager.

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use mailparse::{MailHeaderMap, ParsedMail, addrparse, dateparse, parse_headers, parse_mail};

use crate::config::Config;

/// Header summary of a single message, enough for the index view and threading.
#[derive(Debug, Clone)]
pub struct Message {
    /// IMAP UID within the mailbox.
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
    /// `\Answered` flag.
    pub answered: bool,
    /// `\Deleted` flag.
    pub deleted: bool,
    /// `\Flagged` flag.
    pub flagged: bool,
    /// How the message relates to the account owner.
    pub relation: Relation,
}

/// How a message relates to the account owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// The user is not an explicit recipient.
    None,
    /// The user sent the message.
    FromMe,
    /// The user appears in `To`.
    ToMe,
    /// The user appears in `Cc`.
    CcMe,
}

/// Fetches only the headers needed for the index; `PEEK` keeps `\Seen` untouched.
const FETCH_QUERY: &str = "(UID FLAGS BODY.PEEK[HEADER.FIELDS (DATE FROM TO CC SUBJECT MESSAGE-ID IN-REPLY-TO REFERENCES)])";

/// An authenticated IMAP session with the configured mailbox selected.
pub struct Client {
    session: imap::Session<imap::Connection>,
    /// The account owner's address, for [`Relation`] detection.
    email: String,
}

impl Client {
    /// Connects over TLS, logs in, and selects the configured mailbox.
    pub fn connect(config: &Config) -> Result<Self> {
        let client = imap::ClientBuilder::new(&config.imap.host, config.imap.port)
            .connect()
            .with_context(|| {
                format!(
                    "failed to connect to {}:{}",
                    config.imap.host, config.imap.port
                )
            })?;

        let mut session = client
            .login(&config.user.email, &config.user.password)
            .map_err(|(e, _)| e)
            .context("IMAP login failed")?;

        session
            .select(&config.imap.mailbox)
            .with_context(|| format!("failed to select mailbox {}", config.imap.mailbox))?;

        Ok(Self {
            session,
            email: config.user.email.clone(),
        })
    }

    /// Fetches header summaries of every message in the mailbox.
    pub fn fetch_messages(&mut self) -> Result<Vec<Message>> {
        let fetches = match self.session.fetch("1:*", FETCH_QUERY) {
            Ok(f) => f,
            // "1:*" is invalid on an empty mailbox; treat that as no messages.
            Err(imap::Error::Bad(_)) | Err(imap::Error::No(_)) => return Ok(Vec::new()),
            Err(e) => return Err(e).context("failed to fetch messages"),
        };
        Ok(fetches
            .iter()
            .filter_map(|f| parse_fetch(f, &self.email))
            .collect())
    }

    /// Fetches the full message with `uid` and renders it as displayable text.
    /// `PEEK` keeps the `\Seen` flag untouched.
    pub fn fetch_body(&mut self, uid: u32) -> Result<String> {
        let fetches = self
            .session
            .uid_fetch(uid.to_string(), "BODY.PEEK[]")
            .with_context(|| format!("failed to fetch message {uid}"))?;
        let raw = fetches
            .iter()
            .find_map(|f| f.body())
            .with_context(|| format!("server returned no body for message {uid}"))?;
        render_message(raw)
    }

    /// Sets or clears `\Seen` on `uids` (`+FLAGS`/`-FLAGS`); no-op when empty.
    pub fn set_seen(&mut self, uids: &[u32], seen: bool) -> Result<()> {
        if uids.is_empty() {
            return Ok(());
        }

        let set = uids
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let op = if seen { '+' } else { '-' };
        self.session
            .uid_store(&set, format!("{op}FLAGS.SILENT (\\Seen)"))
            .context("failed to sync read status")?;
        Ok(())
    }

    /// Closes the connection politely; errors are ignored.
    pub fn logout(mut self) {
        let _ = self.session.logout();
    }
}

/// Renders a raw RFC 822 message as `headers + blank line + body text`.
///
/// The body is the first `text/plain` part; if there is none, the first
/// `text/html` part is shown raw.
fn render_message(raw: &[u8]) -> Result<String> {
    let mail = parse_mail(raw).context("failed to parse message")?;

    let mut out = String::new();
    for name in ["Date", "From", "To", "Cc", "Subject"] {
        if let Some(v) = mail.headers.get_first_value(name) {
            out.push_str(&format!("{name}: {v}\n"));
        }
    }
    out.push('\n');

    let body = find_part(&mail, "text/plain")
        .or_else(|| find_part(&mail, "text/html"))
        .map(|p| p.get_body())
        .transpose()
        .context("failed to decode body")?
        .unwrap_or_else(|| "[no text part]".to_string());
    out.push_str(&body);
    Ok(out)
}

/// Searches depth-first for the first leaf part whose content type is `mime`.
fn find_part<'a>(part: &'a ParsedMail<'a>, mime: &str) -> Option<&'a ParsedMail<'a>> {
    if part.subparts.is_empty() {
        return (part.ctype.mimetype.eq_ignore_ascii_case(mime)).then_some(part);
    }
    part.subparts.iter().find_map(|p| find_part(p, mime))
}

/// Converts one IMAP FETCH response into a [`Message`]; `None` if it lacks a UID or headers.
/// `me` is the account owner's address, for [`Relation`] detection.
fn parse_fetch(fetch: &imap::types::Fetch, me: &str) -> Option<Message> {
    let uid = fetch.uid?;
    let mut unread = true;
    let (mut answered, mut deleted, mut flagged) = (false, false, false);
    for f in fetch.flags() {
        match f {
            imap::types::Flag::Seen => unread = false,
            imap::types::Flag::Answered => answered = true,
            imap::types::Flag::Deleted => deleted = true,
            imap::types::Flag::Flagged => flagged = true,
            _ => {}
        }
    }
    let raw = fetch.header().unwrap_or(b"");
    let (headers, _) = parse_headers(raw).ok()?;

    let relation = if headers
        .get_first_value("From")
        .is_some_and(|v| contains_addr(&v, me))
    {
        Relation::FromMe
    } else if headers
        .get_first_value("To")
        .is_some_and(|v| contains_addr(&v, me))
    {
        Relation::ToMe
    } else if headers
        .get_first_value("Cc")
        .is_some_and(|v| contains_addr(&v, me))
    {
        Relation::CcMe
    } else {
        Relation::None
    };

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
        && references.last() != Some(&irt)
    {
        references.retain(|r| r != &irt);
        references.push(irt);
    }

    Some(Message {
        uid,
        date,
        sender,
        subject,
        message_id,
        references,
        unread,
        answered,
        deleted,
        flagged,
        relation,
    })
}

/// True when `me` appears among the addresses of `header`.
fn contains_addr(header: &str, me: &str) -> bool {
    let Ok(list) = addrparse(header) else {
        return false;
    };
    list.iter().any(|a| match a {
        mailparse::MailAddr::Single(s) => s.addr.eq_ignore_ascii_case(me),
        mailparse::MailAddr::Group(g) => g.addrs.iter().any(|s| s.addr.eq_ignore_ascii_case(me)),
    })
}

/// Display name if present, otherwise the address.
fn parse_sender(from: &str) -> String {
    if let Ok(list) = addrparse(from)
        && let Some(addr) = list.into_inner().into_iter().next()
    {
        return match addr {
            mailparse::MailAddr::Single(s) => s
                .display_name
                .filter(|n| !n.trim().is_empty())
                .unwrap_or(s.addr),
            mailparse::MailAddr::Group(g) => g.group_name,
        };
    }
    from.trim().to_string()
}

/// Extracts `<...>` message ids from a header value.
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
    fn prefers_plain_over_html() {
        let raw =
            b"From: a@x\r\nSubject: hi\r\nContent-Type: multipart/alternative; boundary=B\r\n\r\n\
--B\r\nContent-Type: text/html\r\n\r\n<b>hi</b>\r\n\
--B\r\nContent-Type: text/plain\r\n\r\nplain hi\r\n--B--\r\n";
        let text = render_message(raw).unwrap();
        assert!(text.starts_with("From: a@x\nSubject: hi\n\n"));
        assert!(text.contains("plain hi"));
        assert!(!text.contains("<b>"));
    }

    #[test]
    fn falls_back_to_raw_html() {
        let raw = b"Subject: h\r\nContent-Type: text/html\r\n\r\n<p>only html</p>\r\n";
        assert!(render_message(raw).unwrap().contains("<p>only html</p>"));
    }

    #[test]
    fn detects_own_address() {
        assert!(contains_addr("Alice <a@x>, Bob <b@y>", "B@Y"));
        assert!(!contains_addr("Alice <a@x>", "b@y"));
    }

    #[test]
    fn sender_name_or_addr() {
        assert_eq!(
            parse_sender("Linus Torvalds <torvalds@linux-foundation.org>"),
            "Linus Torvalds"
        );
        assert_eq!(
            parse_sender("torvalds@linux-foundation.org"),
            "torvalds@linux-foundation.org"
        );
    }
}
