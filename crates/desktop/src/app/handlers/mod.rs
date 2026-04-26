//! App-level event handlers — window/system/menu/tray dispatch, plus
//! cross-view handlers for chrome that doesn't belong to any single view
//! (sidebar, account switcher). Per-view handlers live in
//! `views/<name>/handler.rs`.

mod account_switcher;
pub(super) mod magnify;
mod platform;
mod sidebar;
