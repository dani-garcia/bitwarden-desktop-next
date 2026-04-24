pub(super) mod passphrase;
pub(super) mod password;
pub(super) mod username;

use iced::widget::{Checkbox, checkbox};

use crate::theme::AppTheme;

use super::GeneratorMessage;

/// Standard labeled toggle used across the generator tabs.
pub(super) fn tab_checkbox<'a, F>(
    checked: bool,
    label: impl Into<String>,
    on_toggle: F,
) -> Checkbox<'a, GeneratorMessage, AppTheme>
where
    F: 'a + Fn(bool) -> GeneratorMessage,
{
    checkbox(checked)
        .label(label.into())
        .size(18)
        .spacing(8)
        .on_toggle(on_toggle)
}
