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
    pub client: GmailClient,
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

    // For detail mode:
    /// Scroll offset for detail view content.
    pub detail_scroll_offset: u16,
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
            client,
            mode: ViewMode::List,
            scroll_offset: 0,
            visible_items: 10, // Will be updated when rendering.
            detail_scroll_offset: 0,
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
                // Fetch email body if not already loaded
                if self.emails[selected].body.is_none() {
                    if let Ok(body) = self.client.fetch_email_body(self.emails[selected]._uid) {
                        self.emails[selected].body = Some(body);
                    }
                }
                self.mode = ViewMode::Detail(selected);
            }
        }
    }

    /// Returns to the email list view from detail view.
    pub fn back_to_list(&mut self) {
        self.mode = ViewMode::List;
        self.detail_scroll_offset = 0; // Reset detail scroll when going back to list
    }

    /// Moves cursor to the top of the visible window.
    pub fn goto_page_top(&mut self) {
        if self.emails.is_empty() {
            return;
        }

        // Move to the first visible item in the current window
        self.list_state.select(Some(self.scroll_offset));
    }

    /// Moves cursor to the middle of the visible window.
    pub fn goto_page_middle(&mut self) {
        if self.emails.is_empty() {
            return;
        }

        // Calculate the middle of the visible window
        let window_end = (self.scroll_offset + self.visible_items).min(self.emails.len());
        let window_size = window_end - self.scroll_offset;
        let middle_offset = window_size / 2;
        let middle_index = self.scroll_offset + middle_offset;

        // Ensure we don't go past the last email
        let target_index = middle_index.min(self.emails.len() - 1);
        self.list_state.select(Some(target_index));
    }

    /// Moves cursor to the bottom of the visible window.
    pub fn goto_page_bottom(&mut self) {
        if self.emails.is_empty() {
            return;
        }

        // Move to the last visible item in the current window
        let last_visible = (self.scroll_offset + self.visible_items - 1).min(self.emails.len() - 1);
        self.list_state.select(Some(last_visible));
    }

    /// Moves forward one page.
    pub fn page_forward(&mut self) {
        if self.emails.is_empty() {
            return;
        }

        // Calculate new scroll offset (one page forward)
        let new_offset = (self.scroll_offset + self.visible_items)
            .min(self.emails.len().saturating_sub(self.visible_items));

        // If we can scroll forward
        if new_offset != self.scroll_offset {
            self.scroll_offset = new_offset;
            // Move cursor to the top of the new page
            self.list_state.select(Some(self.scroll_offset));
        } else {
            // Already at the bottom, move cursor to last email
            self.list_state.select(Some(self.emails.len() - 1));
        }
    }

    /// Moves backward one page.
    pub fn page_backward(&mut self) {
        if self.emails.is_empty() {
            return;
        }

        // Calculate new scroll offset (one page backward)
        let new_offset = self.scroll_offset.saturating_sub(self.visible_items);

        // Update scroll offset and move cursor to top of new page
        self.scroll_offset = new_offset;
        self.list_state.select(Some(self.scroll_offset));
    }

    /// Scrolls the window down by half a page.
    pub fn half_page_forward(&mut self) {
        if self.emails.is_empty() {
            return;
        }

        let half_page = (self.visible_items / 2).max(1);
        let current_selected = self.list_state.selected().unwrap_or(0);
        let current_position_in_window = current_selected.saturating_sub(self.scroll_offset);

        // Scroll the window down by half a page
        let new_scroll_offset = (self.scroll_offset + half_page)
            .min(self.emails.len().saturating_sub(self.visible_items));

        // Try to keep cursor at the same relative position in the window
        let new_selected =
            (new_scroll_offset + current_position_in_window).min(self.emails.len() - 1);

        self.scroll_offset = new_scroll_offset;
        self.list_state.select(Some(new_selected));
    }

    /// Scrolls the window up by half a page.
    pub fn half_page_backward(&mut self) {
        if self.emails.is_empty() {
            return;
        }

        let half_page = (self.visible_items / 2).max(1);
        let current_selected = self.list_state.selected().unwrap_or(0);
        let current_position_in_window = current_selected.saturating_sub(self.scroll_offset);

        // Scroll the window up by half a page
        let new_scroll_offset = self.scroll_offset.saturating_sub(half_page);

        // Try to keep cursor at the same relative position in the window
        let new_selected = new_scroll_offset + current_position_in_window;

        self.scroll_offset = new_scroll_offset;
        self.list_state.select(Some(new_selected));
    }

    /// Scrolls the window down by one line.
    pub fn line_forward(&mut self) {
        if self.emails.is_empty() {
            return;
        }

        let current_selected = self.list_state.selected().unwrap_or(0);

        // Check if cursor is at the top of the visible window
        let cursor_at_top = current_selected == self.scroll_offset;

        // Scroll the window down by one line
        let new_scroll_offset =
            (self.scroll_offset + 1).min(self.emails.len().saturating_sub(self.visible_items));

        self.scroll_offset = new_scroll_offset;

        // If cursor was at top and window actually scrolled, move cursor down to stay visible
        if cursor_at_top
            && new_scroll_offset > current_selected
            && current_selected < self.emails.len() - 1
        {
            self.list_state.select(Some(current_selected + 1));
        }
    }

    /// Scrolls the window up by one line.
    pub fn line_backward(&mut self) {
        if self.emails.is_empty() {
            return;
        }

        let current_selected = self.list_state.selected().unwrap_or(0);

        // Check if cursor is at the bottom of the visible window
        let cursor_at_bottom = current_selected
            == (self.scroll_offset + self.visible_items - 1).min(self.emails.len() - 1);

        // Scroll the window up by one line
        let new_scroll_offset = self.scroll_offset.saturating_sub(1);

        self.scroll_offset = new_scroll_offset;

        // If cursor was at bottom and window actually scrolled, move cursor up to stay visible
        if cursor_at_bottom && new_scroll_offset < current_selected && current_selected > 0 {
            self.list_state.select(Some(current_selected - 1));
        }
    }

    /// Scrolls detail view down by one line (j key).
    pub fn detail_scroll_down(&mut self) {
        self.detail_scroll_offset = self.detail_scroll_offset.saturating_add(1);
    }

    /// Scrolls detail view up by one line (k key).
    pub fn detail_scroll_up(&mut self) {
        self.detail_scroll_offset = self.detail_scroll_offset.saturating_sub(1);
    }

    /// Scrolls detail view down by one line (ctrl-e).
    pub fn detail_line_forward(&mut self) {
        self.detail_scroll_offset = self.detail_scroll_offset.saturating_add(1);
    }

    /// Scrolls detail view up by one line (ctrl-y).
    pub fn detail_line_backward(&mut self) {
        self.detail_scroll_offset = self.detail_scroll_offset.saturating_sub(1);
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
                to: None,
                cc: None,
                bcc: None,
                date: Local::now(),
                is_read: false,
                body: None,
            },
            Email {
                _uid: 2,
                subject: "Email 2".to_string(),
                from: "test2@test.com".to_string(),
                to: None,
                cc: None,
                bcc: None,
                date: Local::now(),
                is_read: true,
                body: None,
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
                to: None,
                cc: None,
                bcc: None,
                date: Local::now(),
                is_read: false,
                body: None,
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
            to: None,
            cc: None,
            bcc: None,
            date: Local::now(),
            is_read: false,
            body: None,
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

    #[test]
    fn test_vim_navigation() {
        let emails: Vec<Email> = (0..20)
            .map(|i| Email {
                _uid: i + 1,
                subject: format!("Email {}", i + 1),
                from: format!("test{}@test.com", i + 1),
                to: None,
                cc: None,
                bcc: None,
                date: Local::now(),
                is_read: false,
                body: None,
            })
            .collect();

        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let mut app = App::new(client.unwrap(), emails);
        app.set_visible_items(5); // Window shows 5 items

        // Start at position 0 with scroll_offset 0
        assert_eq!(app.list_state.selected(), Some(0));
        assert_eq!(app.scroll_offset, 0);

        // Move down to trigger scrolling
        for _ in 0..7 {
            app.next();
        }
        assert_eq!(app.list_state.selected(), Some(7));
        assert_eq!(app.scroll_offset, 3); // Window scrolled to show items 3-7

        // Test goto_page_top (H key) - should go to top of visible window (item 3)
        app.goto_page_top();
        assert_eq!(app.list_state.selected(), Some(3)); // First visible item
        assert_eq!(app.scroll_offset, 3); // Scroll doesn't change

        // Test goto_page_bottom (L key) - should go to bottom of visible window (item 7)
        app.goto_page_bottom();
        assert_eq!(app.list_state.selected(), Some(7)); // Last visible item (3+5-1)
        assert_eq!(app.scroll_offset, 3); // Scroll doesn't change

        // Test goto_page_middle (M key) - should go to middle of visible window
        app.goto_page_middle();
        assert_eq!(app.list_state.selected(), Some(5)); // Middle of window (3 + 2)
        assert_eq!(app.scroll_offset, 3); // Scroll doesn't change

        // Test at end of list where window is partial
        for _ in 0..14 {
            app.next();
        }
        assert_eq!(app.list_state.selected(), Some(19)); // Last email
        assert_eq!(app.scroll_offset, 15); // Window shows items 15-19

        // Window shows items 15-19 (only 5 items visible)
        app.goto_page_top();
        assert_eq!(app.list_state.selected(), Some(15)); // Top of window

        app.goto_page_bottom();
        assert_eq!(app.list_state.selected(), Some(19)); // Bottom of window

        app.goto_page_middle();
        assert_eq!(app.list_state.selected(), Some(17)); // Middle of window (15 + 2)
    }

    #[test]
    fn test_vim_navigation_empty_list() {
        let emails = vec![];
        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let mut app = App::new(client.unwrap(), emails);

        // These should not panic on empty list
        app.goto_page_top();
        app.goto_page_middle();
        app.goto_page_bottom();
        assert_eq!(app.list_state.selected(), None);
    }

    #[test]
    fn test_page_navigation() {
        let emails: Vec<Email> = (0..30)
            .map(|i| Email {
                _uid: i + 1,
                subject: format!("Email {}", i + 1),
                from: format!("test{}@test.com", i + 1),
                to: None,
                cc: None,
                bcc: None,
                date: Local::now(),
                is_read: false,
                body: None,
            })
            .collect();

        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let mut app = App::new(client.unwrap(), emails);
        app.set_visible_items(10); // Window shows 10 items

        // Start at position 0 with scroll_offset 0
        assert_eq!(app.list_state.selected(), Some(0));
        assert_eq!(app.scroll_offset, 0);

        // Test page forward (Ctrl+F) - should move forward one page
        app.page_forward();
        assert_eq!(app.list_state.selected(), Some(10)); // Cursor at top of new page
        assert_eq!(app.scroll_offset, 10); // Window shows items 10-19

        // Another page forward
        app.page_forward();
        assert_eq!(app.list_state.selected(), Some(20)); // Cursor at top of new page
        assert_eq!(app.scroll_offset, 20); // Window shows items 20-29

        // Try to page forward at the end - should just move cursor to last item
        app.page_forward();
        assert_eq!(app.list_state.selected(), Some(29)); // Last email
        assert_eq!(app.scroll_offset, 20); // Window doesn't change

        // Test page backward (Ctrl+B) - should move backward one page
        app.page_backward();
        assert_eq!(app.list_state.selected(), Some(10)); // Cursor at top of new page
        assert_eq!(app.scroll_offset, 10); // Window shows items 10-19

        // Another page backward
        app.page_backward();
        assert_eq!(app.list_state.selected(), Some(0)); // Back to start
        assert_eq!(app.scroll_offset, 0); // Window shows items 0-9

        // Try to page backward at the beginning - should stay at start
        app.page_backward();
        assert_eq!(app.list_state.selected(), Some(0)); // Stay at start
        assert_eq!(app.scroll_offset, 0); // Window doesn't change
    }

    #[test]
    fn test_page_navigation_small_list() {
        // Test with a list smaller than one page
        let emails: Vec<Email> = (0..5)
            .map(|i| Email {
                _uid: i + 1,
                subject: format!("Email {}", i + 1),
                from: format!("test{}@test.com", i + 1),
                to: None,
                cc: None,
                bcc: None,
                date: Local::now(),
                is_read: false,
                body: None,
            })
            .collect();

        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let mut app = App::new(client.unwrap(), emails);
        app.set_visible_items(10); // Window can show 10 items but we only have 5

        // Page forward should move to last item since list is smaller than page
        app.page_forward();
        assert_eq!(app.list_state.selected(), Some(4)); // Last email
        assert_eq!(app.scroll_offset, 0); // No scrolling needed

        // Page backward should go to first item
        app.page_backward();
        assert_eq!(app.list_state.selected(), Some(0)); // First email
        assert_eq!(app.scroll_offset, 0); // No scrolling needed
    }

    #[test]
    fn test_half_page_navigation() {
        let emails: Vec<Email> = (0..30)
            .map(|i| Email {
                _uid: i + 1,
                subject: format!("Email {}", i + 1),
                from: format!("test{}@test.com", i + 1),
                to: None,
                cc: None,
                bcc: None,
                date: Local::now(),
                is_read: false,
                body: None,
            })
            .collect();

        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let mut app = App::new(client.unwrap(), emails);
        app.set_visible_items(10); // Window shows 10 items, so half-page is 5

        // Start at position 0 (cursor at top of window)
        assert_eq!(app.list_state.selected(), Some(0));
        assert_eq!(app.scroll_offset, 0);

        // Move cursor to middle of window first
        app.next();
        app.next();
        app.next(); // Now at position 3
        assert_eq!(app.list_state.selected(), Some(3));
        assert_eq!(app.scroll_offset, 0);

        // Test half page forward (Ctrl-D) - should scroll window down by 5
        app.half_page_forward();
        assert_eq!(app.scroll_offset, 5); // Window scrolled down by 5
        assert_eq!(app.list_state.selected(), Some(8)); // Cursor maintains relative position (3rd item in window)

        // Test half page backward (Ctrl-U) - should scroll window up by 5
        app.half_page_backward();
        assert_eq!(app.scroll_offset, 0); // Window scrolled back up
        assert_eq!(app.list_state.selected(), Some(3)); // Cursor back to original position

        // Test at end of list - scroll near the end first
        app.scroll_offset = 15; // Window shows items 15-24
        app.list_state.select(Some(18)); // Cursor at 3rd position in window

        // Half page forward at end - should scroll as much as possible
        app.half_page_forward();
        assert_eq!(app.scroll_offset, 20); // Max scroll for 30 items with 10 visible (30-10=20)
        assert_eq!(app.list_state.selected(), Some(23)); // Cursor maintains relative position
    }

    #[test]
    fn test_half_page_navigation_small_window() {
        let emails: Vec<Email> = (0..10)
            .map(|i| Email {
                _uid: i + 1,
                subject: format!("Email {}", i + 1),
                from: format!("test{}@test.com", i + 1),
                to: None,
                cc: None,
                bcc: None,
                date: Local::now(),
                is_read: false,
                body: None,
            })
            .collect();

        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let mut app = App::new(client.unwrap(), emails);
        app.set_visible_items(3); // Very small window, half-page = 1 (minimum)

        // Start at position 0 (cursor at top of window)
        assert_eq!(app.list_state.selected(), Some(0));
        assert_eq!(app.scroll_offset, 0);

        // Move cursor to middle of small window
        app.next(); // Now at position 1
        assert_eq!(app.list_state.selected(), Some(1));
        assert_eq!(app.scroll_offset, 0);

        // Half page forward should scroll window by 1
        app.half_page_forward();
        assert_eq!(app.scroll_offset, 1); // Window scrolled down by 1
        assert_eq!(app.list_state.selected(), Some(2)); // Cursor maintains relative position (1st item in window)

        // Half page backward should scroll window back
        app.half_page_backward();
        assert_eq!(app.scroll_offset, 0); // Window scrolled back
        assert_eq!(app.list_state.selected(), Some(1)); // Cursor back to original position
    }

    #[test]
    fn test_line_scrolling() {
        let emails: Vec<Email> = (0..20)
            .map(|i| Email {
                _uid: i + 1,
                subject: format!("Email {}", i + 1),
                from: format!("test{}@test.com", i + 1),
                to: None,
                cc: None,
                bcc: None,
                date: Local::now(),
                is_read: false,
                body: None,
            })
            .collect();

        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let mut app = App::new(client.unwrap(), emails);
        app.set_visible_items(5); // Window shows 5 items

        // Test with cursor in middle - should stay fixed
        app.list_state.select(Some(2));
        assert_eq!(app.list_state.selected(), Some(2));
        assert_eq!(app.scroll_offset, 0);

        // Line forward with cursor in middle - cursor stays fixed
        app.line_forward();
        assert_eq!(app.list_state.selected(), Some(2)); // Cursor stays at same position
        assert_eq!(app.scroll_offset, 1); // Window scrolled down by 1

        // Test with cursor at top - should move down when scrolling
        app.scroll_offset = 0;
        app.list_state.select(Some(0)); // Cursor at top of window

        app.line_forward();
        assert_eq!(app.list_state.selected(), Some(1)); // Cursor moved down to stay visible
        assert_eq!(app.scroll_offset, 1); // Window scrolled down by 1

        // Test with cursor at bottom - should move up when scrolling backward
        app.scroll_offset = 5;
        app.list_state.select(Some(9)); // Cursor at bottom of window (scroll_offset 5 + visible_items 5 - 1 = 9)

        app.line_backward();
        assert_eq!(app.list_state.selected(), Some(8)); // Cursor moved up to stay visible
        assert_eq!(app.scroll_offset, 4); // Window scrolled up by 1
    }

    #[test]
    fn test_line_scrolling_edge_cases() {
        let emails: Vec<Email> = (0..10)
            .map(|i| Email {
                _uid: i + 1,
                subject: format!("Email {}", i + 1),
                from: format!("test{}@test.com", i + 1),
                to: None,
                cc: None,
                bcc: None,
                date: Local::now(),
                is_read: false,
                body: None,
            })
            .collect();

        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let mut app = App::new(client.unwrap(), emails);
        app.set_visible_items(5); // Window shows 5 items, list has 10 items

        // Test at the beginning - line backward should do nothing when scroll is at 0
        app.list_state.select(Some(2)); // Set cursor to position 2
        assert_eq!(app.scroll_offset, 0);
        app.line_backward();
        assert_eq!(app.list_state.selected(), Some(2)); // Cursor stays at position 2
        assert_eq!(app.scroll_offset, 0); // Scroll should stay at 0 (can't go negative)

        // Test at the end - line forward should do nothing when at max scroll
        app.scroll_offset = 5; // Max scroll for 10 items with 5 visible (10-5=5)
        app.line_forward();
        assert_eq!(app.list_state.selected(), Some(2)); // Cursor stays at position 2
        assert_eq!(app.scroll_offset, 5); // Scroll should stay at max
    }

    #[test]
    fn test_line_scrolling_empty_list() {
        let emails = vec![];
        let client = GmailClient::connect("dummy", "dummy");
        if client.is_err() {
            return;
        }

        let mut app = App::new(client.unwrap(), emails);

        // These should not panic on empty list
        app.line_forward();
        app.line_backward();
        assert_eq!(app.list_state.selected(), None);
        assert_eq!(app.scroll_offset, 0);
    }
}
