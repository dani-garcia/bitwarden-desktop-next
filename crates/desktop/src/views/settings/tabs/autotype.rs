use iced::{
    Element, Fill,
    widget::{Space, column},
};

use crate::{
    components::inputs,
    fl,
    services::preferences::{CLEAR_CLIPBOARD_PRESETS, DurationSecs},
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
    let autotype_enabled = setting_checkbox(
        snap.settings.autotype_enabled,
        fl!("settings-autotype-enable"),
        SettingChange::AutotypeEnabled,
    );

    let clear_clipboard = inputs::select_field(
        fl!("settings-clipboard-clear-after"),
        Some(snap.prefs.clear_clipboard),
        CLEAR_CLIPBOARD_PRESETS.to_vec(),
        |v: &DurationSecs| v.label(),
        SettingChange::ClearClipboard,
        colors,
    );

    let minimize_on_copy = setting_checkbox(
        snap.prefs.minimize_on_copy,
        fl!("settings-clipboard-minimize-on-copy"),
        SettingChange::MinimizeOnCopy,
    );

    column![
        settings_section(fl!("settings-autotype-heading"), autotype_enabled, colors),
        Space::new().height(SECTION_GAP_PX),
        settings_section(
            fl!("settings-clipboard-heading"),
            column![clear_clipboard, minimize_on_copy].spacing(16),
            colors,
        ),
    ]
    .width(Fill)
    .into()
}
