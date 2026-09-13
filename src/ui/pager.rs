//! The message pager: scrolling, stepping between messages, and styling.

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Rect, Size};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::text::wrap_lines;
use super::theme::{
    DIFF_ADD_COLOR, DIFF_DEL_COLOR, DIFF_HUNK_COLOR, HEADER_PRIMARY_COLOR, HEADER_RECIPIENT_COLOR,
    META_COLOR, QUOTE_COLORS,
};
use super::{App, Effect, Mode, page_height};

impl App {
    /// Handles a key in the pager and returns any required effect.
    pub(super) fn handle_pager_key(&mut self, key: KeyEvent, size: Size) -> Option<Effect> {
        let page = page_height(size);
        let Mode::Pager { lines, scroll } = &mut self.mode else {
            return None;
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
            return None;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.mode = Mode::Index,
            KeyCode::Char('j') | KeyCode::Down => *scroll = (*scroll + 1).min(max),
            KeyCode::Char('k') | KeyCode::Up => *scroll = scroll.saturating_sub(1),
            KeyCode::Char('J') => return self.adjacent(1).map(Effect::Open),
            KeyCode::Char('K') => return self.adjacent(-1).map(Effect::Open),
            KeyCode::PageDown => *scroll = (*scroll + page).min(max),
            KeyCode::PageUp => *scroll = scroll.saturating_sub(page),
            KeyCode::Char('g') | KeyCode::Home => *scroll = 0,
            KeyCode::Char('G') | KeyCode::End => *scroll = max,
            _ => {}
        }
        None
    }

    /// The row after/before the selection; `None` at either end.
    fn adjacent(&self, dir: isize) -> Option<usize> {
        let next = self.state.selected()? as isize + dir;
        (0..self.rows.len() as isize)
            .contains(&next)
            .then_some(next as usize)
    }

    /// Renders the open message into `area` and returns the status line text.
    pub(super) fn draw_pager(&self, frame: &mut Frame, area: Rect) -> String {
        let Mode::Pager { lines, scroll } = &self.mode else {
            return String::new();
        };
        let wrapped = style_message(lines, area.width as usize);
        let height = area.height as usize;
        let total = wrapped.len();
        let visible: Vec<Line> = wrapped.into_iter().skip(*scroll).take(height).collect();
        frame.render_widget(Paragraph::new(visible), area);

        let pct = if total <= height {
            100
        } else {
            ((scroll + height) * 100 / total).min(100)
        };
        format!(" q:Back  j/k:Scroll   -- {pct}% --")
    }
}

/// Wraps and colorizes a `headers + blank line + body` message for the pager:
/// per-field header colors with bold names, quote colors by nesting depth,
/// git-style patch colors, dim signature.
fn style_message(lines: &[String], width: usize) -> Vec<Line<'static>> {
    let mut out = Vec::new();

    let mut in_headers = true;
    let mut in_signature = false;
    // Inside an unquoted patch: entered at a `diff`/`@@` line, left at the
    // first line that is not diff-shaped.
    let mut in_diff = false;

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
                0 => {
                    if line.starts_with("diff ") || line.starts_with("@@ ") {
                        in_diff = true;
                    }
                    let diff = if in_diff { diff_style(line) } else { None };
                    in_diff = diff.is_some();
                    diff.unwrap_or_default()
                }
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

/// Style for one line of a unified diff, or `None` if the line is not
/// diff-shaped: file headers bold, hunk headers cyan, `+` green, `-` red,
/// context and `\ No newline` plain. Blank lines count as context, since
/// some mailers strip the leading space.
fn diff_style(line: &str) -> Option<Style> {
    const META_PREFIXES: [&str; 14] = [
        "diff ",
        "index ",
        "--- ",
        "+++ ",
        "similarity index ",
        "dissimilarity index ",
        "rename from ",
        "rename to ",
        "copy from ",
        "copy to ",
        "new file mode ",
        "deleted file mode ",
        "old mode ",
        "new mode ",
    ];
    if META_PREFIXES.iter().any(|p| line.starts_with(p)) || line.starts_with("Binary files ") {
        return Some(Style::default().add_modifier(Modifier::BOLD));
    }
    if line.starts_with("@@") {
        return Some(Style::default().fg(DIFF_HUNK_COLOR));
    }
    match line.chars().next() {
        Some('+') => Some(Style::default().fg(DIFF_ADD_COLOR)),
        Some('-') => Some(Style::default().fg(DIFF_DEL_COLOR)),
        Some(' ' | '\\') | None => Some(Style::default()),
        _ => None,
    }
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

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::*;
    use crate::ui::testing::{ctrl, index, key, pager, press, scroll};

    /// Foreground color of the first span of each rendered line.
    fn colors(text: &str) -> Vec<Option<Color>> {
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        style_message(&lines, 80)
            .iter()
            .map(|l| l.spans[0].style.fg)
            .collect()
    }

    #[test]
    fn colors_patch_body() {
        let got = colors(
            "Subject: [PATCH] x\n\n- a bullet, not a diff\n---\n f | 1 +\n\ndiff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-old\n+new\n context\n\nTrailing prose\n-- \n2.50.0",
        );
        assert_eq!(
            got,
            vec![
                Some(HEADER_PRIMARY_COLOR),
                None,
                None, // bullet
                None, // ---
                None, // diffstat
                None,
                None, // diff --git (bold only)
                None, // ---
                None, // +++
                Some(DIFF_HUNK_COLOR),
                Some(DIFF_DEL_COLOR),
                Some(DIFF_ADD_COLOR),
                None, // context
                None, // blank
                None, // prose leaves the diff
                Some(META_COLOR),
                Some(META_COLOR),
            ]
        );
    }

    #[test]
    fn pager_opens_adjacent_within_bounds() {
        let mut app = pager(index(&[false; 3]), 0, 1);
        assert_eq!(press(&mut app, key('K')), None);
        assert_eq!(press(&mut app, key('J')), Some(Effect::Open(1)));

        let mut app = pager(index(&[false; 3]), 2, 1);
        assert_eq!(press(&mut app, key('J')), None);
        assert_eq!(press(&mut app, key('K')), Some(Effect::Open(1)));
    }

    #[test]
    fn pager_scrolls_within_bounds() {
        let mut app = pager(index(&[false]), 0, 10);
        press(&mut app, key('k'));
        assert_eq!(scroll(&app), 0);
        press(&mut app, ctrl('f'));
        assert_eq!(scroll(&app), 5);
        press(&mut app, key('G'));
        assert_eq!(scroll(&app), 9);
        press(&mut app, key('j'));
        assert_eq!(scroll(&app), 9);
        press(&mut app, ctrl('u'));
        assert_eq!(scroll(&app), 7);
        press(&mut app, key('g'));
        assert_eq!(scroll(&app), 0);
    }

    #[test]
    fn pager_goes_back_to_index() {
        let mut app = pager(index(&[false]), 0, 1);
        assert_eq!(press(&mut app, key('q')), None);
        assert!(matches!(app.mode, Mode::Index));
    }
}
