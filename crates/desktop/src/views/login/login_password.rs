use iced::{
    Alignment, Element, Fill,
    widget::{Space, column, container, svg, text},
};

use crate::{
    components::{buttons, inputs::reveal_text_field_with_submit},
    fl,
    theme::{AppColors, AppTheme},
};

use super::{LoginMessage, layout};

/// Renders the full center content for the login password screen:
/// wave icon, "Welcome back" title, email subtitle, card with password field + buttons.
pub fn view<'a>(
    email: &'a str,
    password: &'a str,
    colors: &'a AppColors,
) -> Element<'a, LoginMessage, AppTheme> {
    let wave_icon = svg(svg::Handle::from_memory(crate::assets::WAVE_ICON))
        .width(64)
        .height(60);

    let title = text(fl!("login-password-title"))
        .size(28)
        .color(colors.text_primary);

    let email_label = text(email).size(16).color(colors.text_secondary);

    let card = layout::auth_card(card_content(password, colors));

    column![wave_icon, title, email_label, Space::new().height(12), card]
        .spacing(8)
        .align_x(Alignment::Center)
        .into()
}

fn card_content<'a>(
    password: &'a str,
    colors: &'a AppColors,
) -> Element<'a, LoginMessage, AppTheme> {
    let password_field = reveal_text_field_with_submit(
        fl!("login-password-placeholder"),
        password,
        LoginMessage::LoginPasswordChanged,
        LoginMessage::LoginWithPassword,
        false,
        colors,
    );

    // TODO: "Get master password hint" navigates to hint request flow (not yet implemented)
    let hint_link = buttons::transparent(
        text(fl!("login-password-get-hint"))
            .size(14)
            .color(colors.accent),
    )
    .on_press(LoginMessage::GetPasswordHint)
    .padding(0);

    let login_button = buttons::primary(
        container(text(fl!("login-password-submit")).size(16))
            .center_x(Fill)
            .padding([4, 8]),
    )
    .on_press(LoginMessage::LoginWithPassword)
    .width(Fill);

    let back_button = buttons::secondary(
        container(text(fl!("login-password-back")).size(16))
            .center_x(Fill)
            .padding([4, 8]),
    )
    .on_press(LoginMessage::BackToEmail)
    .width(Fill);

    column![password_field, hint_link, login_button, back_button]
        .spacing(12)
        .into()
}
