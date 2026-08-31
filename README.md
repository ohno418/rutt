# rutt

A minimalist TUI email client.

## Features

- IMAP over TLS (IMAPS).
- Threaded index, newest first, with unread messages highlighted.
- Pager showing the plain-text part (or raw HTML as a fallback).
- Vim-style navigation.

Not yet implemented: deleting, sending, multiple accounts or mailboxes.

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
