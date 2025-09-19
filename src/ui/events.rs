//! Event handling and main application loop.
//!
//! Processes keyboard input and manages the main UI event loop for navigation
//! and application control.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{Terminal, backend::Backend};

use crate::ui::app::{App, ViewMode};
use crate::ui::render::ui;

pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
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
