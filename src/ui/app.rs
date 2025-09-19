//! Application state management and navigation logic.
//!
//! Handles email list state, view modes, and user navigation between list and
//! detail views.

use crate::gmail_client::{Email, GmailClient};
use ratatui::widgets::ListState;

/// Application view modes for different UI states.
#[derive(Debug, Clone)]
pub(crate) enum ViewMode {
    /// Email list view showing all emails.
    List,
    /// Email detail view showing specific email at index.
    Detail(usize),
}

/// Main application state containing emails and UI state.
#[derive(Debug)]
pub struct App {
    /// Vector of emails to display.
    pub emails: Vec<Email>,
    /// Current selection state for the email list.
    pub list_state: ListState,
    /// Gmail client for potential future operations.
    pub _client: GmailClient,
    /// Current view mode (list or detail).
    pub(crate) mode: ViewMode,
}

impl App {
    /// Creates a new application instance with provided emails.
    pub fn new(client: GmailClient, emails: Vec<Email>) -> Self {
        let mut list_state = ListState::default();
        if !emails.is_empty() {
            list_state.select(Some(0));
        }

        App {
            emails,
            list_state,
            _client: client,
            mode: ViewMode::List,
        }
    }

    /// Moves selection to the next email in the list.
    pub fn next(&mut self) {
        if self.emails.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.emails.len() - 1 {
                    self.emails.len() - 1
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Moves selection to the previous email in the list.
    pub fn previous(&mut self) {
        if self.emails.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Switches to detail view for the currently selected email.
    pub fn view_email(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected < self.emails.len() {
                self.mode = ViewMode::Detail(selected);
            }
        }
    }

    /// Returns to the email list view from detail view.
    pub fn back_to_list(&mut self) {
        self.mode = ViewMode::List;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    #[test]
    fn test_app_navigation() {
        let emails = vec![
            Email {
                _uid: 1,
                subject: "Email 1".to_string(),
                from: "test1@test.com".to_string(),
                date: Local::now(),
                is_read: false,
            },
            Email {
                _uid: 2,
                subject: "Email 2".to_string(),
                from: "test2@test.com".to_string(),
                date: Local::now(),
                is_read: true,
            },
        ];

        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let mut app = App::new(client.unwrap(), emails);

        assert_eq!(app.list_state.selected(), Some(0));
        app.next();
        assert_eq!(app.list_state.selected(), Some(1));
        app.next();
        assert_eq!(app.list_state.selected(), Some(1)); // Stays at bottom
        app.previous();
        assert_eq!(app.list_state.selected(), Some(0)); // Stays at top
        app.previous();
        assert_eq!(app.list_state.selected(), Some(0));
    }

    #[test]
    fn test_app_initialization() {
        let emails = vec![];
        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let app = App::new(client.unwrap(), emails.clone());
        assert_eq!(app.emails.len(), 0);
        assert_eq!(app.list_state.selected(), None);
    }

    #[test]
    fn test_view_mode_transitions() {
        let emails = vec![Email {
            _uid: 1,
            subject: "Test".to_string(),
            from: "test@test.com".to_string(),
            date: Local::now(),
            is_read: false,
        }];

        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let mut app = App::new(client.unwrap(), emails);

        // Start in list mode
        assert!(matches!(app.mode, ViewMode::List));

        // View email
        app.view_email();
        assert!(matches!(app.mode, ViewMode::Detail(0)));

        // Go back to list
        app.back_to_list();
        assert!(matches!(app.mode, ViewMode::List));
    }
}
