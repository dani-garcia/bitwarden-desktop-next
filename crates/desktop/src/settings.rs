//! App-wide user settings, read once from `data/settings.json`.
//!
//! No UI yet — the user edits the file by hand. The struct is designed so a
//! future settings view can mutate fields on the `App` struct and have changes
//! apply immediately: every behavioural branch reads `self.settings.<field>`
//! at the moment of the event (close, minimize, startup) rather than caching
//! a derived decision. See `docs/decisions.md`.

use std::io::BufReader;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Master switch: keep the tray icon visible even when the app is
    /// in the foreground.
    pub show_tray_icon: bool,
    /// Clicking the minimize button hides the window (not the taskbar
    /// minimize). Auto-creates the tray if not already shown.
    pub minimize_to_tray: bool,
    /// Clicking the OS close button hides the window instead of exiting.
    /// Auto-creates the tray if not already shown.
    pub close_to_tray: bool,
    /// On launch, don't open the main window; only show the tray.
    pub start_to_tray: bool,
}

impl Settings {
    /// Read `<workspace>/data/settings.json`. Missing file → defaults.
    /// Malformed → defaults + `tracing::warn!`.
    pub fn load() -> Self {
        let path = crate::paths::data_dir().join("settings.json");
        let file = match std::fs::File::open(&path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "settings.json unreadable; using defaults");
                return Self::default();
            }
        };
        match serde_json::from_reader(BufReader::new(file)) {
            Ok(settings) => settings,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "settings.json malformed; using defaults");
                Self::default()
            }
        }
    }

    /// Whether *any* tray feature is active — used to decide if the tray
    /// should exist at startup.
    pub fn wants_tray(&self) -> bool {
        self.show_tray_icon
            || self.minimize_to_tray
            || self.close_to_tray
            || self.start_to_tray
    }
}
