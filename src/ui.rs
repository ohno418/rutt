//! The threaded index view, the message pager, and their key handling.

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use unicode_width::UnicodeWidthStr;

use crate::mail::{Client, Relation};
use crate::thread::Row;

const DATE_WIDTH: usize = 16; // "2026-12-31 23:59"
const SENDER_WIDTH: usize = 20;

/// Thread-tree prefixes and signatures: metadata that should recede.
const META_COLOR: Color = Color::DarkGray;
/// Row color for unread messages.
const UNREAD_COLOR: Color = Color::Yellow;
/// The `!` marker on flagged messages.
const FLAGGED_COLOR: Color = Color::Red;
/// Quoted text in the pager, rotated by nesting depth (mutt `quoted1..N`).
const QUOTE_COLORS: [Color; 3] = [Color::Cyan, Color::Blue, Color::Green];
/// `Date`/`From`/`Subject` header lines in the pager.
const HEADER_PRIMARY_COLOR: Color = Color::Yellow;
/// `To`/`Cc`/`Bcc` header lines in the pager.
const HEADER_RECIPIENT_COLOR: Color = Color::Cyan;

/// Which screen is showing.
enum Mode {
    Index,
    /// Message text split into source lines, plus the scroll offset in wrapped lines.
    Pager {
        lines: Vec<String>,
        scroll: usize,
    },
}

/// A message that temporarily takes over the status line.
enum Status {
    /// Shown while a blocking call is in flight.
    Notice(&'static str),
    /// Failure of the last action; sticky until the next one overwrites it.
    Error(String),
}

/// Application state: the index rows, the IMAP client, and the current screen.
pub struct App {
    /// Threaded index rows, in display order.
    rows: Vec<Row>,
    /// Name of the open mailbox, shown on the status line.
    mailbox: String,
    /// IMAP session used to fetch bodies and push flag changes.
    client: Client,
    /// Index selection and scroll offset.
    state: ListState,
    /// Which screen is showing.
    mode: Mode,
    /// Message taking over the status line; `None` shows the usual content.
    status: Option<Status>,
    /// UIDs read locally but not yet synced to the server.
    pending_read: Vec<u32>,
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
            status: None,
            pending_read: Vec::new(),
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
                Mode::Index => self.handle_index_key(key, terminal)?,
                Mode::Pager { .. } => self.handle_pager_key(key, terminal)?,
            };
            if quit {
                break;
            }
        }
        self.client.logout();
        Ok(())
    }

    /// Index keys; returns true to quit.
    fn handle_index_key(&mut self, key: KeyEvent, terminal: &mut DefaultTerminal) -> Result<bool> {
        let page = page_height(terminal.size()?);
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            let half = (page / 2).max(1) as isize;
            match key.code {
                KeyCode::Char('f') => self.page_by(page as isize, page),
                KeyCode::Char('b') => self.page_by(-(page as isize), page),
                KeyCode::Char('d') => self.page_by(half, page),
                KeyCode::Char('u') => self.page_by(-half, page),
                KeyCode::Char('e') => self.page_by(1, page),
                KeyCode::Char('y') => self.page_by(-1, page),
                KeyCode::Char('r') => self.sync(terminal)?,
                _ => {}
            }
            return Ok(false);
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
            KeyCode::Char('j') | KeyCode::Down => self.move_by(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_by(-1),
            KeyCode::Char('J') => self.select_unread(1),
            KeyCode::Char('K') => self.select_unread(-1),
            KeyCode::Char('g') | KeyCode::Home => self.select(0),
            KeyCode::Char('G') | KeyCode::End => self.select(self.rows.len().saturating_sub(1)),
            KeyCode::Char(c @ ('H' | 'M' | 'L')) => self.select_visible(c, page),
            KeyCode::PageDown => self.page_by(page as isize, page),
            KeyCode::PageUp => self.page_by(-(page as isize), page),
            KeyCode::Enter => self.open_selected(terminal)?,
            KeyCode::Char(' ') => self.mark_selected_read(),
            _ => {}
        }
        Ok(false)
    }

    /// Pager keys; `q`/`Esc` return to the index, never quit.
    fn handle_pager_key(&mut self, key: KeyEvent, terminal: &mut DefaultTerminal) -> Result<bool> {
        let size = terminal.size()?;
        let page = page_height(size);
        let Mode::Pager { lines, scroll } = &mut self.mode else {
            return Ok(false);
        };
        let max = wrap_lines(lines, size.width as usize)
            .len()
            .saturating_sub(1);
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            let half = (page / 2).max(1);
            match key.code {
                KeyCode::Char('f') => *scroll = (*scroll + page).min(max),
                KeyCode::Char('b') => *scroll = scroll.saturating_sub(page),
                KeyCode::Char('d') => *scroll = (*scroll + half).min(max),
                KeyCode::Char('u') => *scroll = scroll.saturating_sub(half),
                KeyCode::Char('e') => *scroll = (*scroll + 1).min(max),
                KeyCode::Char('y') => *scroll = scroll.saturating_sub(1),
                _ => {}
            }
            return Ok(false);
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.mode = Mode::Index,
            KeyCode::Char('j') | KeyCode::Down => *scroll = (*scroll + 1).min(max),
            KeyCode::Char('k') | KeyCode::Up => *scroll = scroll.saturating_sub(1),
            KeyCode::Char('J') => self.open_adjacent(1, terminal)?,
            KeyCode::Char('K') => self.open_adjacent(-1, terminal)?,
            KeyCode::PageDown => *scroll = (*scroll + page).min(max),
            KeyCode::PageUp => *scroll = scroll.saturating_sub(page),
            KeyCode::Char('g') | KeyCode::Home => *scroll = 0,
            KeyCode::Char('G') | KeyCode::End => *scroll = max,
            _ => {}
        }
        Ok(false)
    }

    /// Show `text` on the status line while the blocking call that follows
    /// runs; the caller must then overwrite `status` with the call's outcome.
    fn notify(&mut self, terminal: &mut DefaultTerminal, text: &'static str) -> Result<()> {
        self.status = Some(Status::Notice(text));
        terminal.draw(|f| self.draw(f))?;
        Ok(())
    }

    /// Fetch the selected message and switch to the pager, marking it read
    /// locally (`PEEK` leaves the server's `\Seen` untouched until a sync).
    fn open_selected(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let Some(i) = self.state.selected() else {
            return Ok(());
        };
        self.notify(terminal, "Fetching message...")?;
        let uid = self.rows[i].message.uid;
        match self.client.fetch_body(uid) {
            Ok(text) => {
                self.mark_read(i);
                self.status = None;
                self.mode = Mode::Pager {
                    lines: text.lines().map(str::to_string).collect(),
                    scroll: 0,
                };
            }
            Err(e) => self.status = Some(Status::Error(format!("{e:#}"))),
        }
        Ok(())
    }

    /// `J`/`K` in the pager (mutt next-entry/previous-entry): open the
    /// message after/before the current one; stays put at either end.
    /// The selection is restored if the fetch fails.
    fn open_adjacent(&mut self, dir: isize, terminal: &mut DefaultTerminal) -> Result<()> {
        let Some(cur) = self.state.selected() else {
            return Ok(());
        };
        let next = cur as isize + dir;
        if next < 0 || next >= self.rows.len() as isize {
            return Ok(());
        }
        self.select(next as usize);
        self.open_selected(terminal)?;
        if matches!(self.status, Some(Status::Error(_))) {
            self.select(cur);
        }
        Ok(())
    }

    /// `Space`: mark the selected message read without opening it, then
    /// advance to the next row.
    fn mark_selected_read(&mut self) {
        if let Some(i) = self.state.selected() {
            self.mark_read(i);
            self.move_by(1);
        }
    }

    /// Flip row `i` to read locally and queue its UID for the next sync.
    fn mark_read(&mut self, i: usize) {
        let m = &mut self.rows[i].message;
        if m.unread {
            m.unread = false;
            self.pending_read.push(m.uid);
        }
    }

    /// `ctrl-r` (mutt sync-mailbox): push locally-read messages' `\Seen` to the server.
    fn sync(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.notify(terminal, "Syncing...")?;
        match self.client.store_seen(&self.pending_read) {
            Ok(()) => {
                self.pending_read.clear();
                self.status = None;
            }
            Err(e) => self.status = Some(Status::Error(format!("{e:#}"))),
        }
        Ok(())
    }

    /// `J`/`K`: jump to the nearest unread row after/before the selection;
    /// stays put when there is none (no wrap-around).
    fn select_unread(&mut self, dir: isize) {
        let Some(cur) = self.state.selected() else {
            return;
        };
        let found = if dir > 0 {
            self.rows
                .iter()
                .enumerate()
                .skip(cur + 1)
                .find(|(_, r)| r.message.unread)
        } else {
            self.rows[..cur]
                .iter()
                .enumerate()
                .rev()
                .find(|(_, r)| r.message.unread)
        };
        if let Some((i, _)) = found {
            self.select(i);
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

    /// Vim `ctrl-f`/`ctrl-b`/`ctrl-d`/`ctrl-u`/`ctrl-e`/`ctrl-y`: scroll the
    /// view by `delta` rows; the selection moves only as far as needed to
    /// stay on screen.
    fn page_by(&mut self, delta: isize, page: usize) {
        if self.rows.is_empty() {
            return;
        }
        let offset = self.shift_offset(delta, page) as isize;
        let cur = self.state.selected().unwrap_or(0) as isize;
        let bottom = (offset + page as isize - 1).min(self.rows.len() as isize - 1);
        self.state.select(Some(cur.clamp(offset, bottom) as usize));
    }

    /// Vim `H`/`M`/`L`: select the top, middle, or bottom row on screen
    /// without scrolling.
    fn select_visible(&mut self, key: char, page: usize) {
        if self.rows.is_empty() {
            return;
        }
        let top = self.state.offset().min(self.rows.len() - 1);
        let bottom = (top + page - 1).min(self.rows.len() - 1);
        self.select(match key {
            'H' => top,
            'L' => bottom,
            _ => top + (bottom - top) / 2,
        });
    }

    /// Move the view offset by `delta`, clamped so the last page stays full.
    fn shift_offset(&mut self, delta: isize, page: usize) -> usize {
        let max = self.rows.len().saturating_sub(page) as isize;
        let offset = (self.state.offset() as isize + delta).clamp(0, max) as usize;
        *self.state.offset_mut() = offset;
        offset
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
                    " q:Quit  j/k:Move  Enter:Read  ^R:Sync   [{}] {} messages, {} unread",
                    self.mailbox,
                    self.rows.len(),
                    unread
                )
            }
            Mode::Pager { lines, scroll } => {
                let wrapped = style_message(lines, main_area.width as usize);
                let height = main_area.height as usize;
                let total = wrapped.len();
                let visible: Vec<Line> = wrapped.into_iter().skip(*scroll).take(height).collect();
                frame.render_widget(Paragraph::new(visible), main_area);

                let pct = if total <= height {
                    100
                } else {
                    ((scroll + height) * 100 / total).min(100)
                };
                format!(" q:Back  j/k:Scroll   -- {pct}% --")
            }
        };
        let (status, style) = match &self.status {
            Some(Status::Notice(n)) => (
                format!(" {n}"),
                Style::default().fg(Color::Black).bg(Color::Cyan),
            ),
            Some(Status::Error(e)) => (
                format!(" Error: {e}"),
                Style::default().fg(Color::White).bg(Color::Red),
            ),
            None => (status, Style::default().fg(Color::Black).bg(Color::Cyan)),
        };
        frame.render_widget(Paragraph::new(status).style(style), status_area);
    }
}

/// Format one row as `[<flags>] <date> <time> <sender> <tree><subject>`,
/// colored if unread.
///
/// Flags follow mutt's `%Z`: status (`D` > `N` > `r`), then flagged/recipient
/// (`!` > `F` > `T` > `C`); each is a space when absent, so `[N!]`, `[ C]`, `[  ]`.
fn render_row(row: &Row) -> ListItem<'_> {
    let m = &row.message;
    let date = m
        .date
        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default();
    let sender = pad(&m.sender, SENDER_WIDTH);
    let text_style = if m.unread {
        Style::default().fg(UNREAD_COLOR)
    } else {
        Style::default()
    };
    let status = if m.deleted {
        'D'
    } else if m.unread {
        'N'
    } else if m.answered {
        'r'
    } else {
        ' '
    };
    let (relation, relation_style) = if m.flagged {
        ('!', Style::default().fg(FLAGGED_COLOR))
    } else {
        let c = match m.relation {
            Relation::FromMe => 'F',
            Relation::ToMe => 'T',
            Relation::CcMe => 'C',
            Relation::None => ' ',
        };
        (c, text_style)
    };
    ListItem::new(Line::from(vec![
        Span::styled(format!("[{status}"), text_style),
        Span::styled(relation.to_string(), relation_style),
        Span::styled(format!("]  {date:<DATE_WIDTH$}  "), text_style),
        Span::styled(format!("{sender}  "), text_style),
        Span::styled(row.prefix.as_str(), Style::default().fg(META_COLOR)),
        Span::styled(m.subject.as_str(), text_style),
    ]))
}

/// Wrap and colorize a `headers + blank line + body` message for the pager:
/// per-field header colors with bold names, quote colors by nesting depth,
/// dim signature.
fn style_message(lines: &[String], width: usize) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let mut in_headers = true;
    let mut in_signature = false;
    for line in lines {
        if in_headers && line.is_empty() {
            in_headers = false;
        }
        if !in_headers && line == "-- " {
            in_signature = true;
        }
        let style = if in_signature {
            Style::default().fg(META_COLOR)
        } else if in_headers {
            match line.split(':').next() {
                Some("Date" | "From" | "Subject") => Style::default()
                    .fg(HEADER_PRIMARY_COLOR)
                    .add_modifier(Modifier::BOLD),
                Some("To" | "Cc" | "Bcc") => Style::default().fg(HEADER_RECIPIENT_COLOR),
                _ => Style::default(),
            }
        } else {
            match quote_depth(line) {
                0 => Style::default(),
                d => Style::default().fg(QUOTE_COLORS[(d - 1) % QUOTE_COLORS.len()]),
            }
        };
        let name_len = if in_headers {
            line.find(':').map(|i| i + 1).unwrap_or(0)
        } else {
            0
        };
        for (i, chunk) in wrap_lines(std::slice::from_ref(line), width)
            .into_iter()
            .enumerate()
        {
            // Bold the `Name:` prefix on the first wrapped chunk of a header line.
            if i == 0 && name_len > 0 && chunk.is_char_boundary(name_len) {
                let (name, rest) = chunk.split_at(name_len);
                out.push(Line::from(vec![
                    Span::styled(name.to_string(), style.add_modifier(Modifier::BOLD)),
                    Span::styled(rest.to_string(), style),
                ]));
            } else {
                out.push(Line::from(Span::styled(chunk, style)));
            }
        }
    }
    out
}

/// Rows in one page of the main area (screen minus status line, minus one
/// line of overlap for scroll context).
fn page_height(size: ratatui::layout::Size) -> usize {
    size.height.saturating_sub(2).max(1) as usize
}

/// Quote nesting depth: leading `>` characters, ignoring interleaved spaces.
fn quote_depth(line: &str) -> usize {
    let mut depth = 0;
    for ch in line.chars() {
        match ch {
            '>' => depth += 1,
            ' ' => {}
            _ => break,
        }
    }
    depth
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
