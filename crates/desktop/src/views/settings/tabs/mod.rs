pub(super) mod advanced;
pub(super) mod appearance;
pub(super) mod autotype;
pub(super) mod integrations;
pub(super) mod security;

use iced::widget::{Checkbox, checkbox};

use crate::theme::AppTheme;

use super::SettingChange;

/// Standard labeled toggle used by every settings tab. Centralises the
/// checkbox's size / spacing so the look stays uniform across tabs.
pub(super) fn setting_checkbox<'a, F>(
    checked: bool,
    label: impl Into<String>,
    on_toggle: F,
) -> Checkbox<'a, SettingChange, AppTheme>
where
    F: 'a + Fn(bool) -> SettingChange,
{
    checkbox(checked)
        .label(label.into())
        .size(18)
        .spacing(8)
        .on_toggle(on_toggle)
}
