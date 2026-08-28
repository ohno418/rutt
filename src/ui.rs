//! The threaded index view and its key handling.

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use unicode_width::UnicodeWidthStr;

use crate::thread::Row;

const DATE_WIDTH: usize = 16; // "2026-12-31 23:59"
const SENDER_WIDTH: usize = 20;

/// Application state for the index view.
pub struct App {
    rows: Vec<Row>,
    mailbox: String,
    state: ListState,
}

impl App {
    /// Create the view with the first row selected.
    pub fn new(rows: Vec<Row>, mailbox: String) -> Self {
        let mut state = ListState::default();
        if !rows.is_empty() {
            state.select(Some(0));
        }
        Self {
            rows,
            mailbox,
            state,
        }
    }

    /// Event loop: redraw and handle keys until the user quits.
    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|f| self.draw(f))?;
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) => return Ok(()),
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(()),
                    (KeyCode::Char('j') | KeyCode::Down, _) => self.move_by(1),
                    (KeyCode::Char('k') | KeyCode::Up, _) => self.move_by(-1),
                    (KeyCode::Char('g') | KeyCode::Home, _) => self.select(0),
                    (KeyCode::Char('G') | KeyCode::End, _) => {
                        self.select(self.rows.len().saturating_sub(1))
                    }
                    _ => {}
                }
            }
        }
    }

    /// Move the selection by `delta` rows, clamped to the list bounds.
    fn move_by(&mut self, delta: isize) {
        if self.rows.is_empty() {
            return;
        }
        let cur = self.state.selected().unwrap_or(0) as isize;
        let next = (cur + delta).clamp(0, self.rows.len() as isize - 1);
        self.select(next as usize);
    }

    fn select(&mut self, i: usize) {
        if !self.rows.is_empty() {
            self.state.select(Some(i));
        }
    }

    /// Render the message list and the status line.
    fn draw(&mut self, frame: &mut Frame) {
        let [list_area, status_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

        let items: Vec<ListItem> = self.rows.iter().map(render_row).collect();
        let list = List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(list, list_area, &mut self.state);

        let unread = self.rows.iter().filter(|r| r.message.unread).count();
        let status = format!(
            " q:Quit  j/k:Move   [{}] {} messages, {} unread",
            self.mailbox,
            self.rows.len(),
            unread
        );
        frame.render_widget(
            Paragraph::new(status).style(Style::default().fg(Color::Black).bg(Color::Green)),
            status_area,
        );
    }
}

/// Format one row as `<date> <time> <sender> <tree><subject>`, highlighted if unread.
fn render_row(row: &Row) -> ListItem<'_> {
    let m = &row.message;
    let date = m
        .date
        .map(|d| d.format("%Y-%-m-%-d %H:%M").to_string())
        .unwrap_or_default();
    let sender = pad(&m.sender, SENDER_WIDTH);
    let text = format!(
        "{:<dw$} {} {}{}",
        date,
        sender,
        row.prefix,
        m.subject,
        dw = DATE_WIDTH
    );
    let style = if m.unread {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    ListItem::new(Line::from(Span::styled(text, style)))
}

/// Pad or truncate `s` to exactly `width` display columns.
fn pad(s: &str, width: usize) -> String {
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > width {
            break;
        }
        out.push(ch);
        w += cw;
    }
    while w < width {
        out.push(' ');
        w += 1;
    }
    debug_assert_eq!(out.width(), width);
    out
}
