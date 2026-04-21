use iced::{
    Element, Fill,
    widget::{Space, column},
};

use crate::{
    components::inputs,
    fl,
    preferences::SshPromptBehavior,
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
    let browser_heading = section_heading(fl!("settings-integrations-browser"), colors);

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

    let ssh_heading = section_heading(fl!("settings-integrations-ssh"), colors);

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

    let other_heading = section_heading(fl!("settings-integrations-other"), colors);

    let ddg = setting_checkbox(
        snap.settings.duck_duck_go,
        fl!("settings-integrations-duckduckgo"),
        SettingChange::DuckDuckGo,
    );

    column![
        browser_heading,
        Space::new().height(8),
        column![browser, browser_fp].spacing(10),
        Space::new().height(24),
        ssh_heading,
        Space::new().height(8),
        column![ssh, ssh_prompt].spacing(16),
        Space::new().height(24),
        other_heading,
        Space::new().height(8),
        ddg,
    ]
    .width(Fill)
    .into()
}
