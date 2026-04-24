use iced::{
    Element, Fill,
    widget::{Space, column},
};

use crate::{
    components::inputs,
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

    let mut col = column![type_picker, Space::new().height(12)].width(Fill);

    match form.kind {
        UsernameKind::Word => {
            col = col.push(tab_checkbox(
                form.capitalize,
                fl!("generator-username-capitalize"),
                GeneratorMessage::ToggleUsernameCapitalize,
            ));
            col = col.push(Space::new().height(8));
            col = col.push(tab_checkbox(
                form.include_number,
                fl!("generator-username-include-number"),
                GeneratorMessage::ToggleUsernameIncludeNumber,
            ));
        }
        UsernameKind::Subaddress => {
            col = col.push(inputs::text_field(
                fl!("generator-username-email"),
                &form.email,
                GeneratorMessage::SetEmail,
                None,
                false,
                colors,
            ));
        }
        UsernameKind::Catchall => {
            col = col.push(inputs::text_field(
                fl!("generator-username-domain"),
                &form.domain,
                GeneratorMessage::SetDomain,
                None,
                false,
                colors,
            ));
        }
    }

    col.into()
}
