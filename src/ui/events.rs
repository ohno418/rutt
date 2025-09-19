//! Event handling and main application loop.
//!
//! Processes keyboard input and manages the main UI event loop for navigation
//! and application control.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{Terminal, backend::Backend};

use crate::ui::app::{App, ViewMode};
use crate::ui::render::ui;

/// Main application event loop handling keyboard input and UI updates.
///
/// Continuously renders the UI and processes keyboard events until the user
/// quits. Supports navigation in list view and switching between views.
pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match app.mode {
                    ViewMode::List => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.next(),
                        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.next()
                        }
                        KeyCode::Char('k') | KeyCode::Up => app.previous(),
                        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.previous()
                        }
                        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.page_forward()
                        }
                        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.page_backward()
                        }
                        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.half_page_forward()
                        }
                        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.half_page_backward()
                        }
                        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.line_forward()
                        }
                        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.line_backward()
                        }
                        KeyCode::Char('H') => app.goto_page_top(),
                        KeyCode::Char('M') => app.goto_page_middle(),
                        KeyCode::Char('L') => app.goto_page_bottom(),
                        KeyCode::Enter => app.view_email(),
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        _ => {}
                    },
                    ViewMode::Detail(_) => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.detail_scroll_down(),
                        KeyCode::Char('k') | KeyCode::Up => app.detail_scroll_up(),
                        KeyCode::Char('n') | KeyCode::Char('e')
                            if key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            app.detail_line_forward()
                        }
                        KeyCode::Char('p') | KeyCode::Char('y')
                            if key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            app.detail_line_backward()
                        }
                        KeyCode::Char('q') | KeyCode::Esc => app.back_to_list(),
                        _ => {}
                    },
                }
            }
        }
    }
}
