use iced::{
    Element, Fill,
    widget::{Space, column},
};

use crate::{
    components::inputs,
    fl,
    services::preferences::ClearClipboardDelay,
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
    let autotype_heading = section_heading(fl!("settings-autotype-heading"), colors);

    let autotype_enabled = setting_checkbox(
        snap.settings.autotype_enabled,
        fl!("settings-autotype-enable"),
        SettingChange::AutotypeEnabled,
    );

    let clipboard_heading = section_heading(fl!("settings-clipboard-heading"), colors);

    let clear_clipboard = inputs::select_field(
        fl!("settings-clipboard-clear-after"),
        Some(snap.prefs.clear_clipboard),
        ClearClipboardDelay::ALL.to_vec(),
        |v: &ClearClipboardDelay| v.label(),
        SettingChange::ClearClipboard,
        colors,
    );

    let minimize_on_copy = setting_checkbox(
        snap.prefs.minimize_on_copy,
        fl!("settings-clipboard-minimize-on-copy"),
        SettingChange::MinimizeOnCopy,
    );

    column![
        autotype_heading,
        Space::new().height(8),
        autotype_enabled,
        Space::new().height(24),
        clipboard_heading,
        Space::new().height(8),
        column![clear_clipboard, minimize_on_copy].spacing(16),
    ]
    .width(Fill)
    .into()
}
