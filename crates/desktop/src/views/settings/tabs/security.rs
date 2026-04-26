use iced::{
    Element, Fill,
    widget::{Space, column},
};

use crate::{
    components::inputs,
    fl,
    services::preferences::{DurationSecs, LOCK_AFTER_PRESETS, LOGOUT_AFTER_PRESETS},
    theme::{AppColors, AppTheme},
};

use super::{
    super::{SettingChange, SettingsSnapshot, section_heading},
    setting_checkbox,
};

pub fn view<'a>(
    snap: &'a SettingsSnapshot,
    colors: &'a AppColors,
) -> Element<'a, SettingChange, AppTheme> {
    let access_heading = section_heading(fl!("settings-security-access-options"), colors);

    let open_at_login = setting_checkbox(
        snap.settings.open_at_login,
        fl!("settings-security-open-at-login"),
        SettingChange::OpenAtLogin,
    );
    let pin = setting_checkbox(
        snap.prefs.pin_unlock,
        fl!("settings-security-unlock-pin"),
        SettingChange::PinUnlock,
    );
    let touch = setting_checkbox(
        snap.prefs.touch_id_unlock,
        fl!("settings-security-unlock-touch"),
        SettingChange::TouchIdUnlock,
    );

    let timeout_heading = section_heading(fl!("settings-security-session-timeout"), colors);

    let lock_after = inputs::select_field(
        fl!("settings-security-lock-after"),
        Some(snap.prefs.lock_after),
        LOCK_AFTER_PRESETS.to_vec(),
        |v: &DurationSecs| v.label(),
        SettingChange::LockAfter,
        colors,
    );

    let logout_after = inputs::select_field(
        fl!("settings-security-logout-after"),
        Some(snap.prefs.logout_after),
        LOGOUT_AFTER_PRESETS.to_vec(),
        |v: &DurationSecs| v.label(),
        SettingChange::LogoutAfter,
        colors,
    );

    column![
        access_heading,
        Space::new().height(8),
        column![open_at_login, pin, touch].spacing(10),
        Space::new().height(24),
        timeout_heading,
        Space::new().height(8),
        column![lock_after, logout_after].spacing(16),
    ]
    .spacing(0)
    .width(Fill)
    .into()
}
