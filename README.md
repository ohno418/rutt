# rutt

A minimalist TUI email client like mutt.

## Features

- Fetches message headers from an IMAP server over TLS (IMAPS).
- Threaded index view (like mutt's `sort = threads`), newest thread first,
  replies nested as a tree.
- Each row shows `<date> <time> <sender> <subject>` in aligned columns.
- Unread messages are highlighted. Fetching never marks messages as read.
- Vim-style navigation.

Not yet implemented: viewing message bodies, marking read / deleting,
sending, multiple accounts or mailboxes, caching, OAuth2.

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

| Key                | Action       |
|--------------------|--------------|
| `j` / `k`, `↓` / `↑` | move         |
| `g` / `G`, `Home` / `End` | top / bottom |
| `q`, `Ctrl-C`      | quit         |

## Layout

| File            | Role                                          |
|-----------------|-----------------------------------------------|
| `src/config.rs` | Load `config.toml`                            |
| `src/mail.rs`   | IMAP fetch and header parsing                 |
| `src/thread.rs` | Thread construction and tree flattening       |
| `src/ui.rs`     | Index view and key handling (ratatui)         |
