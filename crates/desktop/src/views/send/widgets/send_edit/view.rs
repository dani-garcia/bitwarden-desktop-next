//! Top-level send form view: header + scrollable body (two cards) + footer.
//! The cards themselves live in [`super::sections`].

use bitwarden_send::SendType;
use iced::{
    Alignment, Border, Element, Fill,
    widget::{Space, column, container, row, scrollable, text},
};

use crate::{
    components::{buttons, icons, separator_h},
    fl,
    theme::{AppColors, AppTheme},
};

use super::{
    message::SendEditMessage,
    sections::{additional::additional_options_card, details::details_card},
    state::SendForm,
};

pub fn view<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
    top_radius: f32,
) -> Element<'a, SendEditMessage, AppTheme> {
    let is_new = form.id.is_none();

    let title = match (is_new, form.send_type) {
        (true, SendType::Text) => fl!("send-form-title-new-text"),
        (true, SendType::File) => fl!("send-form-title-new-file"),
        (false, SendType::Text) => fl!("send-form-title-edit-text"),
        (false, SendType::File) => fl!("send-form-title-edit-file"),
    };

    let header = row![
        text(title)
            .size(18)
            .color(colors.text_primary)
            .font(crate::APP_FONT_BOLD)
            .width(Fill),
        buttons::ghost_icon(
            icons::BWI_CLOSE.render(16.0, colors.text_secondary),
            colors.item_hover,
        )
        .on_press(SendEditMessage::CancelPressed)
        .padding([6, 8]),
    ]
    .align_y(Alignment::Center);

    let header_container = container(header)
        .padding([16, 20])
        .style(move |theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.card_bg)
                .border(Border::default().rounded(iced::border::top(top_radius)))
        });

    let body: Element<'a, SendEditMessage, AppTheme> = scrollable(
        column![
            details_card(form, colors),
            additional_options_card(form, colors)
        ]
        .spacing(16)
        .padding([16, 20]),
    )
    .height(Fill)
    .style(crate::components::rail_scroll_style)
    .into();

    let footer = footer(form, colors);

    crate::components::rounded_top_pane(
        column![header_container, separator_h(), body, footer].height(Fill),
        top_radius,
    )
}

// ── Footer (Save / Cancel / Delete) ───────────────────────────────────────

fn footer<'a>(form: &'a SendForm, colors: &'a AppColors) -> Element<'a, SendEditMessage, AppTheme> {
    let mut save = buttons::primary(text(fl!("send-form-save")).size(14)).padding([8, 20]);
    if !form.saving {
        save = save.on_press(SendEditMessage::SavePressed);
    }

    let cancel = buttons::secondary(text(fl!("send-form-cancel")).size(14))
        .on_press(SendEditMessage::CancelPressed)
        .padding([8, 20]);

    let mut left_row = row![save, cancel].spacing(8).align_y(Alignment::Center);

    // Delete icon only for existing sends — there's no saved resource to
    // remove on a brand-new draft. Uses `titlebar_close_hover` (red) for
    // consistency with the vault detail pane's trash icon.
    if form.id.is_some() {
        left_row = left_row.push(Space::new().width(Fill));
        left_row = left_row.push(buttons::delete_icon_button(
            SendEditMessage::DeletePressed,
            colors,
        ));
    }

    container(left_row)
        .padding([12, 20])
        .width(Fill)
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.card_bg)
                .border(Border::default().color(theme.colors.border).width(1.0))
        })
        .into()
}

