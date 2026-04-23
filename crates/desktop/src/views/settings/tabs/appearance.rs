use iced::{
    Element, Fill,
    widget::{Space, column},
};

use crate::{
    components::inputs,
    fl,
    services::{i18n, settings::LANGUAGE_SYSTEM},
    theme::{AppColors, AppTheme, ThemePreference},
};

use super::{
    super::{SettingChange, SettingsSnapshot, section_heading},
    setting_checkbox,
};

pub fn view<'a>(
    snap: &'a SettingsSnapshot,
    colors: &'a AppColors,
) -> Element<'a, SettingChange, AppTheme> {
    let theme_heading = section_heading(fl!("settings-appearance-theme-heading"), colors);

    let theme_field = inputs::select_field(
        fl!("settings-appearance-theme"),
        Some(snap.settings.theme),
        vec![
            ThemePreference::System,
            ThemePreference::Light,
            ThemePreference::Dark,
        ],
        |v: &ThemePreference| match v {
            ThemePreference::System => fl!("settings-appearance-theme-system"),
            ThemePreference::Light => fl!("settings-appearance-theme-light"),
            ThemePreference::Dark => fl!("settings-appearance-theme-dark"),
        },
        SettingChange::Theme,
        colors,
    );

    // Language list is derived from `assets/i18n/` at runtime — adding a new
    // locale is a filesystem change, not a code change.
    let mut languages: Vec<String> = vec![LANGUAGE_SYSTEM.to_string()];
    for lang in i18n::available_languages() {
        languages.push(lang.to_string());
    }
    let selected_lang = if languages.iter().any(|l| l == &snap.settings.language) {
        Some(snap.settings.language.clone())
    } else {
        Some(LANGUAGE_SYSTEM.to_string())
    };

    let language_field = inputs::select_field(
        fl!("settings-appearance-language"),
        selected_lang,
        languages,
        |tag: &String| {
            if tag == LANGUAGE_SYSTEM {
                fl!("settings-appearance-language-system")
            } else {
                i18n::language_label(tag)
            }
        },
        SettingChange::Language,
        colors,
    );

    let display_heading = section_heading(fl!("settings-appearance-display-heading"), colors);

    let favicons = setting_checkbox(
        snap.settings.show_favicons,
        fl!("settings-appearance-show-favicons"),
        SettingChange::ShowFavicons,
    );

    column![
        theme_heading,
        Space::new().height(8),
        column![theme_field, language_field].spacing(16),
        Space::new().height(24),
        display_heading,
        Space::new().height(8),
        favicons,
    ]
    .width(Fill)
    .into()
}
