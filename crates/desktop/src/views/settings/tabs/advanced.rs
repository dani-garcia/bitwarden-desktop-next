use iced::{
    Element, Fill,
    widget::{Space, column},
};

use crate::{
    fl,
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
    let tray_heading = section_heading(fl!("settings-advanced-tray"), colors);

    let tray_enabled = setting_checkbox(
        snap.settings.show_tray_icon,
        fl!("settings-advanced-tray-enable"),
        SettingChange::TrayEnabled,
    );
    let min_to_tray = setting_checkbox(
        snap.settings.minimize_to_tray,
        fl!("settings-advanced-minimize-to-tray"),
        SettingChange::MinimizeToTray,
    );
    let close_to_tray = setting_checkbox(
        snap.settings.close_to_tray,
        fl!("settings-advanced-close-to-tray"),
        SettingChange::CloseToTray,
    );

    let platform_heading = section_heading(fl!("settings-advanced-platform"), colors);

    let always_dock = setting_checkbox(
        snap.settings.always_show_dock,
        fl!("settings-advanced-always-show-dock"),
        SettingChange::AlwaysShowDock,
    );
    let hardware_accel = setting_checkbox(
        snap.settings.hardware_acceleration,
        fl!("settings-advanced-hardware-acceleration"),
        SettingChange::HardwareAcceleration,
    );
    let allow_screenshots = setting_checkbox(
        snap.settings.allow_screenshots,
        fl!("settings-advanced-allow-screenshots"),
        SettingChange::AllowScreenshots,
    );

    column![
        tray_heading,
        Space::new().height(8),
        column![tray_enabled, min_to_tray, close_to_tray].spacing(10),
        Space::new().height(24),
        platform_heading,
        Space::new().height(8),
        column![always_dock, hardware_accel, allow_screenshots].spacing(10),
    ]
    .width(Fill)
    .into()
}
