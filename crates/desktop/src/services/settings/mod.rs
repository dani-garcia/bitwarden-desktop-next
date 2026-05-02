//! App-wide user settings, read once from `data/settings.json`. Behavioral
//! branches re-read `self.settings.<field>` at event time (never cached), so
//! live changes apply without a restart. See `docs/decisions.md`.

use std::{collections::HashMap, io::BufReader};

use bitwarden_core::UserId;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{services::preferences::UserPreferences, theme::ThemePreference};

/// Sentinel for "follow the OS locale". Stored in `language` when no explicit
/// language has been picked.
pub const LANGUAGE_SYSTEM: &str = "";

/// UI zoom multiplier in tenths (10 = 1.0×). Stepping is exact integer
/// arithmetic — no floating-point drift after a few +/- presses. Self-clamps
/// to `MIN..=MAX` on construction and on deserialize, so a hand-edited
/// `zoom_factor: 0` in `settings.json` snaps to `MIN` instead of bricking the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ZoomFactor(u8);

impl ZoomFactor {
    pub const DEFAULT: u8 = 10;
    pub const MIN: u8 = 5;
    pub const MAX: u8 = 30;

    pub fn new(tenths: u8) -> Self {
        Self(tenths.clamp(Self::MIN, Self::MAX))
    }

    /// Tenths value as an f32 multiplier ready for iced's `scale_factor` callback.
    pub fn scale(self) -> f32 {
        f32::from(self.0) / 10.0
    }

    /// Bump one tenth up, saturating at [`Self::MAX`]. Returns `true` if changed.
    pub fn step_in(&mut self) -> bool {
        if self.0 < Self::MAX {
            self.0 += 1;
            true
        } else {
            false
        }
    }

    /// Drop one tenth down, saturating at [`Self::MIN`]. Returns `true` if changed.
    pub fn step_out(&mut self) -> bool {
        if self.0 > Self::MIN {
            self.0 -= 1;
            true
        } else {
            false
        }
    }

    /// Restore to [`Self::DEFAULT`]. Returns `true` if changed.
    pub fn reset(&mut self) -> bool {
        if self.0 != Self::DEFAULT {
            self.0 = Self::DEFAULT;
            true
        } else {
            false
        }
    }
}

impl Default for ZoomFactor {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

impl<'de> Deserialize<'de> for ZoomFactor {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        u8::deserialize(d).map(Self::new)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    // ── Appearance ─────────────────────────────────────────────────────────
    pub theme: ThemePreference,
    /// BCP-47 language tag (e.g. `"en"`, `"es"`) or [`LANGUAGE_SYSTEM`] for OS
    /// preferred. Plain string so new locales need only an `assets/i18n/<tag>/`
    /// directory, no code change.
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

    // ── Advanced — platform (all currently stubbed) ───────────────────────
    pub always_show_dock: bool,
    pub hardware_acceleration: bool,
    pub allow_screenshots: bool,

    // ── wgpu backend cache ────────────────────────────────────────────────
    // Only meaningful when the `gpu` Cargo feature is on. Pre-existing fields
    // in settings.json are silently dropped on deserialize when the feature
    // is off (no `deny_unknown_fields`), and re-populated on the first launch
    // after re-enabling the feature.
    /// Backend wgpu used last time we reached first paint. Reused on the next
    /// launch via `WGPU_BACKEND` to skip the multi-backend enumeration walk
    /// (saves ~325 ms on a warm machine — see `select_backend` in `main.rs`).
    #[cfg(feature = "gpu")]
    pub wgpu_backend_verified: Option<String>,
    /// Set just before `iced::run`, cleared on first paint. If still set on
    /// the next launch, the previous run never reached first paint — treat
    /// the cache as poisoned and re-enumerate.
    #[cfg(feature = "gpu")]
    pub wgpu_backend_pending: Option<String>,

    // ── Per-user preferences ───────────────────────────────────────────────
    /// Keyed by `UserId`. Populated lazily; persisted alongside the app-wide
    /// fields so unlock-with-PIN, clipboard delay, etc. survive across launches.
    pub user_preferences: HashMap<UserId, UserPreferences>,

    // ── Zoom ──────────────────────────────────────────────────────────────
    /// UI scale fed into iced's per-window `scale_factor` callback. Composes
    /// multiplicatively with OS DPI. Stepped from the View → Zoom in / out / reset menu.
    pub zoom_factor: ZoomFactor,
}

impl Settings {
    /// Current preferences for `uid`, or defaults if none are saved.
    pub fn preferences_for(&self, uid: &UserId) -> UserPreferences {
        self.user_preferences.get(uid).copied().unwrap_or_default()
    }
}

impl Settings {
    /// Missing file → defaults. Malformed → defaults + `tracing::warn!`.
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

    /// Whether *any* tray feature is active — drives tray creation at startup.
    pub fn wants_tray(&self) -> bool {
        self.show_tray_icon || self.minimize_to_tray || self.close_to_tray
    }
}
