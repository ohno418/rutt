//! The threaded index: selection, paging, flag toggles, and row rendering.

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem};

use super::text::pad;
use super::theme::{FLAGGED_COLOR, META_COLOR, UNREAD_COLOR};
use super::{App, Effect};
use crate::mail::{Flag, Relation};
use crate::thread::Row;

const DATE_WIDTH: usize = 16; // "2026-12-31 23:59"
const SENDER_WIDTH: usize = 20;

impl App {
    /// Handles a key on the index and returns any required effect.
    pub(super) fn handle_index_key(&mut self, key: KeyEvent) -> Option<Effect> {
        let page = self.page_step() as isize;
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            let half = self.half_page_step() as isize;
            match key.code {
                KeyCode::Char('f') => self.page_by(page),
                KeyCode::Char('b') => self.page_by(-page),
                KeyCode::Char('d') => self.page_by(half),
                KeyCode::Char('u') => self.page_by(-half),
                KeyCode::Char('e') => self.page_by(1),
                KeyCode::Char('y') => self.page_by(-1),
                KeyCode::Char('r') => return Some(Effect::Sync),
                _ => {}
            }
            return None;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Some(Effect::Quit),
            KeyCode::Char('j') | KeyCode::Down => self.move_by(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_by(-1),
            KeyCode::Char('J') => self.select_unread(1),
            KeyCode::Char('K') => self.select_unread(-1),
            KeyCode::Char('g') | KeyCode::Home => self.select(0),
            KeyCode::Char('G') | KeyCode::End => self.select(self.rows.len().saturating_sub(1)),
            KeyCode::Char(c @ ('H' | 'M' | 'L')) => self.select_visible(c),
            KeyCode::PageDown => self.page_by(page),
            KeyCode::PageUp => self.page_by(-page),
            KeyCode::Enter => return self.index_state.selected().map(Effect::Open),
            KeyCode::Char(' ') => self.toggle_selected_read(),
            KeyCode::Char('!') => self.toggle_selected_flagged(),
            _ => {}
        }
        None
    }

    /// Flips the selected message between read and unread without opening it,
    /// then advances to the next row.
    fn toggle_selected_read(&mut self) {
        if let Some(i) = self.index_state.selected() {
            let unread = !self.rows[i].message.unread;
            self.set_unread(i, unread);
            self.move_by(1);
        }
    }

    /// Flips row `i` to read locally and queues it for the next sync.
    pub(super) fn mark_read(&mut self, i: usize) {
        if self.rows[i].message.unread {
            self.set_unread(i, false);
        }
    }

    /// Sets row `i`'s read state locally and queues it for the next sync.
    fn set_unread(&mut self, i: usize, unread: bool) {
        let m = &mut self.rows[i].message;
        m.unread = unread;
        self.pending.insert((Flag::Seen, m.uid), !unread);
    }

    /// Flips the selected message between flagged and unflagged locally,
    /// queues it for the next sync, then advances to the next row.
    fn toggle_selected_flagged(&mut self) {
        if let Some(i) = self.index_state.selected() {
            let m = &mut self.rows[i].message;
            m.flagged = !m.flagged;
            self.pending.insert((Flag::Flagged, m.uid), m.flagged);
            self.move_by(1);
        }
    }

    /// Jumps to the nearest unread row after/before the selection; stays put
    /// when there is none (no wrap-around).
    fn select_unread(&mut self, dir: isize) {
        let Some(cur) = self.index_state.selected() else {
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

    /// Moves the selection by `delta` rows, clamped to the list bounds.
    fn move_by(&mut self, delta: isize) {
        if self.rows.is_empty() {
            return;
        }
        let cur = self.index_state.selected().unwrap_or(0) as isize;
        let next = (cur + delta).clamp(0, self.rows.len() as isize - 1);
        self.select(next as usize);
    }

    pub(super) fn select(&mut self, i: usize) {
        if !self.rows.is_empty() {
            self.index_state.select(Some(i));
        }
    }

    /// Scrolls the view by `delta` rows; the selection moves only as far as
    /// needed to stay on screen.
    fn page_by(&mut self, delta: isize) {
        if self.rows.is_empty() {
            return;
        }
        let offset = self.shift_offset(delta);
        let cur = self.index_state.selected().unwrap_or(0);
        let bottom = (offset + self.visible_rows() - 1).min(self.rows.len() - 1);
        self.index_state.select(Some(cur.clamp(offset, bottom)));
    }

    /// Selects the top, middle, or bottom row on screen without scrolling.
    fn select_visible(&mut self, key: char) {
        if self.rows.is_empty() {
            return;
        }
        let top = self.index_state.offset().min(self.rows.len() - 1);
        let bottom = (top + self.visible_rows() - 1).min(self.rows.len() - 1);
        self.select(match key {
            'H' => top,
            'L' => bottom,
            _ => top + (bottom - top) / 2,
        });
    }

    /// Moves the view offset by `delta`, clamped so the last page stays full.
    fn shift_offset(&mut self, delta: isize) -> usize {
        let max = self.rows.len().saturating_sub(self.visible_rows()) as isize;
        let offset = (self.index_state.offset() as isize + delta).clamp(0, max) as usize;
        *self.index_state.offset_mut() = offset;
        offset
    }

    /// Renders the index into `area` and returns the status line text.
    pub(super) fn draw_index(&mut self, frame: &mut Frame, area: Rect) -> String {
        let items: Vec<ListItem> = self.rows.iter().map(render_row).collect();
        let list =
            List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(list, area, &mut self.index_state);

        let unread = self.rows.iter().filter(|r| r.message.unread).count();
        format!(
            " q:Quit  j/k:Move  Enter:Read  Space:Toggle  !:Flag  ^R:Sync   [{}] {} messages, {} unread",
            self.mailbox,
            self.rows.len(),
            unread
        )
    }
}

/// Formats one row as `[<flags>] <date> <time> <sender> <tree><subject>`,
/// colored if unread.
///
/// Flags are two columns: status (`D` > `N` > `r`), then flagged/recipient
/// (`!` > `F` > `T` > `C`).
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ratatui::crossterm::event::KeyCode;

    use super::*;
    use crate::ui::testing::{ctrl, index, key, press, selected};

    #[test]
    fn index_keys_return_effects() {
        let mut app = index(&[false; 3]);
        assert_eq!(press(&mut app, key('j')), None);
        assert_eq!(
            press(&mut app, KeyCode::Enter.into()),
            Some(Effect::Open(1))
        );
        assert_eq!(press(&mut app, ctrl('r')), Some(Effect::Sync));
        assert_eq!(press(&mut app, key('q')), Some(Effect::Quit));
        assert_eq!(press(&mut app, KeyCode::Esc.into()), Some(Effect::Quit));
    }

    #[test]
    fn empty_index_opens_nothing() {
        let mut app = index(&[]);
        assert_eq!(press(&mut app, key('j')), None);
        assert_eq!(press(&mut app, ctrl('f')), None);
        assert_eq!(press(&mut app, KeyCode::Enter.into()), None);
        assert_eq!(selected(&app), None);
    }

    #[test]
    fn moves_within_bounds() {
        let mut app = index(&[false; 3]);
        press(&mut app, key('k'));
        assert_eq!(selected(&app), Some(0));
        press(&mut app, key('G'));
        assert_eq!(selected(&app), Some(2));
        press(&mut app, key('j'));
        assert_eq!(selected(&app), Some(2));
        press(&mut app, key('g'));
        assert_eq!(selected(&app), Some(0));
    }

    #[test]
    fn jumps_between_unread() {
        let mut app = index(&[false, true, false, true]);
        press(&mut app, key('J'));
        assert_eq!(selected(&app), Some(1));
        press(&mut app, key('J'));
        assert_eq!(selected(&app), Some(3));
        press(&mut app, key('J'));
        assert_eq!(selected(&app), Some(3));
        press(&mut app, key('K'));
        assert_eq!(selected(&app), Some(1));
        press(&mut app, key('K'));
        assert_eq!(selected(&app), Some(1));
    }

    #[test]
    fn pages_keep_selection_on_screen() {
        let mut app = index(&[false; 10]);
        press(&mut app, ctrl('f'));
        assert_eq!((app.index_state.offset(), selected(&app)), (4, Some(4)));
        // Already at the last full page.
        press(&mut app, ctrl('f'));
        assert_eq!((app.index_state.offset(), selected(&app)), (4, Some(4)));
        // A selection still on screen stays put.
        press(&mut app, key('j'));
        press(&mut app, ctrl('b'));
        assert_eq!((app.index_state.offset(), selected(&app)), (0, Some(5)));
        press(&mut app, ctrl('d'));
        assert_eq!((app.index_state.offset(), selected(&app)), (3, Some(5)));
    }

    #[test]
    fn selects_visible_rows() {
        let mut app = index(&[false; 10]);
        press(&mut app, key('L'));
        assert_eq!(selected(&app), Some(5));
        press(&mut app, key('M'));
        assert_eq!(selected(&app), Some(2));
        press(&mut app, key('H'));
        assert_eq!(selected(&app), Some(0));

        // Fewer rows than a page.
        let mut app = index(&[false; 3]);
        press(&mut app, key('L'));
        assert_eq!(selected(&app), Some(2));
    }

    #[test]
    fn toggles_read_and_queues_sync() {
        let mut app = index(&[true, false]);
        press(&mut app, key(' '));
        assert!(!app.rows[0].message.unread);
        assert_eq!(selected(&app), Some(1));
        // The last row stays selected.
        press(&mut app, key(' '));
        assert!(app.rows[1].message.unread);
        assert_eq!(selected(&app), Some(1));
        assert_eq!(
            app.pending,
            BTreeMap::from([((Flag::Seen, 1), true), ((Flag::Seen, 2), false)])
        );
    }

    #[test]
    fn toggles_flagged_and_queues_sync() {
        let mut app = index(&[false; 2]);
        press(&mut app, key('!'));
        assert!(app.rows[0].message.flagged);
        assert_eq!(selected(&app), Some(1));
        press(&mut app, key('k'));
        press(&mut app, key('!'));
        assert!(!app.rows[0].message.flagged);
        assert_eq!(app.pending, BTreeMap::from([((Flag::Flagged, 1), false)]));
    }
}
