use anyhow::{Context, Result};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

use rutt::{Config, GmailClient, App, run_app};


fn main() -> Result<()> {
    let config = Config::load_default().context("Failed to load config.toml")?;

    println!("Connecting to Gmail IMAP...");
    let mut client = GmailClient::connect(&config.gmail.username, &config.gmail.app_password)
        .context("Failed to connect to Gmail")?;

    println!("Fetching emails...");
    let emails = client.fetch_emails(200).context("Failed to fetch emails")?;

    println!("Found {} emails", emails.len());

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let app = App::new(client, emails);
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}
