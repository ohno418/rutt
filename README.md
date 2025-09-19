# rutt

A minimal Gmail IMAP client with a mutt-like terminal interface written in Rust.

## Features

- Connect to Gmail via IMAP with SSL/TLS
- Fetch and display recent emails from your inbox
- Terminal-based user interface with ratatui
- Keyboard navigation similar to mutt
- View email details including sender, subject, date, and read status
- Color-coded read/unread indicators

## Prerequisites

- Rust toolchain (1.70+)
- Gmail account with app-specific password enabled

## Installation

Clone the repository and build:

```bash
git clone https://github.com/yourusername/rutt.git
cd rutt
cargo build --release
```

## Configuration

Create a `config.toml` file in the project root:

```toml
[gmail]
username = "your-email@gmail.com"
app_password = "xxxx-xxxx-xxxx-xxxx"
```

### Getting a Gmail App Password

1. Go to your Google Account settings
2. Navigate to Security → 2-Step Verification
3. Scroll down and click on "App passwords"
4. Generate a new app password for "Mail"
5. Use this 16-character password in your config.toml

## Usage

Run the application:

```bash
cargo run
```

### Keyboard Controls

**List View:**
- `j` / `↓` - Move down
- `k` / `↑` - Move up
- `Enter` - View email details
- `q` - Quit

**Detail View:**
- `ESC` / `Backspace` - Return to list
- `q` - Quit

## Interface

The terminal interface displays:
- **[N]** - New/unread emails
- **[R]** - Read emails
- Date formatting:
  - Today: `HH:MM`
  - This week: `MMM DD`
  - Older: `YYYY-MM-DD`
- Sender (truncated to 25 chars)
- Subject (truncated to 50 chars)

## Development

Run tests:

```bash
cargo test
```

Build documentation:

```bash
cargo doc --open
```

## License

MIT
