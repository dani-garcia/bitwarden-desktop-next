use iced::{
    Alignment, Border, Element, Fill,
    widget::{column, container, row, text},
};

use crate::{
    components::buttons,
    state::UserId,
    theme::{AppColors, AppTheme},
};

#[derive(Debug, Clone)]
pub enum AccountSwitcherMessage {
    ToggleDropdown,
    SwitchUser(UserId),
    AddAccount,
}

pub struct AccountEntry {
    pub user_id: UserId,
    pub email: String,
    #[expect(dead_code)] // Not displayed yet; reserved for future avatar / profile views.
    pub display_name: String,
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
        container(text(initials).size(14).color(colors.text_primary))
            .width(36)
            .height(36)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(|theme: &AppTheme| {
                container::Style::default()
                    .background(theme.colors.avatar_bg)
                    .border(iced::border::rounded(18))
            }),
    )
    .on_press(AccountSwitcherMessage::ToggleDropdown)
    .padding(0)
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
                    container(text(initial).size(14).color(colors.text_primary))
                        .padding([3, 7])
                        .style(|theme: &AppTheme| {
                            container::Style::default()
                                .background(theme.colors.accent)
                                .border(iced::border::rounded(10))
                        }),
                    column![
                        text(format!("{}{}", account.email, locked_label))
                            .size(14)
                            .color(colors.text_primary),
                        text(&account.server_url)
                            .size(14)
                            .color(colors.text_secondary),
                    ]
                    .spacing(1),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                false,
                iced::Color::TRANSPARENT,
                colors.item_hover,
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
            text("+ Add account").size(14).color(colors.text_secondary),
            false,
            iced::Color::TRANSPARENT,
            colors.item_hover,
            0.0,
        )
        .on_press(AccountSwitcherMessage::AddAccount)
        .padding([8, 12])
        .width(Fill)
        .into(),
    );

    container(column(items).spacing(0))
        .width(240)
        .padding([4, 0])
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.card_bg)
                .border(
                    Border::default()
                        .color(theme.colors.border)
                        .width(1.0)
                        .rounded(4),
                )
        })
        .into()
}
