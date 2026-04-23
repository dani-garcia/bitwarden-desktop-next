mod handler;
mod message;
mod state;
mod update;
mod view;
mod widgets;

pub use message::{VaultEvent, VaultMessage};
pub use state::VaultView;
// Re-export for the `widgets/` submodule — these types stay private to the
// vault subtree but widgets need them reachable at the vault module path.
pub(in crate::views::vault) use state::{NavSection, SidebarFilter, SidebarMode, SidebarState};

/// Below this window width (logical px) the detail pane renders as a
/// bottom sheet instead of a side-by-side `pane_grid` split.
pub(super) const SHEET_BREAKPOINT_PX: f32 = 1000.0;

/// Visible strip at the top of the vault content above the bottom sheet
/// (≈2× `TITLE_BAR_HEIGHT`).
pub(super) const SHEET_TOP_INSET_PX: f32 = 64.0;

/// Top-corner radius of the bottom sheet.
pub(super) const SHEET_TOP_RADIUS_PX: f32 = 16.0;
