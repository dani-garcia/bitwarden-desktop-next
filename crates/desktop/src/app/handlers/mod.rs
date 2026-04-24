//! App-level event handlers — window/system/menu/tray dispatch, plus the
//! cross-view handlers for chrome that doesn't belong to any single view
//! (sidebar, account switcher). Per-view event handlers live next to the
//! view they belong to (`views/<name>/handler.rs`).

mod account_switcher;
mod platform;
mod sidebar;
