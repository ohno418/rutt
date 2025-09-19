//! Terminal user interface components using ratatui.
//!
//! Provides the main UI loop, application state management, and rendering for
//! email list and detail views.

pub mod app;
pub mod events;
pub mod render;

pub use app::{App, ViewMode};
pub use events::run_app;
pub use render::{render_detail, render_list, ui};
