pub mod config;
pub mod gmail_client;
pub mod ui;
pub mod utils;

pub use config::Config;
pub use gmail_client::{Email, GmailClient};
pub use ui::{App, ViewMode, run_app};
pub use utils::format_date;
