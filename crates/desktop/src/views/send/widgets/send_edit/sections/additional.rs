//! "Additional options" card — view limit, hide-email flag, private notes.

use iced::{
    Alignment, Element, Length,
    widget::{checkbox, column, row, text, text_editor},
};

use crate::{
    components::{buttons, icons, inputs},
    fl,
    theme::{AppColors, AppTheme},
    views::send::widgets::send_edit::{SendForm, SendEditMessage},
};

use super::shared::card_section;

pub(in super::super) fn additional_options_card<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Element<'a, SendEditMessage, AppTheme> {
    let mut items: Vec<Element<'a, SendEditMessage, AppTheme>> = Vec::new();
    items.push(limit_views_field(form, colors));
    items.push(views_left_hint(form, colors).unwrap_or_else(|| {
        text(fl!("send-form-limit-views-hint"))
            .size(12)
            .color(colors.text_secondary)
            .into()
    }));
    items.push(
        checkbox(form.hide_email)
            .label(fl!("send-form-hide-email"))
            .on_toggle(SendEditMessage::HideEmailToggled)
            .size(18)
            .spacing(8)
            .into(),
    );
    items.push(notes_field(form, colors));

    card_section(fl!("send-form-additional-heading"), items, colors)
}

// ── Fields ────────────────────────────────────────────────────────────────

fn limit_views_field<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Element<'a, SendEditMessage, AppTheme> {
    let input = inputs::bare_text_input(&form.max_access_count_raw)
        .on_input(SendEditMessage::MaxAccessCountChanged);
    let inc = buttons::ghost_icon(
        icons::CHEVRON_UP.render(11.0, colors.text_secondary),
        colors.item_hover,
    )
    .on_press(SendEditMessage::MaxAccessCountIncrement)
    .padding([2, 6]);
    let dec = buttons::ghost_icon(
        icons::CHEVRON_DOWN.render(11.0, colors.text_secondary),
        colors.item_hover,
    )
    .on_press(SendEditMessage::MaxAccessCountDecrement)
    .padding([2, 6]);

    let steppers = column![inc, dec].spacing(0);
    let row_el = row![input, steppers].align_y(Alignment::Center);
    inputs::field_frame(fl!("send-form-limit-views"), row_el.into(), colors)
}

fn views_left_hint<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Option<Element<'a, SendEditMessage, AppTheme>> {
    let left = form.views_left()?;
    Some(
        text(fl!("send-form-views-left", count = left.to_string()))
            .size(12)
            .color(colors.text_secondary)
            .into(),
    )
}

fn notes_field<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Element<'a, SendEditMessage, AppTheme> {
    let mut editor = text_editor(&form.notes_content)
        .padding([10, 12])
        .height(Length::Shrink)
        .min_height(80.0)
        .max_height(240.0);
    if !form.saving {
        editor = editor.on_action(SendEditMessage::NotesAction);
    }
    inputs::field_frame(fl!("send-form-private-note"), editor.into(), colors)
}
