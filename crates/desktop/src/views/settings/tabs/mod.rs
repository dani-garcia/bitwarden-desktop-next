pub(super) mod advanced;
pub(super) mod appearance;
pub(super) mod autotype;
pub(super) mod integrations;
pub(super) mod security;

pub(super) use crate::components::labeled_checkbox as setting_checkbox;

use iced::{
    Element, Fill,
    widget::{Space, column},
};

use crate::{
    components::section_heading,
    theme::{AppColors, AppTheme},
};

use super::SettingChange;

/// Vertical gap between sections in a settings tab. Pair with
/// [`settings_section`] when stacking multiple groups in a tab body.
pub(super) const SECTION_GAP_PX: f32 = 24.0;

/// One settings-tab section: a [`section_heading`], an 8 px breather,
/// then the caller-supplied fields column. Pure layout — caller controls
/// the fields' own spacing.
pub(super) fn settings_section<'a>(
    heading: impl Into<String>,
    fields: impl Into<Element<'a, SettingChange, AppTheme>>,
    colors: &'a AppColors,
) -> Element<'a, SettingChange, AppTheme> {
    column![
        section_heading(heading.into(), colors),
        Space::new().height(8),
        fields.into(),
    ]
    .width(Fill)
    .into()
}
