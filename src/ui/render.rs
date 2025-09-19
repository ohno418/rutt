//! UI rendering functions for email list and detail views.
//!
//! Provides rendering logic for the terminal interface including email lists,
//! headers, footers, and email detail display.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::ui::app::{App, ViewMode};
use crate::utils::format_date;

/// Main UI rendering function that dispatches to appropriate view.
pub(crate) fn ui(f: &mut Frame, app: &mut App) {
    match app.mode {
        ViewMode::List => render_list(f, app),
        ViewMode::Detail(idx) => render_detail(f, app, idx),
    }
}

/// Renders the email list view with header and footer.
fn render_list(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
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

    // Update visible items count based on list area height.
    app.set_visible_items(chunks[1].height as usize);

    // Email list - only show items in the visible window.
    let visible_emails = app
        .emails
        .iter()
        .skip(app.scroll_offset)
        .take(app.visible_items);

    let items: Vec<ListItem> = visible_emails
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

            let subject_span = if email.is_read {
                Span::raw(subject)
            } else {
                Span::styled(subject, Style::default().fg(Color::Yellow))
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
                subject_span,
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

    // Create a temporary list state for rendering with relative positioning.
    let mut render_state = ListState::default();
    if let Some(selected) = app.list_state.selected() {
        if selected >= app.scroll_offset && selected < app.scroll_offset + app.visible_items {
            render_state.select(Some(selected - app.scroll_offset));
        }
    }

    f.render_stateful_widget(emails, chunks[1], &mut render_state);

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
        Span::raw("q/Esc"),
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
            Constraint::Length(2),
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

    let mut content = vec![
        Line::from(vec![
            Span::styled(
                "Date: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(email.date.format("%Y/%m/%d %H:%M").to_string()),
        ]),
        Line::from(vec![
            Span::styled(
                "From: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&email.from),
        ]),
    ];

    // Add To field if present
    if let Some(to) = &email.to {
        content.push(Line::from(vec![
            Span::styled(
                "To: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(to),
        ]));
    }

    // Add Cc field if present
    if let Some(cc) = &email.cc {
        content.push(Line::from(vec![
            Span::styled(
                "Cc: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(cc),
        ]));
    }

    // Add Bcc field if present
    if let Some(bcc) = &email.bcc {
        content.push(Line::from(vec![
            Span::styled(
                "Bcc: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(bcc),
        ]));
    }

    content.push(Line::from(vec![
        Span::styled(
            "Subject: ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(&email.subject),
    ]));

    // Add empty line separator between headers and body
    content.push(Line::from(""));

    // Add email body
    let body_text = email
        .body
        .as_ref()
        .unwrap_or(&"Loading...".to_string())
        .clone();
    let body_lines: Vec<Line> = body_text
        .lines()
        .map(|line| Line::from(line.to_string()))
        .collect();
    content.extend(body_lines);

    let combined_widget = Paragraph::new(content)
        .wrap(Wrap { trim: true })
        .scroll((0, 0)); // Can be enhanced later for scrolling
    f.render_widget(combined_widget, chunks[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::raw("q/Esc"),
        Span::styled(":back", Style::default().fg(Color::DarkGray)),
    ]))
    .style(Style::default().fg(Color::White))
    .alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}
