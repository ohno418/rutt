mod config;
mod gmail_client;

use anyhow::{Context, Result};
use chrono::Local;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use std::io;

use config::Config;
use gmail_client::{Email, GmailClient};

enum ViewMode {
    List,
    Detail(usize),
}

struct App {
    emails: Vec<Email>,
    list_state: ListState,
    _client: GmailClient,
    mode: ViewMode,
}

impl App {
    fn new(client: GmailClient, emails: Vec<Email>) -> Self {
        let mut list_state = ListState::default();
        if !emails.is_empty() {
            list_state.select(Some(0));
        }

        App {
            emails,
            list_state,
            _client: client,
            mode: ViewMode::List,
        }
    }

    fn next(&mut self) {
        if self.emails.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.emails.len() - 1 {
                    self.emails.len() - 1
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.emails.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn view_email(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected < self.emails.len() {
                self.mode = ViewMode::Detail(selected);
            }
        }
    }

    fn back_to_list(&mut self) {
        self.mode = ViewMode::List;
    }
}

pub fn format_date(date: &chrono::DateTime<Local>) -> String {
    date.format("%Y/%m/%d %H:%M").to_string()
}

fn ui(f: &mut Frame, app: &App) {
    match app.mode {
        ViewMode::List => render_list(f, app),
        ViewMode::Detail(idx) => render_detail(f, app, idx),
    }
}

fn render_list(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(f.area());

    // Header
    let header = Paragraph::new(Text::from(vec![Line::from(vec![
        Span::styled(
            "Gmail IMAP Client",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" - "),
        Span::styled(
            format!("{} emails", app.emails.len()),
            Style::default().fg(Color::Gray),
        ),
    ])]))
    .block(Block::default().borders(Borders::BOTTOM))
    .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    // Email list
    let items: Vec<ListItem> = app
        .emails
        .iter()
        .map(|email| {
            let status = if email.is_read {
                Span::styled("R", Style::default().fg(Color::Gray))
            } else {
                Span::styled(
                    "N",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            };

            let date_str = format_date(&email.date);

            let from = if email.from.len() > 25 {
                format!("{}...", &email.from[..22])
            } else {
                format!("{:<25}", email.from)
            };

            let subject = if email.subject.len() > 100 {
                format!("{}...", &email.subject[..97])
            } else {
                email.subject.clone()
            };

            let content = vec![Line::from(vec![
                Span::raw("["),
                status,
                Span::raw("] "),
                Span::styled(
                    format!("{:>10}", date_str),
                    Style::default().fg(Color::Blue),
                ),
                Span::raw(" │ "),
                Span::styled(from, Style::default().fg(Color::Green)),
                Span::raw(" │ "),
                Span::raw(subject),
            ])];

            ListItem::new(content)
        })
        .collect();

    let emails = List::new(items)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(emails, chunks[1], &mut app.list_state.clone());

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::raw("j/^n/↓"),
        Span::styled(":down", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::raw("k/^p/↑"),
        Span::styled(":up", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::raw("Enter"),
        Span::styled(":view", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::raw("q"),
        Span::styled(":quit", Style::default().fg(Color::DarkGray)),
    ]))
    .style(Style::default().fg(Color::White))
    .alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}

fn render_detail(f: &mut Frame, app: &App, idx: usize) {
    if idx >= app.emails.len() {
        return;
    }

    let email = &app.emails[idx];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(f.area());

    // Header
    let header = Paragraph::new(Text::from(vec![Line::from(vec![Span::styled(
        "Email Details",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )])]))
    .block(Block::default().borders(Borders::BOTTOM))
    .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    // Email content
    let status = if email.is_read { "Read" } else { "Unread" };
    let status_color = if email.is_read {
        Color::Gray
    } else {
        Color::Yellow
    };

    let content = vec![
        Line::from(vec![
            Span::styled(
                "From: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&email.from),
        ]),
        Line::default(),
        Line::from(vec![
            Span::styled(
                "Subject: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&email.subject),
        ]),
        Line::default(),
        Line::from(vec![
            Span::styled(
                "Date: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(email.date.format("%Y/%m/%d %H:%M").to_string()),
        ]),
        Line::default(),
        Line::from(vec![
            Span::styled(
                "Status: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(status, Style::default().fg(status_color)),
        ]),
    ];

    let details = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(details, chunks[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::raw("ESC/Backspace"),
        Span::styled(":back", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::raw("q"),
        Span::styled(":quit", Style::default().fg(Color::DarkGray)),
    ]))
    .style(Style::default().fg(Color::White))
    .alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match app.mode {
                    ViewMode::List => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('j') | KeyCode::Down => app.next(),
                        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.next()
                        }
                        KeyCode::Char('k') | KeyCode::Up => app.previous(),
                        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.previous()
                        }
                        KeyCode::Enter => app.view_email(),
                        _ => {}
                    },
                    ViewMode::Detail(_) => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Esc | KeyCode::Backspace => app.back_to_list(),
                        _ => {}
                    },
                }
            }
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_date_today() {
        let now = Local::now();
        let formatted = format_date(&now);
        assert!(formatted.contains('/'));
        assert!(formatted.contains(':'));
        assert_eq!(formatted.len(), 16); // YYYY/MM/DD HH:MM
    }

    #[test]
    fn test_format_date_this_week() {
        let date = Local::now() - chrono::Duration::days(3);
        let formatted = format_date(&date);
        assert!(formatted.contains('/'));
        assert!(formatted.contains(':'));
        assert_eq!(formatted.len(), 16); // YYYY/MM/DD HH:MM
    }

    #[test]
    fn test_format_date_older() {
        let date = Local::now() - chrono::Duration::days(30);
        let formatted = format_date(&date);
        assert!(formatted.contains('/'));
        assert!(formatted.contains(':'));
        assert_eq!(formatted.len(), 16); // YYYY/MM/DD HH:MM
    }

    #[test]
    fn test_app_navigation() {
        let emails = vec![
            Email {
                _uid: 1,
                subject: "Email 1".to_string(),
                from: "test1@test.com".to_string(),
                date: Local::now(),
                is_read: false,
            },
            Email {
                _uid: 2,
                subject: "Email 2".to_string(),
                from: "test2@test.com".to_string(),
                date: Local::now(),
                is_read: true,
            },
        ];

        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let mut app = App::new(client.unwrap(), emails);

        assert_eq!(app.list_state.selected(), Some(0));
        app.next();
        assert_eq!(app.list_state.selected(), Some(1));
        app.next();
        assert_eq!(app.list_state.selected(), Some(1)); // Stays at bottom
        app.previous();
        assert_eq!(app.list_state.selected(), Some(0)); // Stays at top
        app.previous();
        assert_eq!(app.list_state.selected(), Some(0));
    }

    #[test]
    fn test_app_initialization() {
        let emails = vec![];
        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let app = App::new(client.unwrap(), emails.clone());
        assert_eq!(app.emails.len(), 0);
        assert_eq!(app.list_state.selected(), None);
    }

    #[test]
    fn test_view_mode_transitions() {
        let emails = vec![Email {
            _uid: 1,
            subject: "Test".to_string(),
            from: "test@test.com".to_string(),
            date: Local::now(),
            is_read: false,
        }];

        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let mut app = App::new(client.unwrap(), emails);

        // Start in list mode
        assert!(matches!(app.mode, ViewMode::List));

        // View email
        app.view_email();
        assert!(matches!(app.mode, ViewMode::Detail(0)));

        // Go back to list
        app.back_to_list();
        assert!(matches!(app.mode, ViewMode::List));
    }
}
