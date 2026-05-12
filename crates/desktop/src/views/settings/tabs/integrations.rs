use iced::{
    Element, Fill,
    widget::{Space, column},
};

use crate::{
    components::inputs,
    fl,
    services::preferences::SshPromptBehavior,
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
    let browser = setting_checkbox(
        snap.settings.browser_integration,
        fl!("settings-integrations-browser-enable"),
        SettingChange::BrowserIntegration,
    );
    let browser_fp = setting_checkbox(
        snap.settings.browser_integration_fingerprint,
        fl!("settings-integrations-browser-fingerprint"),
        SettingChange::BrowserIntegrationFingerprint,
    );

    let ssh = setting_checkbox(
        snap.settings.ssh_agent,
        fl!("settings-integrations-ssh-enable"),
        SettingChange::SshAgent,
    );
    let ssh_prompt = inputs::select_field(
        fl!("settings-integrations-ssh-prompt"),
        Some(snap.prefs.ssh_prompt_behavior),
        SshPromptBehavior::ALL.to_vec(),
        |v: &SshPromptBehavior| v.label(),
        SettingChange::SshPromptBehavior,
        colors,
    );

    let ddg = setting_checkbox(
        snap.settings.duck_duck_go,
        fl!("settings-integrations-duckduckgo"),
        SettingChange::DuckDuckGo,
    );

    column![
        settings_section(
            fl!("settings-integrations-browser"),
            column![browser, browser_fp].spacing(10),
            colors,
        ),
        Space::new().height(SECTION_GAP_PX),
        settings_section(
            fl!("settings-integrations-ssh"),
            column![ssh, ssh_prompt].spacing(16),
            colors,
        ),
        Space::new().height(SECTION_GAP_PX),
        settings_section(fl!("settings-integrations-other"), ddg, colors),
    ]
    .width(Fill)
    .into()
}
