//! Self-hosted environment configuration modal.
//!
//! Opened from the server selector dropdown when the user picks "Self-hosted".
//! Collects a base URL and writes it back into the active `LoginEmail`'s
//! `selected_server` on Save.

use iced::{
    Alignment, Element, Fill, Padding,
    widget::{self, Space, column, container, row, text},
};

use crate::{
    components::{buttons, icons, inputs, modal},
    fl,
    theme::{AppColors, AppTheme},
};

use super::{LoginMessage, SelfHostedModal};

pub const SELF_HOSTED_URL_FIELD_ID: widget::Id = widget::Id::new("self-hosted-url-field");

pub fn view<'a>(
    state: &'a SelfHostedModal,
    colors: &'a AppColors,
) -> Option<Element<'a, LoginMessage, AppTheme>> {
    let progress = state.fade.progress_if_visible()?;

    let header = row![
        text(fl!("login-self-hosted-modal-title"))
            .size(18)
            .font(crate::APP_FONT_BOLD)
            .color(colors.text_primary),
        Space::new().width(Fill),
        buttons::ghost_icon(
            icons::X_LG.render(16.0, colors.text_primary),
            colors.item_hover,
        )
        .padding([6, 6])
        .on_press(LoginMessage::SelfHostedCancel),
    ]
    .align_y(Alignment::Center);

    let url_input = inputs::bare_text_input(&state.url_input)
        .id(SELF_HOSTED_URL_FIELD_ID)
        .on_input(LoginMessage::SelfHostedUrlChanged)
        .on_submit(LoginMessage::SelfHostedSave);
    let url_field = if state.url_error {
        inputs::errored_field_frame(fl!("login-self-hosted-modal-url-label"), url_input, colors)
    } else {
        inputs::field_frame(fl!("login-self-hosted-modal-url-label"), url_input, colors)
    };

    // Inline validation row replaces the helper text once the user hits Save
    // with an invalid URL — they read as alternatives, not stacked siblings.
    let info_row: Element<'a, LoginMessage, AppTheme> = if state.url_error {
        inputs::field_error_row(fl!("login-self-hosted-modal-url-error"), colors)
    } else {
        text(fl!("login-self-hosted-modal-url-helper"))
            .size(12)
            .color(colors.text_secondary)
            .into()
    };

    let footer = modal::footer_actions(
        fl!("login-self-hosted-modal-save"),
        Some(LoginMessage::SelfHostedSave),
        fl!("login-self-hosted-modal-cancel"),
        LoginMessage::SelfHostedCancel,
    );

    let body = column![header, url_field, info_row, footer]
    .spacing(16)
    .padding(Padding::from([20, 24]))
    .width(Fill);

    Some(modal::dialog(
        460.0,
        None,
        |c| c.background,
        progress,
        container(body),
        LoginMessage::SelfHostedCancel,
    ))
}
