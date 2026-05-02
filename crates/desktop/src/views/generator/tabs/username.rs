use iced::{
    Element, Fill,
    widget::{Space, column},
};

use crate::{
    components::{self, inputs},
    fl,
    theme::{AppColors, AppTheme},
};

use super::{
    super::{GeneratorMessage, UsernameForm, UsernameKind},
    tab_checkbox,
};

pub(in super::super) fn view<'a>(
    form: &'a UsernameForm,
    colors: &'a AppColors,
) -> Element<'a, GeneratorMessage, AppTheme> {
    let type_picker = inputs::select_field(
        fl!("generator-username-type"),
        Some(form.kind),
        UsernameKind::ALL.to_vec(),
        |k: &UsernameKind| k.to_string(),
        GeneratorMessage::SelectUsernameKind,
        colors,
    );

    let sub_fields: Element<'a, GeneratorMessage, AppTheme> = match form.kind {
        UsernameKind::Word => column![
            tab_checkbox(
                form.capitalize,
                fl!("generator-username-capitalize"),
                GeneratorMessage::ToggleUsernameCapitalize,
            ),
            Space::new().height(8),
            tab_checkbox(
                form.include_number,
                fl!("generator-username-include-number"),
                GeneratorMessage::ToggleUsernameIncludeNumber,
            ),
        ]
        .width(Fill)
        .into(),
        UsernameKind::Subaddress => {
            inputs::text_field(fl!("generator-username-email"), &form.email, colors)
                .on_input(GeneratorMessage::SetEmail)
                .into()
        }
        UsernameKind::Catchall => {
            inputs::text_field(fl!("generator-username-domain"), &form.domain, colors)
                .on_input(GeneratorMessage::SetDomain)
                .into()
        }
    };

    let combined_card = components::styled_card(
        column![type_picker, Space::new().height(12), sub_fields].width(Fill),
    );

    column![components::card_with_margin(combined_card)]
        .width(Fill)
        .into()
}
