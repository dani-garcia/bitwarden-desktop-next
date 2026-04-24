mod handler;
mod message;
mod state;
mod update;
mod view;
mod widgets;

pub use message::{SendEvent, SendMessage};
pub use state::SendView;

/// Below this window width (logical px) the form pane renders as a
/// bottom sheet instead of a side-by-side `pane_grid` split. Mirrors the
/// vault view's breakpoint so both authenticated screens behave alike.
pub(super) const SHEET_BREAKPOINT_PX: f32 = 1000.0;
pub(super) const SHEET_TOP_INSET_PX: f32 = 64.0;
pub(super) const SHEET_TOP_RADIUS_PX: f32 = 16.0;
