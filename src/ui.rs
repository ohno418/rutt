//! The event loop and status line shared by the index view and the pager.

mod index;
mod pager;
mod text;
mod theme;

use std::collections::BTreeMap;

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{ListState, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use crate::mail::{Client, Flag};
use crate::thread::Row;

/// The screen currently shown.
enum Mode {
    /// The threaded message list.
    Index,
    /// A single message's text.
    Pager {
        /// Message text split into source lines.
        lines: Vec<String>,
        /// Scroll offset, counted in wrapped lines.
        scroll: usize,
    },
}

/// Work a key handler leaves to the event loop.
///
/// Exiting, or a blocking call that needs a status notice drawn first.
#[derive(Debug, PartialEq, Eq)]
enum Effect {
    /// Leave the event loop.
    Quit,
    /// Fetch row `i` and show it in the pager.
    Open(usize),
    /// Push pending flag changes to the server.
    Sync,
}

/// A message that temporarily takes over the status line.
enum Status {
    /// Shown while a blocking call is in flight.
    Notice(&'static str),
    /// Failure of the last action; sticky until the next one overwrites it.
    Error(String),
}

/// UI state: the index rows, the current screen, and unsynced flag changes.
pub struct App {
    /// Threaded index rows, in display order.
    rows: Vec<Row>,
    /// Name of the open mailbox, shown on the status line.
    mailbox: String,
    /// Index selection and scroll offset.
    state: ListState,
    /// Which screen is showing.
    mode: Mode,
    /// Message taking over the status line; `None` shows the usual content.
    status: Option<Status>,
    /// Flag changes not yet synced to the server: (flag, UID) to whether it is set.
    pending: BTreeMap<(Flag, u32), bool>,
    /// Main area from the last draw; key handlers size pages from it.
    viewport: Rect,
}

impl App {
    /// Creates the view with the first row selected.
    pub fn new(rows: Vec<Row>, mailbox: String) -> Self {
        let mut state = ListState::default();
        if !rows.is_empty() {
            state.select(Some(0));
        }
        Self {
            rows,
            mailbox,
            state,
            mode: Mode::Index,
            status: None,
            pending: BTreeMap::new(),
            viewport: Rect::default(),
        }
    }

    /// Event loop: redraws and handles keys until the user quits.
    pub fn run(mut self, terminal: &mut DefaultTerminal, client: &mut Client) -> Result<()> {
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
            let effect = match self.mode {
                Mode::Index => self.handle_index_key(key),
                Mode::Pager { .. } => self.handle_pager_key(key),
            };
            match effect {
                None => {}
                Some(Effect::Quit) => break,
                Some(Effect::Open(i)) => {
                    self.notify(terminal, "Fetching message...")?;
                    self.open(client, i);
                }
                Some(Effect::Sync) => {
                    self.notify(terminal, "Syncing...")?;
                    self.sync(client);
                }
            }
        }
        Ok(())
    }

    /// Shows `text` on the status line while the blocking call that follows
    /// runs; the caller must then overwrite `status` with the call's outcome.
    fn notify(&mut self, terminal: &mut DefaultTerminal, text: &'static str) -> Result<()> {
        self.status = Some(Status::Notice(text));
        terminal.draw(|f| self.draw(f))?;
        Ok(())
    }

    /// Fetches row `i`, then selects it and switches to the pager, marking it
    /// read locally (`PEEK` leaves the server's `\Seen` untouched until a
    /// sync). On failure the selection and screen stay as they were.
    fn open(&mut self, client: &mut Client, i: usize) {
        let uid = self.rows[i].message.uid;
        match client.fetch_body(uid) {
            Ok(text) => {
                self.select(i);
                self.mark_read(i);
                self.status = None;
                self.mode = Mode::Pager {
                    lines: text.lines().map(str::to_string).collect(),
                    scroll: 0,
                };
            }
            Err(e) => self.status = Some(Status::Error(format!("{e:#}"))),
        }
    }

    /// Pushes local flag changes to the server.
    fn sync(&mut self, client: &mut Client) {
        let mut groups: BTreeMap<(Flag, bool), Vec<u32>> = BTreeMap::new();
        for (&(flag, uid), &on) in &self.pending {
            groups.entry((flag, on)).or_default().push(uid);
        }
        let result = groups
            .iter()
            .try_for_each(|(&(flag, on), uids)| client.store_flag(uids, flag, on));

        match result {
            Ok(()) => {
                self.pending.clear();
                self.status = None;
            }
            Err(e) => self.status = Some(Status::Error(format!("{e:#}"))),
        }
    }

    /// Renders the current screen and the status line.
    fn draw(&mut self, frame: &mut Frame) {
        let [main_area, status_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());
        self.viewport = main_area;

        let status = match self.mode {
            Mode::Index => self.draw_index(frame, main_area),
            Mode::Pager { .. } => self.draw_pager(frame, main_area),
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

    /// Rows visible in the main area.
    fn visible_rows(&self) -> usize {
        (self.viewport.height as usize).max(1)
    }

    /// Rows moved by a full page: one row of overlap for scroll context.
    fn page_step(&self) -> usize {
        self.visible_rows().saturating_sub(1).max(1)
    }

    /// Rows moved by a half page.
    fn half_page_step(&self) -> usize {
        (self.visible_rows() / 2).max(1)
    }
}

/// Fixtures and key helpers for the index and pager tests.
#[cfg(test)]
mod testing {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::layout::Rect;

    use super::{App, Effect, Mode};
    use crate::mail::{Message, Relation};
    use crate::thread::Row;

    /// An index over rows with UIDs 1.. and the given unread states, drawn
    /// in a main area of 6 rows.
    pub(super) fn index(unread: &[bool]) -> App {
        let rows = unread
            .iter()
            .enumerate()
            .map(|(i, &unread)| Row {
                message: Message {
                    uid: i as u32 + 1,
                    date: None,
                    sender: String::new(),
                    subject: String::new(),
                    message_id: None,
                    references: Vec::new(),
                    unread,
                    answered: false,
                    deleted: false,
                    flagged: false,
                    relation: Relation::None,
                },
                prefix: String::new(),
            })
            .collect();

        let mut app = App::new(rows, "INBOX".to_string());
        app.viewport = Rect::new(0, 0, 80, 6);
        app
    }

    /// The same app with row `i` selected and `n` lines open in the pager.
    pub(super) fn pager(mut app: App, i: usize, n: usize) -> App {
        app.select(i);
        app.mode = Mode::Pager {
            lines: vec!["x".to_string(); n],
            scroll: 0,
        };
        app
    }

    pub(super) fn press(app: &mut App, key: KeyEvent) -> Option<Effect> {
        match app.mode {
            Mode::Index => app.handle_index_key(key),
            Mode::Pager { .. } => app.handle_pager_key(key),
        }
    }

    pub(super) fn key(c: char) -> KeyEvent {
        KeyEvent::from(KeyCode::Char(c))
    }

    pub(super) fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    pub(super) fn selected(app: &App) -> Option<usize> {
        app.state.selected()
    }

    pub(super) fn scroll(app: &App) -> usize {
        match app.mode {
            Mode::Pager { scroll, .. } => scroll,
            Mode::Index => panic!("not in the pager"),
        }
    }
}
