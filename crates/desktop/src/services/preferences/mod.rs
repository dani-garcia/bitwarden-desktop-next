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

    /// Localized label. Decomposes into hours / minutes / seconds and joins
    /// the non-zero components with spaces, so `0` reads as "Never", `3600`
    /// as "1 hour", and `3671` as "1 hour 1 minute 11 seconds". Each
    /// component goes through Fluent's `$n` selector so the right
    /// singular/plural form lands in every locale.
    pub fn label(self) -> String {
        let total = self.0;
        if total == 0 {
            return fl!("settings-duration-never");
        }
        let hours = total / 3600;
        let minutes = (total % 3600) / 60;
        let seconds = total % 60;

        let mut parts = Vec::with_capacity(3);
        if hours > 0 {
            parts.push(fl!("settings-duration-hours", n = hours));
        }
        if minutes > 0 {
            parts.push(fl!("settings-duration-minutes", n = minutes));
        }
        if seconds > 0 {
            parts.push(fl!("settings-duration-seconds", n = seconds));
        }
        parts.join(" ")
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
