use iced::{
    Alignment, Background, Border, Element, Fill,
    widget::{column, container, row, text},
};

use crate::{components::{buttons, icons}, state::UserId, theme::{AppColors, AppTheme}};

#[derive(Debug, Clone)]
pub enum AccountSwitcherMessage {
    ToggleDropdown,
    SwitchUser(UserId),
}

pub struct AccountEntry {
    pub user_id: UserId,
    pub email: String,
    pub server_url: String,
    pub locked: bool,
}

/// Renders a round avatar circle trigger for the vault header.
pub fn avatar_trigger<'a>(
    active_email: &'a str,
    colors: &AppColors,
) -> Element<'a, AccountSwitcherMessage, AppTheme> {
    let initials = active_email
        .chars()
        .take(2)
        .collect::<String>()
        .to_uppercase();

    buttons::transparent(
        container(text(initials).size(13).color(colors.text_primary))
            .width(36)
            .height(36)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(|theme: &AppTheme| container::Style {
                background: Some(Background::Color(theme.colors.avatar_bg)),
                border: Border {
                    radius: 18.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
    )
    .on_press(AccountSwitcherMessage::ToggleDropdown)
    .padding(0)
    .into()
}

/// Renders just the trigger button (initials + email + server + arrow).
/// Used by the login page's legacy account switcher bar. May be removed later.
#[allow(dead_code)]
pub fn trigger<'a>(
    active_email: &'a str,
    active_server: &'a str,
    dropdown_open: bool,
    colors: &AppColors,
) -> Element<'a, AccountSwitcherMessage, AppTheme> {
    let initials = active_email
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    let icon = if dropdown_open {
        icons::CHEVRON_UP
    } else {
        icons::CHEVRON_DOWN
    };

    buttons::transparent(
        row![
            container(text(initials).size(12).color(colors.text_primary))
                .padding([4, 8])
                .style(|theme: &AppTheme| container::Style {
                    background: Some(Background::Color(theme.colors.accent)),
                    border: Border {
                        radius: 12.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            column![
                text(active_email).size(14).color(colors.text_primary),
                text(active_server).size(12).color(colors.text_secondary),
            ]
            .spacing(1),
            icon.render(14.0, colors.text_secondary),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .on_press(AccountSwitcherMessage::ToggleDropdown)
    .padding([4, 8])
    .into()
}

/// Renders the floating dropdown panel (other accounts + add account)
pub fn dropdown<'a>(
    active_email: &'a str,
    accounts: &'a [AccountEntry],
    colors: &AppColors,
) -> Element<'a, AccountSwitcherMessage, AppTheme> {
    let mut items: Vec<Element<'a, AccountSwitcherMessage, AppTheme>> = accounts
        .iter()
        .filter(|a| a.email != active_email)
        .map(|account| {
            let uid = account.user_id.clone();
            let locked_label = if account.locked { " (locked)" } else { "" };
            let initial = account
                .email
                .chars()
                .next()
                .unwrap_or('?')
                .to_uppercase()
                .to_string();

            buttons::ghost(
                row![
                    container(text(initial).size(11).color(colors.text_primary))
                        .padding([3, 7])
                        .style(|theme: &AppTheme| container::Style {
                            background: Some(Background::Color(theme.colors.accent)),
                            border: Border {
                                radius: 10.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    column![
                        text(format!("{}{}", account.email, locked_label))
                            .size(12)
                            .color(colors.text_primary),
                        text(&account.server_url)
                            .size(10)
                            .color(colors.text_secondary),
                    ]
                    .spacing(1),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                false,
                iced::Color::TRANSPARENT,
                0.0,
            )
            .on_press(AccountSwitcherMessage::SwitchUser(uid))
            .padding([6, 12])
            .width(Fill)
            .into()
        })
        .collect();

    items.push(
        buttons::ghost(
            text("+ Add account")
                .size(12)
                .color(colors.text_secondary),
            false,
            iced::Color::TRANSPARENT,
            0.0,
        )
        .padding([8, 12])
        .width(Fill)
        .into(),
    );

    container(column(items).spacing(0))
        .width(240)
        .padding([4, 0])
        .style(|theme: &AppTheme| container::Style {
            background: Some(Background::Color(theme.colors.card_bg)),
            border: Border {
                color: theme.colors.border,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
}
