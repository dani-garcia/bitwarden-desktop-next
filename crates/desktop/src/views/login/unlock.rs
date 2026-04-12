use iced::{
    Alignment, Element, Fill,
    widget::{Space, column, container, svg, text},
};

use crate::{
    components::buttons,
    state::UnlockMethod,
    theme::{AppColors, AppTheme},
};

use super::{LoginMessage, input_field, layout};

/// Renders the full center content for the unlock screen:
/// lock icon, title, email, and the card with method-specific controls.
pub fn view<'a>(
    method: UnlockMethod,
    alternatives: &[UnlockMethod],
    email: &'a str,
    password: &'a str,
    pin: &'a str,
    show_password: bool,
    colors: &'a AppColors,
) -> Element<'a, LoginMessage, AppTheme> {
    let lock_icon = svg(svg::Handle::from_memory(crate::assets::LOCK_ICON))
        .width(64)
        .height(60);

    let title = text("Your vault is locked")
        .size(28)
        .color(colors.text_primary);

    let email_label = text(email).size(16).color(colors.text_secondary);

    let card = layout::auth_card(card_content(
        method,
        alternatives,
        password,
        pin,
        show_password,
        colors,
    ));

    column![lock_icon, title, email_label, Space::new().height(12), card]
        .spacing(8)
        .align_x(Alignment::Center)
        .into()
}

fn card_content<'a>(
    method: UnlockMethod,
    alternatives: &[UnlockMethod],
    password: &'a str,
    pin: &'a str,
    show_password: bool,
    colors: &'a AppColors,
) -> Element<'a, LoginMessage, AppTheme> {
    let mut items: Vec<Element<'_, LoginMessage, AppTheme>> = Vec::new();

    match method {
        UnlockMethod::Biometrics => {
            items.push(
                buttons::primary(
                    container(text("Unlock with biometrics").size(16))
                        .center_x(Fill)
                        .padding([4, 8]),
                )
                .on_press(LoginMessage::UnlockWithBiometrics)
                .width(Fill)
                .into(),
            );
        }
        UnlockMethod::Pin => {
            items.push(input_field::floating_label_input(
                "PIN (required)",
                pin,
                LoginMessage::PinChanged,
                Some(LoginMessage::UnlockWithPin),
                true,
                None,
                colors,
            ));
            items.push(
                buttons::primary(
                    container(text("Unlock with PIN").size(16))
                        .center_x(Fill)
                        .padding([4, 8]),
                )
                .on_press(LoginMessage::UnlockWithPin)
                .width(Fill)
                .into(),
            );
        }
        UnlockMethod::MasterPassword => {
            items.push(input_field::floating_label_input(
                "Master password (required)",
                password,
                LoginMessage::PasswordChanged,
                Some(LoginMessage::Unlock),
                !show_password,
                Some((show_password, LoginMessage::TogglePasswordVisibility)),
                colors,
            ));
            items.push(
                buttons::primary(
                    container(text("Unlock").size(16))
                        .center_x(Fill)
                        .padding([4, 8]),
                )
                .on_press(LoginMessage::Unlock)
                .width(Fill)
                .into(),
            );
        }
    }

    // "or" separator + fallback buttons
    if !alternatives.is_empty() {
        items.push(
            text("or")
                .size(14)
                .color(colors.text_primary)
                .center()
                .into(),
        );

        for alt in alternatives {
            let (label, msg) = match alt {
                UnlockMethod::Biometrics => (
                    "Unlock with biometrics",
                    LoginMessage::SwitchUnlockMethod(UnlockMethod::Biometrics),
                ),
                UnlockMethod::Pin => (
                    "Unlock with PIN",
                    LoginMessage::SwitchUnlockMethod(UnlockMethod::Pin),
                ),
                UnlockMethod::MasterPassword => (
                    "Unlock with master password",
                    LoginMessage::SwitchUnlockMethod(UnlockMethod::MasterPassword),
                ),
            };
            items.push(
                buttons::secondary(
                    container(text(label).size(16))
                        .center_x(Fill)
                        .padding([4, 8]),
                )
                .on_press(msg)
                .width(Fill)
                .into(),
            );
        }
    } else {
        items.push(
            text("or")
                .size(14)
                .color(colors.text_primary)
                .center()
                .into(),
        );
    }

    // Log out button always at the end
    items.push(
        buttons::secondary(
            container(text("Log out").size(16))
                .center_x(Fill)
                .padding([4, 8]),
        )
        .on_press(LoginMessage::LogOut)
        .width(Fill)
        .into(),
    );

    column(items).spacing(12).align_x(Alignment::Center).into()
}
