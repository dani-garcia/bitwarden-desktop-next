use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Fill};

use crate::icons;
use crate::state::UserId;
use crate::theme;

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
pub fn avatar_trigger<'a>(active_email: &'a str) -> Element<'a, AccountSwitcherMessage> {
    let initials = active_email
        .chars()
        .take(2)
        .collect::<String>()
        .to_uppercase();

    button(
        container(text(initials).size(13).color(theme::TEXT_PRIMARY))
            .width(36)
            .height(36)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(theme::AVATAR_BG)),
                border: iced::Border {
                    radius: 18.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
    )
    .on_press(AccountSwitcherMessage::ToggleDropdown)
    .padding(0)
    .style(|_theme, _status| button::Style {
        background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
        text_color: theme::TEXT_PRIMARY,
        border: iced::Border::default(),
        shadow: iced::Shadow::default(),
        snap: false,
    })
    .into()
}

/// Renders just the trigger button (initials + email + server + arrow).
/// Used by the login page's legacy account switcher bar. May be removed later.
#[allow(dead_code)]
pub fn trigger<'a>(
    active_email: &'a str,
    active_server: &'a str,
    dropdown_open: bool,
) -> Element<'a, AccountSwitcherMessage> {
    let initials = active_email
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    let arrow = if dropdown_open {
        icons::CHEVRON_UP
    } else {
        icons::CHEVRON_DOWN
    }
    .render(14.0, theme::TEXT_SECONDARY);

    button(
        row![
            container(text(initials).size(12).color(theme::TEXT_PRIMARY))
                .padding([4, 8])
                .style(|_theme| container::Style {
                    background: Some(iced::Background::Color(theme::ACCENT)),
                    border: iced::Border {
                        radius: 12.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            column![
                text(active_email).size(14).color(theme::TEXT_PRIMARY),
                text(active_server).size(12).color(theme::TEXT_SECONDARY),
            ]
            .spacing(1),
            arrow,
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .on_press(AccountSwitcherMessage::ToggleDropdown)
    .padding([4, 8])
    .style(|_theme, _status| button::Style {
        background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
        text_color: theme::TEXT_PRIMARY,
        border: iced::Border::default(),
        shadow: iced::Shadow::default(),
        snap: false,
    })
    .into()
}

/// Renders the floating dropdown panel (other accounts + add account)
pub fn dropdown<'a>(
    active_email: &'a str,
    accounts: &'a [AccountEntry],
) -> Element<'a, AccountSwitcherMessage> {
    let mut items: Vec<Element<'a, AccountSwitcherMessage>> = accounts
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

            button(
                row![
                    container(text(initial).size(11).color(theme::TEXT_PRIMARY))
                        .padding([3, 7])
                        .style(|_theme| container::Style {
                            background: Some(iced::Background::Color(theme::ACCENT)),
                            border: iced::Border {
                                radius: 10.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    column![
                        text(format!("{}{}", account.email, locked_label))
                            .size(12)
                            .color(theme::TEXT_PRIMARY),
                        text(&account.server_url)
                            .size(10)
                            .color(theme::TEXT_SECONDARY),
                    ]
                    .spacing(1),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .on_press(AccountSwitcherMessage::SwitchUser(uid))
            .padding([6, 12])
            .width(Fill)
            .style(|_theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme::ITEM_HOVER,
                    _ => iced::Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(iced::Background::Color(bg)),
                    text_color: theme::TEXT_PRIMARY,
                    border: iced::Border::default(),
                    shadow: iced::Shadow::default(),
                    snap: false,
                }
            })
            .into()
        })
        .collect();

    items.push(
        button(text("+ Add account").size(12).color(theme::TEXT_SECONDARY))
            .padding([8, 12])
            .width(Fill)
            .style(|_theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme::ITEM_HOVER,
                    _ => iced::Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(iced::Background::Color(bg)),
                    text_color: theme::TEXT_SECONDARY,
                    border: iced::Border::default(),
                    shadow: iced::Shadow::default(),
                    snap: false,
                }
            })
            .into(),
    );

    container(column(items).spacing(0))
        .width(240)
        .padding([4, 0])
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::CARD_BG)),
            border: iced::Border {
                color: theme::BORDER,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
}
