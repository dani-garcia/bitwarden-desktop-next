use iced::{
    Alignment, Element, Fill,
    widget::{Space, checkbox, column, container, row, svg, text},
};

use crate::{
    components::{buttons, icons},
    theme::{AppColors, AppTheme},
};

use super::{LoginMessage, input_field, layout};

/// Renders the full center content for the login email entry screen:
/// vault icon, title, card with email field + checkbox + buttons,
/// and "New to Bitwarden?" link below the card.
pub fn view<'a>(
    email: &'a str,
    remember_email: bool,
    colors: &'a AppColors,
) -> Element<'a, LoginMessage, AppTheme> {
    let vault_icon = svg(svg::Handle::from_memory(crate::assets::VAULT_ICON))
        .width(64)
        .height(60);

    let title = text("Log in to Bitwarden")
        .size(28)
        .color(colors.text_primary);

    let card = layout::auth_card(card_content(email, remember_email, colors));

    // "New to Bitwarden? Create account" link below card
    // TODO: Create account navigates to registration view (not yet implemented)
    let create_account_link = row![
        text("New to Bitwarden? ").size(14).color(colors.text_secondary),
        text("Create account").size(14).color(colors.accent),
    ]
    .align_y(Alignment::Center);

    column![vault_icon, title, Space::new().height(12), card, Space::new().height(8), create_account_link]
        .spacing(8)
        .align_x(Alignment::Center)
        .into()
}

fn card_content<'a>(
    email: &'a str,
    remember_email: bool,
    colors: &'a AppColors,
) -> Element<'a, LoginMessage, AppTheme> {
    let email_field = input_field::floating_label_input(
        "Email address (required)",
        email,
        LoginMessage::EmailChanged,
        Some(LoginMessage::ContinueWithEmail),
        false,
        None,
        colors,
    );

    let remember_checkbox = checkbox(remember_email)
        .label("Remember email")
        .on_toggle(LoginMessage::ToggleRememberEmail)
        .size(18)
        .spacing(8);

    let continue_button = buttons::primary(
        container(text("Continue").size(16))
            .center_x(Fill)
            .padding([4, 8]),
    )
    .on_press(LoginMessage::ContinueWithEmail)
    .width(Fill);

    let or_text = text("Or")
        .size(14)
        .color(colors.text_primary)
        .center();

    let sso_button = buttons::secondary(
        container(
            row![
                icons::BWI_HANDSHAKE.render(16.0, colors.accent),
                text("Use single sign-on").size(16),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .center_x(Fill)
        .padding([4, 8]),
    )
    .on_press(LoginMessage::UseSingleSignOn)
    .width(Fill);

    column![
        email_field,
        remember_checkbox,
        continue_button,
        container(or_text).center_x(Fill),
        sso_button,
    ]
    .spacing(12)
    .into()
}
