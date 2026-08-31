# rutt

A minimalist TUI email client like mutt.

## Features

- Fetches message headers from an IMAP server over TLS (IMAPS).
- Threaded index view (like mutt's `sort = threads`), newest thread first,
  replies nested as a tree.
- Each row shows `<date> <time> <sender> <subject>` in aligned columns.
- Unread messages are highlighted. Nothing is ever marked as read.
- Message pager: shows the first `text/plain` part as-is, or the raw HTML
  if the message has no plain-text part.
- Vim-style navigation.

Not yet implemented: marking read / deleting, sending, multiple accounts or
mailboxes, caching, OAuth2.

## Setup

Create `~/.config/rutt/config.toml` (see `config.example.toml`):

```toml
[user]
email = "you@example.com"
password = "your-password-or-app-password"

[imap]
host = "imap.example.com"
port = 993          # optional (default 993)
mailbox = "INBOX"   # optional (default INBOX)
```

The password is stored in plain text, so `chmod 600` the file.
Gmail and iCloud require an app password.

## Usage

```
cargo run
```

Index:

| Key                       | Action       |
|---------------------------|--------------|
| `j` / `k`, `↓` / `↑`      | move         |
| `g` / `G`, `Home` / `End` | top / bottom |
| `Enter`                   | read message |
| `q`, `Ctrl-C`             | quit         |

Pager:

| Key                       | Action              |
|---------------------------|---------------------|
| `j` / `k`, `↓` / `↑`      | scroll one line     |
| `Space` / `b`, `PgDn` / `PgUp` | scroll one page |
| `g` / `G`, `Home` / `End` | top / bottom        |
| `q`, `i`, `Esc`           | back to index       |
| `Ctrl-C`                  | quit                |

## Layout

| File            | Role                                          |
|-----------------|-----------------------------------------------|
| `src/config.rs` | Load `config.toml`                            |
| `src/mail.rs`   | IMAP session, header parsing, body rendering  |
| `src/thread.rs` | Thread construction and tree flattening       |
| `src/ui.rs`     | Index view, pager, and key handling (ratatui) |
