//! Colors shared by the index and the pager.

use ratatui::style::Color;

/// Thread-tree prefixes and signatures: metadata that should recede.
pub(super) const META_COLOR: Color = Color::DarkGray;
/// Row color for unread messages.
pub(super) const UNREAD_COLOR: Color = Color::Yellow;
/// The `!` marker on flagged messages.
pub(super) const FLAGGED_COLOR: Color = Color::Red;
/// Quoted text in the pager, rotated by nesting depth.
pub(super) const QUOTE_COLORS: [Color; 3] = [Color::Cyan, Color::Blue, Color::Green];
/// `Date`/`From`/`Subject` header lines in the pager.
pub(super) const HEADER_PRIMARY_COLOR: Color = Color::Yellow;
/// `To`/`Cc`/`Bcc` header lines in the pager.
pub(super) const HEADER_RECIPIENT_COLOR: Color = Color::Cyan;
/// Added lines in a patch.
pub(super) const DIFF_ADD_COLOR: Color = Color::Green;
/// Removed lines in a patch.
pub(super) const DIFF_DEL_COLOR: Color = Color::Red;
/// `@@` hunk headers in a patch.
pub(super) const DIFF_HUNK_COLOR: Color = Color::Cyan;
