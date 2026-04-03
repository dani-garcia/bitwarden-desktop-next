use iced::{
    Alignment, Background, Border, Color, Element, Fill, Shadow,
    widget::{Space, button, column, container, svg, text},
};

use crate::{
    components::buttons,
    theme::{AppColors, AppTheme},
};

use super::{LoginMessage, input_field, layout};

/// Renders the full center content for the login password screen:
/// wave icon, "Welcome back" title, email subtitle, card with password field + buttons.
pub fn view<'a>(
    email: &'a str,
    password: &'a str,
    show_password: bool,
    colors: &'a AppColors,
) -> Element<'a, LoginMessage, AppTheme> {
    let wave_icon = svg(svg::Handle::from_memory(crate::assets::WAVE_ICON))
        .width(64)
        .height(60);

    let title = text("Welcome back")
        .size(28)
        .color(colors.text_primary);

    let email_label = text(email).size(16).color(colors.text_secondary);

    let card = layout::auth_card(card_content(password, show_password, colors));

    column![wave_icon, title, email_label, Space::new().height(12), card]
        .spacing(8)
        .align_x(Alignment::Center)
        .into()
}

fn card_content<'a>(
    password: &'a str,
    show_password: bool,
    colors: &'a AppColors,
) -> Element<'a, LoginMessage, AppTheme> {
    let password_field = input_field::floating_label_input(
        "Master password (required)",
        password,
        LoginMessage::LoginPasswordChanged,
        Some(LoginMessage::LoginWithPassword),
        !show_password,
        Some((show_password, LoginMessage::ToggleLoginPasswordVisibility)),
        colors,
    );

    // TODO: "Get master password hint" navigates to hint request flow (not yet implemented)
    let hint_link = button(
        text("Get master password hint")
            .size(14)
            .color(colors.accent),
    )
    .on_press(LoginMessage::GetPasswordHint)
    .padding(0)
    .style(|_theme: &AppTheme, _status| button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: Color::TRANSPARENT,
        border: Border::default(),
        shadow: Shadow::default(),
        snap: false,
    });

    let login_button = buttons::primary(
        container(text("Log in with master password").size(16))
            .center_x(Fill)
            .padding([4, 8]),
    )
    .on_press(LoginMessage::LoginWithPassword)
    .width(Fill);

    let back_button = buttons::secondary(
        container(text("Back").size(16))
            .center_x(Fill)
            .padding([4, 8]),
    )
    .on_press(LoginMessage::BackToEmail)
    .width(Fill);

    column![password_field, hint_link, login_button, back_button]
        .spacing(12)
        .into()
}
