//! A minimal Gmail IMAP client with a mutt-like terminal interface.
//!
//! This crate provides a simple TUI application for reading Gmail messages via
//! IMAP connection with SSL/TLS support.

mod config;
mod gmail_client;
mod ui;
mod utils;

pub use config::Config;
pub use gmail_client::{Email, GmailClient, NameAddr};
pub use ui::{App, run_app};
