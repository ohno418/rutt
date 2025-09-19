//! UI rendering functions for email list and detail views.
//!
//! Provides rendering logic for the terminal interface including email lists,
//! headers, footers, and email detail display.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::ui::app::{App, ViewMode};
use crate::utils::format_date;

/// Main UI rendering function that dispatches to appropriate view.
pub(crate) fn ui(f: &mut Frame, app: &App) {
    match app.mode {
        ViewMode::List => render_list(f, app),
        ViewMode::Detail(idx) => render_detail(f, app, idx),
    }
}

/// Renders the email list view with header and footer.
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

/// Renders the email detail view for a specific email.
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
