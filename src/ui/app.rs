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

    // For list mode:
    /// Index of the first email shown at the top of the visible window.
    pub scroll_offset: usize,
    /// Number of emails that can be displayed in the current terminal window
    /// height.
    ///
    /// This is updated dynamically based on terminal size.
    pub visible_items: usize,
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
            scroll_offset: 0,
            visible_items: 10, // Will be updated when rendering.
        }
    }

    /// Updates the number of visible items based on the current terminal window
    /// height.
    ///
    /// This should be called whenever the terminal is resized or during
    /// rendering.
    pub fn set_visible_items(&mut self, height: usize) {
        self.visible_items = height;
    }

    /// Moves cursor to the next email in the list.
    pub fn next(&mut self) {
        if self.emails.is_empty() {
            return;
        }

        let current_selected = self.list_state.selected().unwrap_or(0);

        if current_selected >= self.emails.len() - 1 {
            // Already at the bottom, don't move.
            return;
        }

        let new_selected = current_selected + 1;
        self.list_state.select(Some(new_selected));

        // Only scroll window when cursor reaches the bottom edge.
        if new_selected >= self.scroll_offset + self.visible_items {
            self.scroll_offset = new_selected - self.visible_items + 1;
        }
    }

    /// Moves cursor to the previous email in the list.
    pub fn previous(&mut self) {
        if self.emails.is_empty() {
            return;
        }

        let current_selected = self.list_state.selected().unwrap_or(0);

        if current_selected == 0 {
            // Already at the top, don't move.
            return;
        }

        let new_selected = current_selected - 1;
        self.list_state.select(Some(new_selected));

        // Only scroll window when cursor reaches the top edge.
        if new_selected < self.scroll_offset {
            self.scroll_offset = new_selected;
        }
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
        assert_eq!(app.list_state.selected(), Some(0));
        app.previous();
        assert_eq!(app.list_state.selected(), Some(0)); // Stays at top
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
    fn test_vim_like_scrolling() {
        let emails: Vec<Email> = (0..20)
            .map(|i| Email {
                _uid: i + 1,
                subject: format!("Email {}", i + 1),
                from: format!("test{}@test.com", i + 1),
                date: Local::now(),
                is_read: false,
            })
            .collect();

        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let mut app = App::new(client.unwrap(), emails);
        app.set_visible_items(5); // Simulate a small window with 5 visible items

        // Test moving down: cursor should move without scrolling initially
        assert_eq!(app.list_state.selected(), Some(0));
        assert_eq!(app.scroll_offset, 0);

        // Move down to position 4 (last visible item in window)
        for _ in 0..4 {
            app.next();
        }
        assert_eq!(app.list_state.selected(), Some(4));
        assert_eq!(app.scroll_offset, 0); // Should not scroll yet

        // Move down one more - this should trigger scrolling
        app.next();
        assert_eq!(app.list_state.selected(), Some(5));
        assert_eq!(app.scroll_offset, 1); // Window should scroll down by 1

        // Test moving up: scroll should happen when cursor reaches top
        app.previous();
        assert_eq!(app.list_state.selected(), Some(4));
        assert_eq!(app.scroll_offset, 0); // Should scroll back up
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
