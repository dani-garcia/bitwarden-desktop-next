//! Per-user preferences. Held inside [`crate::services::settings::Settings`] as
//! a `HashMap<UserId, UserPreferences>`, persisted in `data/settings.json`.
//!
//! Clipboard-clear delay is pushed to `ClipboardManager` on user switch so the
//! app-global clipboard reflects the active user's preference.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::fl;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct UserPreferences {
    // Security
    pub pin_unlock: bool,
    pub touch_id_unlock: bool,
    pub lock_after: DurationSecs,
    pub logout_after: DurationSecs,
    pub lock_on_system_lock: bool,

    // Integrations
    pub ssh_prompt_behavior: SshPromptBehavior,

    // Autotype & copy
    pub clear_clipboard: DurationSecs,
    pub minimize_on_copy: bool,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            pin_unlock: false,
            touch_id_unlock: false,
            lock_after: DurationSecs(900),
            logout_after: DurationSecs::NEVER,
            lock_on_system_lock: false,
            ssh_prompt_behavior: SshPromptBehavior::Always,
            clear_clipboard: DurationSecs(30),
            minimize_on_copy: false,
        }
    }
}

/// A duration expressed as a whole number of seconds, with `0` meaning "never".
///
/// Used for every time-valued preference (lock-after, logout-after,
/// clear-clipboard) so the dropdown choices are a presentation concern — adding
/// a new preset is a one-line edit to the relevant `*_PRESETS` constant rather
/// than an enum variant churn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct DurationSecs(pub u32);

impl DurationSecs {
    pub const NEVER: Self = Self(0);

    /// Localized label. Presets must be 0, a multiple of 3600, or a multiple
    /// of 60 — arbitrary values fall back to a raw seconds count, which renders
    /// correctly but produces awkward labels like "90 seconds" instead of
    /// "1 minute 30 seconds".
    pub fn label(self) -> String {
        let s = self.0;
        if s == 0 {
            fl!("settings-duration-never")
        } else if s.is_multiple_of(3600) {
            let count = s / 3600;
            fl!("settings-duration-hours", n = count)
        } else if s.is_multiple_of(60) {
            let count = s / 60;
            fl!("settings-duration-minutes", n = count)
        } else {
            debug_assert!(
                false,
                "DurationSecs preset {s} is not a clean unit boundary"
            );
            fl!("settings-duration-seconds", n = s)
        }
    }

    /// `None` disables the timeout entirely (`Self::NEVER`).
    pub fn as_duration(self) -> Option<Duration> {
        (self.0 != 0).then(|| Duration::from_secs(self.0 as u64))
    }
}

pub const LOCK_AFTER_PRESETS: &[DurationSecs] = &[
    DurationSecs(60),
    DurationSecs(300),
    DurationSecs(900),
    DurationSecs(1800),
    DurationSecs(3600),
    DurationSecs(14400),
    DurationSecs::NEVER,
];

pub const LOGOUT_AFTER_PRESETS: &[DurationSecs] = &[
    DurationSecs(3600),
    DurationSecs(14400),
    DurationSecs(28800),
    DurationSecs(86400),
    DurationSecs::NEVER,
];

pub const CLEAR_CLIPBOARD_PRESETS: &[DurationSecs] = &[
    DurationSecs::NEVER,
    DurationSecs(10),
    DurationSecs(20),
    DurationSecs(30),
    DurationSecs(60),
    DurationSecs(120),
    DurationSecs(300),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SshPromptBehavior {
    Always,
    Never,
    RememberUntilLock,
}

impl SshPromptBehavior {
    pub const ALL: &'static [Self] = &[Self::Always, Self::Never, Self::RememberUntilLock];

    pub fn label(self) -> String {
        match self {
            Self::Always => fl!("settings-ssh-prompt-always"),
            Self::Never => fl!("settings-ssh-prompt-never"),
            Self::RememberUntilLock => fl!("settings-ssh-prompt-remember"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_duration_never() {
        assert_eq!(DurationSecs::NEVER.as_duration(), None);
    }

    #[test]
    fn as_duration_nonzero() {
        assert_eq!(
            DurationSecs(30).as_duration(),
            Some(Duration::from_secs(30))
        );
    }

    #[test]
    fn serde_roundtrip_is_bare_integer() {
        let json = serde_json::to_string(&DurationSecs(900)).unwrap();
        assert_eq!(json, "900");
        let parsed: DurationSecs = serde_json::from_str("900").unwrap();
        assert_eq!(parsed, DurationSecs(900));
    }
}
