# CLAUDE.md - Project Context for rutt

## Project Overview
rutt is a minimal Gmail IMAP client with a mutt-like terminal interface built in Rust.

## Key Features
- Connects to Gmail via IMAP with SSL/TLS
- Fetches and displays emails from INBOX
- Terminal UI with ratatui for email list and details
- Keyboard navigation (j/k, arrow keys, Enter, q)
- Read/unread status indicators

## Architecture
- `src/config.rs` - TOML configuration loading
- `src/gmail_client.rs` - IMAP connection and email fetching
- `src/main.rs` - Terminal UI with ratatui
- `src/lib.rs` - Module exports for testing

## Testing Commands
```bash
cargo test        # Run all tests
cargo build       # Build the project
cargo run         # Run the application
```

## Configuration
Create `config.toml` with Gmail credentials:
```toml
[gmail]
username = "your-email@gmail.com"
app_password = "your-app-password"
```

## Dependencies
- imap 2.4 - IMAP protocol
- native-tls - SSL/TLS connections
- ratatui 0.29 - Terminal UI framework
- crossterm - Terminal manipulation
- chrono - Date/time handling
- mailparse - Email parsing
- anyhow - Error handling
- serde/toml - Configuration

## Known Issues
- IMAP sequence set syntax fixed (was using invalid "*:*-N" format)
- Now uses proper range calculation for fetching recent emails