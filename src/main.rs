//! rutt: a minimalist TUI email client.

mod config;
mod mail;
mod thread;
mod ui;

use anyhow::Result;

fn main() -> Result<()> {
    let config = config::load()?;
    eprintln!("Connecting to {}...", config.imap.host);

    let mut client = mail::Client::connect(&config)?;
    let messages = client.fetch_messages()?;
    let rows = thread::build_rows(messages);

    let mut terminal = ratatui::init();
    let result = ui::App::new(rows, config.imap.mailbox.clone()).run(&mut terminal, &mut client);
    ratatui::restore();
    client.logout();
    result
}
