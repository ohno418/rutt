//! Terminal user interface components using ratatui.
//!
//! Provides the main UI loop, application state management, and rendering for
//! email list and detail views.

mod app;
mod events;
mod render;

pub use app::App;
pub use events::run_app;
