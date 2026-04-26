//! Section cards shared by every cipher type:
//! - `section_label` heading helper
//! - `item_details_card` (name, favorite, folder, organization, collections)
//! - `additional_options_card` (notes + master-password reprompt toggle)
//! - `custom_fields_card` (text / hidden / boolean fields, add/remove)

use bitwarden_vault::{CipherRepromptType, FieldType, FieldView};
use iced::{
    Alignment, Element, Fill, Length,
    widget::{checkbox, column, container, row, text, text_editor},
};

use crate::{
    components::{
        buttons, icons,
        inputs::{reveal_text_field, select_field, text_field},
    },
    fl,
    theme::{AppColors, AppTheme},
    views::vault::widgets::{
        cipher_edit::{CipherEditMessage, CipherForm, selectors},
        field_helpers::{card_with_margin, styled_card},
    },
};

pub(in super::super) fn section_label<'a>(
    label: impl Into<String>,
    colors: &AppColors,
) -> Element<'a, CipherEditMessage, AppTheme> {
    text(label.into())
        .size(14)
        .color(colors.text_primary)
        .into()
}

pub(in super::super) fn item_details_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherEditMessage, AppTheme> {
    let mut rows: Vec<Element<'a, CipherEditMessage, AppTheme>> = Vec::new();

    rows.push(text_field(
        fl!("form-name"),
        &form.modified.name,
        CipherEditMessage::NameChanged,
        None,
        form.saving,
        colors,
    ));

    let favorite_checkbox = checkbox(form.modified.favorite)
        .label(fl!("form-favorite"))
        .on_toggle(|_| CipherEditMessage::FavoriteToggled)
        .size(18)
        .spacing(8);

    rows.push(favorite_checkbox.into());

    // Folder dropdown (personal vault only — orgs own their own folder concept)
    rows.push(selectors::folder_selector(form, colors));

    if !form.organizations.is_empty() {
        rows.push(selectors::org_selector(form, colors));

        if form.modified.organization_id.is_some() {
            rows.push(selectors::collections_selector(form, colors));
        }
    }

    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

pub(in super::super) fn additional_options_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherEditMessage, AppTheme> {
    // Multi-line notes field — `text_editor` grows with content between
    // `min_height` and `max_height`. Border/label come from `field_frame`
    // so the visual matches the single-line inputs; we strip the editor's
    // own border via `text_editor::Catalog`'s default style.
    let mut notes_editor = text_editor(&form.notes_content)
        .padding([10, 12])
        .height(Length::Shrink)
        .min_height(80.0)
        .max_height(600.0);
    if !form.saving {
        notes_editor = notes_editor.on_action(CipherEditMessage::NotesAction);
    }
    let notes =
        crate::components::inputs::field_frame(fl!("form-notes"), notes_editor.into(), colors);

    let reprompt_checkbox = checkbox(matches!(
        form.modified.reprompt,
        CipherRepromptType::Password
    ))
    .label(fl!("form-reprompt"))
    .on_toggle(|_| CipherEditMessage::RepromptToggled)
    .size(18)
    .spacing(8);

    let rows: Vec<Element<'a, CipherEditMessage, AppTheme>> = vec![notes, reprompt_checkbox.into()];

    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

pub(in super::super) fn custom_fields_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherEditMessage, AppTheme> {
    let mut rows: Vec<Element<'a, CipherEditMessage, AppTheme>> = Vec::new();

    let fields = form.modified.fields.as_deref().unwrap_or(&[]);

    if fields.is_empty() {
        rows.push(
            text(fl!("form-custom-field-empty"))
                .size(14)
                .color(colors.text_muted)
                .into(),
        );
    } else {
        for (idx, f) in fields.iter().enumerate() {
            rows.push(custom_field_row(idx, f, form, colors));
        }
    }

    let add_btn = buttons::secondary(
        row![
            icons::PLUS.render(14.0, colors.accent),
            text(fl!("form-add-custom-field")).size(14),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .on_press(CipherEditMessage::CustomFieldAdded)
    .padding([6, 12]);
    rows.push(add_btn.into());

    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

fn custom_field_row<'a>(
    idx: usize,
    f: &'a FieldView,
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherEditMessage, AppTheme> {
    let type_picker: Element<'a, CipherEditMessage, AppTheme> = container(select_field(
        fl!("form-custom-field-type"),
        Some(f.r#type),
        vec![FieldType::Text, FieldType::Hidden, FieldType::Boolean],
        |ty: &FieldType| match ty {
            FieldType::Text => fl!("form-custom-field-type-text"),
            FieldType::Hidden => fl!("form-custom-field-type-hidden"),
            FieldType::Boolean => fl!("form-custom-field-type-boolean"),
            FieldType::Linked => fl!("form-custom-field-type-linked"),
        },
        move |ty| CipherEditMessage::CustomFieldTypeSelected(idx, ty),
        colors,
    ))
    .width(140)
    .into();

    let name_input = text_field(
        fl!("form-custom-field-name"),
        f.name.as_deref().unwrap_or(""),
        move |s| CipherEditMessage::CustomFieldNameChanged(idx, s),
        None,
        form.saving,
        colors,
    );

    let value_widget: Element<'a, CipherEditMessage, AppTheme> = match f.r#type {
        FieldType::Text => text_field(
            fl!("form-custom-field-value"),
            f.value.as_deref().unwrap_or(""),
            move |s| CipherEditMessage::CustomFieldValueChanged(idx, s),
            None,
            form.saving,
            colors,
        ),
        FieldType::Hidden => reveal_text_field(
            fl!("form-custom-field-value"),
            f.value.as_deref().unwrap_or(""),
            move |s| CipherEditMessage::CustomFieldValueChanged(idx, s),
            form.saving,
            colors,
        ),
        FieldType::Boolean => {
            let checked = matches!(f.value.as_deref(), Some("true"));
            checkbox(checked)
                .label(fl!("form-custom-field-enabled"))
                .on_toggle(move |_| CipherEditMessage::CustomFieldBoolToggled(idx))
                .size(18)
                .spacing(8)
                .into()
        }
        FieldType::Linked => text(fl!("form-custom-field-linked-unsupported"))
            .size(12)
            .color(colors.text_muted)
            .into(),
    };

    let remove_btn = buttons::ghost_icon(
        icons::BWI_TRASH.render(18.0, colors.titlebar_close_hover),
        colors.item_hover,
    )
    .on_press(CipherEditMessage::CustomFieldRemoved(idx))
    .padding([6, 6]);

    row![
        type_picker,
        container(name_input).width(Fill),
        container(value_widget).width(Fill),
        remove_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}
