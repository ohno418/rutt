//! rutt: a minimalist mutt-like TUI email client.
//!
//! Startup flow: load config -> fetch headers over IMAP -> build threads -> run the TUI.

mod config;
mod mail;
mod thread;
mod ui;

use anyhow::Result;

fn main() -> Result<()> {
    let config = config::load()?;
    eprintln!("Connecting to {}...", config.imap.host);
    let messages = mail::fetch_messages(&config)?;
    let rows = thread::build_rows(messages);

    let mut terminal = ratatui::init();
    let result = ui::App::new(rows, config.imap.mailbox.clone()).run(&mut terminal);
    ratatui::restore();
    result
}
