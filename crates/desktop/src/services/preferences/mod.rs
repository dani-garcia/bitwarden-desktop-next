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
    pub lock_after: LockAfter,
    pub logout_after: LogoutAfter,

    // Integrations
    pub ssh_prompt_behavior: SshPromptBehavior,

    // Autotype & copy
    pub clear_clipboard: ClearClipboardDelay,
    pub minimize_on_copy: bool,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            pin_unlock: false,
            touch_id_unlock: false,
            lock_after: LockAfter::FifteenMinutes,
            logout_after: LogoutAfter::Never,
            ssh_prompt_behavior: SshPromptBehavior::Always,
            clear_clipboard: ClearClipboardDelay::ThirtySeconds,
            minimize_on_copy: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LockAfter {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    ThirtyMinutes,
    OneHour,
    FourHours,
    Never,
}

impl LockAfter {
    pub const ALL: &'static [Self] = &[
        Self::OneMinute,
        Self::FiveMinutes,
        Self::FifteenMinutes,
        Self::ThirtyMinutes,
        Self::OneHour,
        Self::FourHours,
        Self::Never,
    ];

    pub fn label(self) -> String {
        match self {
            Self::OneMinute => fl!("settings-duration-minutes", n = 1),
            Self::FiveMinutes => fl!("settings-duration-minutes", n = 5),
            Self::FifteenMinutes => fl!("settings-duration-minutes", n = 15),
            Self::ThirtyMinutes => fl!("settings-duration-minutes", n = 30),
            Self::OneHour => fl!("settings-duration-hours", n = 1),
            Self::FourHours => fl!("settings-duration-hours", n = 4),
            Self::Never => fl!("settings-duration-never"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LogoutAfter {
    OneHour,
    FourHours,
    EightHours,
    TwentyFourHours,
    Never,
}

impl LogoutAfter {
    pub const ALL: &'static [Self] = &[
        Self::OneHour,
        Self::FourHours,
        Self::EightHours,
        Self::TwentyFourHours,
        Self::Never,
    ];

    pub fn label(self) -> String {
        match self {
            Self::OneHour => fl!("settings-duration-hours", n = 1),
            Self::FourHours => fl!("settings-duration-hours", n = 4),
            Self::EightHours => fl!("settings-duration-hours", n = 8),
            Self::TwentyFourHours => fl!("settings-duration-hours", n = 24),
            Self::Never => fl!("settings-duration-never"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClearClipboardDelay {
    Never,
    TenSeconds,
    TwentySeconds,
    ThirtySeconds,
    OneMinute,
    TwoMinutes,
    FiveMinutes,
}

impl ClearClipboardDelay {
    pub const ALL: &'static [Self] = &[
        Self::Never,
        Self::TenSeconds,
        Self::TwentySeconds,
        Self::ThirtySeconds,
        Self::OneMinute,
        Self::TwoMinutes,
        Self::FiveMinutes,
    ];

    pub fn label(self) -> String {
        match self {
            Self::Never => fl!("settings-duration-never"),
            Self::TenSeconds => fl!("settings-duration-seconds", n = 10),
            Self::TwentySeconds => fl!("settings-duration-seconds", n = 20),
            Self::ThirtySeconds => fl!("settings-duration-seconds", n = 30),
            Self::OneMinute => fl!("settings-duration-minutes", n = 1),
            Self::TwoMinutes => fl!("settings-duration-minutes", n = 2),
            Self::FiveMinutes => fl!("settings-duration-minutes", n = 5),
        }
    }

    /// `None` disables auto-clear entirely.
    pub fn as_duration(self) -> Option<Duration> {
        match self {
            Self::Never => None,
            Self::TenSeconds => Some(Duration::from_secs(10)),
            Self::TwentySeconds => Some(Duration::from_secs(20)),
            Self::ThirtySeconds => Some(Duration::from_secs(30)),
            Self::OneMinute => Some(Duration::from_secs(60)),
            Self::TwoMinutes => Some(Duration::from_secs(120)),
            Self::FiveMinutes => Some(Duration::from_secs(300)),
        }
    }
}

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
