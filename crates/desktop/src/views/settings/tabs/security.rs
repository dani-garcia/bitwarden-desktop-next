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
    super::{SettingChange, SettingsSnapshot},
    SECTION_GAP_PX, setting_checkbox, settings_section,
};

pub fn view<'a>(
    snap: &'a SettingsSnapshot,
    colors: &'a AppColors,
) -> Element<'a, SettingChange, AppTheme> {
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

    let lock_on_system_lock = setting_checkbox(
        snap.prefs.lock_on_system_lock,
        fl!("settings-security-lock-on-system-lock"),
        SettingChange::LockOnSystemLock,
    );

    column![
        settings_section(
            fl!("settings-security-access-options"),
            column![open_at_login, pin, touch].spacing(10),
            colors,
        ),
        Space::new().height(SECTION_GAP_PX),
        settings_section(
            fl!("settings-security-session-timeout"),
            column![
                column![lock_after, logout_after].spacing(16),
                Space::new().height(10),
                lock_on_system_lock,
            ],
            colors,
        ),
    ]
    .width(Fill)
    .into()
}
