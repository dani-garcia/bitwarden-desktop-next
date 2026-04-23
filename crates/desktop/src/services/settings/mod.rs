//! App-wide user settings, read once from `data/settings.json`.
//!
//! Behavioral branches re-read `self.settings.<field>` at event time (never
//! cached), so live changes apply without a restart. See `docs/decisions.md`.

use std::{collections::HashMap, io::BufReader};

use bitwarden_core::UserId;
use serde::{Deserialize, Serialize};

use crate::{services::preferences::UserPreferences, theme::ThemePreference};

/// Sentinel value for "follow the OS locale". Stored in the `language` field
/// when the user hasn't picked an explicit language.
pub const LANGUAGE_SYSTEM: &str = "";

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    // ── Appearance ─────────────────────────────────────────────────────────
    pub theme: ThemePreference,
    /// BCP-47 language tag (e.g. `"en"`, `"es"`) or [`LANGUAGE_SYSTEM`] for OS
    /// preferred. Kept as a plain string so new locales don't need a code
    /// change — only an `assets/i18n/<tag>/` directory.
    pub language: String,

    // ── Security ───────────────────────────────────────────────────────────
    /// Stub (not wired) — shown in the Security tab.
    pub open_at_login: bool,

    // ── Integrations (all currently stubbed) ──────────────────────────────
    pub browser_integration: bool,
    pub browser_integration_fingerprint: bool,
    pub ssh_agent: bool,
    pub duck_duck_go: bool,

    // ── Autotype & copy ───────────────────────────────────────────────────
    /// Stub — the real autotype engine hasn't been ported yet.
    pub autotype_enabled: bool,

    // ── Appearance ─────────────────────────────────────────────────────────
    /// Stub — domain icon fetching isn't wired yet.
    pub show_favicons: bool,

    // ── Advanced — Tray ────────────────────────────────────────────────────
    /// Master switch: keep the tray icon visible even when the app is in the
    /// foreground.
    pub show_tray_icon: bool,
    /// Clicking the minimize button hides the window (not the taskbar
    /// minimize). Auto-creates the tray if not already shown.
    pub minimize_to_tray: bool,
    /// Clicking the OS close button hides the window instead of exiting.
    /// Auto-creates the tray if not already shown.
    pub close_to_tray: bool,
    /// On launch, don't open the main window; only show the tray.
    pub start_to_tray: bool,

    // ── Advanced — platform (all currently stubbed) ───────────────────────
    pub always_show_dock: bool,
    pub hardware_acceleration: bool,
    pub allow_screenshots: bool,

    // ── Per-user preferences ───────────────────────────────────────────────
    /// Keyed by `UserId`. Populated lazily on first read/write via
    /// [`Settings::preferences_for`] / [`Settings::preferences_for_mut`].
    /// Persisted alongside the app-wide fields so unlock-with-PIN, clipboard
    /// delay, etc. survive across launches.
    pub user_preferences: HashMap<UserId, UserPreferences>,
}

impl Settings {
    /// Current preferences for `uid`, or defaults if the user has none saved
    /// yet. Read-only — edits land via the settings modal, which mutates
    /// its own working snapshot and pushes it back through App.
    pub fn preferences_for(&self, uid: &UserId) -> UserPreferences {
        self.user_preferences.get(uid).copied().unwrap_or_default()
    }
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

    /// Persist the current settings to `data/settings.json`. Called whenever
    /// the settings view applies a live-wired change.
    pub fn save(&self) {
        let path = crate::paths::data_dir().join("settings.json");
        let file = match std::fs::File::create(&path) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "settings.json write failed");
                return;
            }
        };
        if let Err(e) = serde_json::to_writer_pretty(file, self) {
            tracing::warn!(path = %path.display(), error = %e, "settings.json serialize failed");
        }
    }

    /// Whether *any* tray feature is active — used to decide if the tray
    /// should exist at startup.
    pub fn wants_tray(&self) -> bool {
        self.show_tray_icon || self.minimize_to_tray || self.close_to_tray || self.start_to_tray
    }
}
