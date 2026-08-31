//! The threaded index view, the message pager, and their key handling.

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use unicode_width::UnicodeWidthStr;

use crate::mail::Client;
use crate::thread::Row;

const DATE_WIDTH: usize = 16; // "2026-12-31 23:59"
const SENDER_WIDTH: usize = 20;

/// Which screen is showing.
enum Mode {
    Index,
    /// Message text split into source lines, plus the scroll offset in wrapped lines.
    Pager {
        lines: Vec<String>,
        scroll: usize,
    },
}

/// Application state: the index rows, the IMAP client, and the current screen.
pub struct App {
    rows: Vec<Row>,
    mailbox: String,
    client: Client,
    state: ListState,
    mode: Mode,
    /// Error from the last action, shown on the status line.
    error: Option<String>,
}

impl App {
    /// Create the view with the first row selected.
    pub fn new(rows: Vec<Row>, mailbox: String, client: Client) -> Self {
        let mut state = ListState::default();
        if !rows.is_empty() {
            state.select(Some(0));
        }
        Self {
            rows,
            mailbox,
            client,
            state,
            mode: Mode::Index,
            error: None,
        }
    }

    /// Event loop: redraw and handle keys until the user quits, then log out.
    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|f| self.draw(f))?;
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
                break;
            }
            let quit = match self.mode {
                Mode::Index => self.handle_index_key(key.code),
                Mode::Pager { .. } => self.handle_pager_key(key.code, terminal.size()?),
            };
            if quit {
                break;
            }
        }
        self.client.logout();
        Ok(())
    }

    /// Index keys; returns true to quit.
    fn handle_index_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('j') | KeyCode::Down => self.move_by(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_by(-1),
            KeyCode::Char('g') | KeyCode::Home => self.select(0),
            KeyCode::Char('G') | KeyCode::End => self.select(self.rows.len().saturating_sub(1)),
            KeyCode::Enter => self.open_selected(),
            _ => {}
        }
        false
    }

    /// Pager keys; `q`/`i` return to the index, never quit.
    fn handle_pager_key(&mut self, code: KeyCode, size: ratatui::layout::Size) -> bool {
        let page = size.height.saturating_sub(2).max(1) as usize;
        let Mode::Pager { lines, scroll } = &mut self.mode else {
            return false;
        };
        let max = wrap_lines(lines, size.width as usize)
            .len()
            .saturating_sub(1);
        match code {
            KeyCode::Char('q') | KeyCode::Char('i') | KeyCode::Esc => self.mode = Mode::Index,
            KeyCode::Char('j') | KeyCode::Down => *scroll = (*scroll + 1).min(max),
            KeyCode::Char('k') | KeyCode::Up => *scroll = scroll.saturating_sub(1),
            KeyCode::Char(' ') | KeyCode::PageDown => *scroll = (*scroll + page).min(max),
            KeyCode::Char('b') | KeyCode::PageUp => *scroll = scroll.saturating_sub(page),
            KeyCode::Char('g') | KeyCode::Home => *scroll = 0,
            KeyCode::Char('G') | KeyCode::End => *scroll = max,
            _ => {}
        }
        false
    }

    /// Fetch the selected message and switch to the pager.
    fn open_selected(&mut self) {
        let Some(i) = self.state.selected() else {
            return;
        };
        let uid = self.rows[i].message.uid;
        match self.client.fetch_body(uid) {
            Ok(text) => {
                self.error = None;
                self.mode = Mode::Pager {
                    lines: text.lines().map(str::to_string).collect(),
                    scroll: 0,
                };
            }
            Err(e) => self.error = Some(format!("{e:#}")),
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

    /// Render the current screen and the status line.
    fn draw(&mut self, frame: &mut Frame) {
        let [main_area, status_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

        let status = match &self.mode {
            Mode::Index => {
                let items: Vec<ListItem> = self.rows.iter().map(render_row).collect();
                let list = List::new(items)
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
                frame.render_stateful_widget(list, main_area, &mut self.state);

                let unread = self.rows.iter().filter(|r| r.message.unread).count();
                format!(
                    " q:Quit  j/k:Move  Enter:Read   [{}] {} messages, {} unread",
                    self.mailbox,
                    self.rows.len(),
                    unread
                )
            }
            Mode::Pager { lines, scroll } => {
                let wrapped = wrap_lines(lines, main_area.width as usize);
                let height = main_area.height as usize;
                let visible: Vec<Line> = wrapped
                    .iter()
                    .skip(*scroll)
                    .take(height)
                    .map(|l| Line::from(l.as_str()))
                    .collect();
                frame.render_widget(Paragraph::new(visible), main_area);

                let pct = if wrapped.len() <= height {
                    100
                } else {
                    ((scroll + height) * 100 / wrapped.len()).min(100)
                };
                format!(" q:Back  j/k:Scroll   -- {pct}% --")
            }
        };
        let status = match &self.error {
            Some(e) => format!(" Error: {e}"),
            None => status,
        };
        let style = if self.error.is_some() {
            Style::default().fg(Color::White).bg(Color::Red)
        } else {
            Style::default().fg(Color::Black).bg(Color::Green)
        };
        frame.render_widget(Paragraph::new(status).style(style), status_area);
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
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    ListItem::new(Line::from(Span::styled(text, style)))
}

/// Hard-wrap each line at `width` display columns (no word breaking, tabs as 4 spaces).
fn wrap_lines(lines: &[String], width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    for line in lines {
        let line = line.replace('\t', "    ");
        let mut cur = String::new();
        let mut w = 0;
        for ch in line.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if w + cw > width && !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
                w = 0;
            }
            cur.push(ch);
            w += cw;
        }
        out.push(cur);
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_by_display_width() {
        let lines = vec![
            "abcdefgh".to_string(),
            "".to_string(),
            "日本語テキスト".to_string(),
        ];
        assert_eq!(
            wrap_lines(&lines, 6),
            vec!["abcdef", "gh", "", "日本語", "テキス", "ト"]
        );
    }
}
