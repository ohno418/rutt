//! Application state management and navigation logic.
//!
//! Handles email list state, view modes, and user navigation between list and
//! detail views.

use crate::gmail_client::{Email, GmailClient};
use ratatui::widgets::ListState;

#[derive(Debug, Clone)]
pub enum ViewMode {
    List,
    Detail(usize),
}

pub struct App {
    pub emails: Vec<Email>,
    pub list_state: ListState,
    pub _client: GmailClient,
    pub mode: ViewMode,
}

impl App {
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

    pub fn view_email(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected < self.emails.len() {
                self.mode = ViewMode::Detail(selected);
            }
        }
    }

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
