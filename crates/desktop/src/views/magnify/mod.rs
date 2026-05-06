//! Magnify launcher — a Spotlight/Raycast-style secondary window summoned by
//! a global hotkey. See [`docs/architecture.md`](../../../docs/architecture.md)
//! for the lifecycle overview.
//!
//! The window is created on the first hotkey press and kept alive in
//! `Mode::Hidden` between summons; resize follows the visible result count.
//!
//! Compositional MVU layout matches the other views (state / message / view /
//! widgets), but routing is direct from `Message::Magnify` rather than through
//! `ViewMessage` because Magnify owns its own window and doesn't share
//! `UpdateCtx` with screen-driven views.

mod message;
mod state;
mod view;
mod widgets;

pub use message::MagnifyMessage;
pub use state::{CopyField, MAGNIFY_RESULTS_SCROLL_ID, MAGNIFY_SEARCH_ID, MagnifyView, Mode};
pub(crate) use view::view;

/// Window dimensions and layout constants. Shared between the view (for
/// rendering) and the handler (for OS-level `window::resize` calls).
/// `pub(crate)` because nothing outside the crate consumes these — they
/// describe one window's geometry, not a public API.
pub(crate) mod dims {
    /// Outer width of the launcher window in logical pixels. Width never
    /// changes — only height grows / shrinks with the result list.
    pub const WIDTH: f32 = 768.0;
    /// Height of the search-bar row when the launcher is collapsed (no
    /// query, locked-state pill, or unlocked-empty state).
    pub const COLLAPSED_HEIGHT: f32 = 56.0;
    /// Height per result row.
    pub const ROW_HEIGHT: f32 = 56.0;
    /// Footer hint-bar height in the searching state.
    pub const FOOTER_HEIGHT: f32 = 32.0;
    /// Maximum number of result rows visible before the list scrolls.
    pub const MAX_VISIBLE_ROWS: usize = 6;

    /// Compute the target window height from the number of visible rows.
    /// `0` rows → collapsed; `n` rows up to `MAX_VISIBLE_ROWS` adds the
    /// rows + footer below the search bar.
    pub fn height_for(rows: usize) -> f32 {
        if rows == 0 {
            COLLAPSED_HEIGHT
        } else {
            let visible = rows.min(MAX_VISIBLE_ROWS) as f32;
            COLLAPSED_HEIGHT + visible * ROW_HEIGHT + FOOTER_HEIGHT
        }
    }
}
