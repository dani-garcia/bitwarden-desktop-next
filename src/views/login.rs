use iced::widget::{Space, button, column, container, row, stack, svg, text, text_input};
use iced::{Alignment, Element, Fill, Length, Padding};

use crate::icons;
use crate::theme;
use crate::widgets::account_switcher::{self, AccountEntry, AccountSwitcherMessage};

#[derive(Debug, Clone)]
pub enum LoginMessage {
    PasswordChanged(String),
    TogglePasswordVisibility,
    Unlock,
    LogOut,
    AccountSwitcher(AccountSwitcherMessage),
}

pub fn view<'a>(
    email: &'a str,
    server: &'a str,
    password: &'a str,
    show_password: bool,
    accounts: &'a [AccountEntry],
    dropdown_open: bool,
) -> Element<'a, LoginMessage> {
    let logo = svg(svg::Handle::from_path("assets/logo-white.svg"))
        .width(209)
        .height(35);

    // Lock icon SVG
    let lock_icon = svg(svg::Handle::from_path("assets/lock-icon.svg"))
        .width(64)
        .height(60);

    let title = text("Your vault is locked")
        .size(26)
        .color(theme::TEXT_PRIMARY);

    let email_label = text(email).size(16).color(theme::TEXT_SECONDARY);

    // Password input — the whole row is wrapped in a styled container for a unified border
    let password_input = {
        let mut input = text_input("", password)
            .on_input(LoginMessage::PasswordChanged)
            .on_submit(LoginMessage::Unlock)
            .size(16)
            .padding([10, 12])
            .width(Fill)
            .style(|_theme, _status| text_input::Style {
                background: iced::Background::Color(iced::Color::TRANSPARENT),
                border: iced::Border {
                    color: iced::Color::TRANSPARENT,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                icon: theme::TEXT_MUTED,
                placeholder: theme::TEXT_SECONDARY,
                value: theme::TEXT_PRIMARY,
                selection: theme::ACCENT,
            });
        if !show_password {
            input = input.secure(true);
        }
        input
    };

    let toggle_icon = if show_password {
        icons::EYE
    } else {
        icons::EYE_SLASH
    }
    .render(16.0, theme::TEXT_SECONDARY);

    let toggle_button = button(toggle_icon)
        .on_press(LoginMessage::TogglePasswordVisibility)
        .padding([10, 12])
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered => iced::Color::from_rgb(
                    0x1f as f32 / 255.0,
                    0x2a as f32 / 255.0,
                    0x3c as f32 / 255.0,
                ),
                _ => iced::Color::TRANSPARENT,
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: theme::TEXT_SECONDARY,
                border: iced::Border::default(),
                shadow: iced::Shadow::default(),
                snap: false,
            }
        });

    // Floating label that sits on the top border of the input
    let floating_label = container(
        text("Master password (required)")
            .size(12)
            .color(theme::TEXT_SECONDARY),
    )
    .padding([0, 4])
    .style(|_theme| container::Style {
        background: Some(iced::Background::Color(theme::BACKGROUND)),
        ..Default::default()
    });

    // Input border container
    let input_border = container(row![password_input, toggle_button].align_y(Alignment::Center))
        .width(Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
            border: iced::Border {
                color: theme::BORDER,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        });

    // Stack the floating label on top of the input, offset up
    let password_field = stack![
        column![Space::new().height(Length::Fixed(8.0)), input_border],
        container(floating_label).padding([0, 12]),
    ];

    let unlock_button = button(
        container(text("Unlock").size(16).color(theme::BACKGROUND))
            .center_x(Fill)
            .padding([4, 8]),
    )
    .on_press(LoginMessage::Unlock)
    .width(Fill)
    .style(|_theme, status| {
        let bg = match status {
            button::Status::Hovered => iced::Color::from_rgb(
                0xaa as f32 / 255.0,
                0xc3 as f32 / 255.0,
                0xef as f32 / 255.0,
            ),
            _ => theme::ACCENT,
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: theme::BACKGROUND,
            border: iced::Border {
                color: bg,
                width: 1.0,
                radius: 20.0.into(),
            },
            shadow: iced::Shadow::default(),
            snap: false,
        }
    });

    let logout_button = button(
        container(text("Log out").size(16).color(theme::ACCENT))
            .center_x(Fill)
            .padding([4, 8]),
    )
    .on_press(LoginMessage::LogOut)
    .width(Fill)
    .style(|_theme, status| {
        let bg = match status {
            button::Status::Hovered => iced::Color::from_rgb(
                0x1f as f32 / 255.0,
                0x2a as f32 / 255.0,
                0x3c as f32 / 255.0,
            ),
            _ => iced::Color::TRANSPARENT,
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: theme::ACCENT,
            border: iced::Border {
                color: theme::ACCENT,
                width: 1.0,
                radius: 20.0.into(),
            },
            shadow: iced::Shadow::default(),
            snap: false,
        }
    });

    // Card only contains the form elements
    let card = container(
        column![
            password_field,
            unlock_button,
            text("or").size(14).color(theme::TEXT_PRIMARY),
            logout_button,
        ]
        .spacing(12)
        .align_x(Alignment::Center),
    )
    .max_width(450)
    .padding(32)
    .style(|_theme| container::Style {
        background: Some(iced::Background::Color(theme::BACKGROUND)),
        border: iced::Border {
            color: theme::BORDER,
            width: 1.0,
            radius: 16.0.into(),
        },
        ..Default::default()
    });

    // Title, email, and lock icon sit above the card
    let center_content = column![lock_icon, title, email_label, Space::new().height(12), card,]
        .spacing(8)
        .align_x(Alignment::Center);

    let logo_row = container(logo).width(Fill).padding([16, 24]);

    // Background illustrations pinned to bottom-left and bottom-right corners (11% opacity)
    let bg_left = svg(svg::Handle::from_path("assets/bg-left.svg"))
        .width(Length::Fixed(400.0))
        .height(Length::Fixed(180.0))
        .opacity(0.11);

    let bg_right = svg(svg::Handle::from_path("assets/bg-right.svg"))
        .width(Length::Fixed(400.0))
        .height(Length::Fixed(240.0))
        .opacity(0.11);

    let bg_illustrations = column![
        Space::new().height(Fill),
        row![bg_left, Space::new().width(Fill), bg_right].align_y(Alignment::End),
    ]
    .width(Fill)
    .height(Fill);

    let status_bar = container(
        text(format!("Accessing {}", server))
            .size(14)
            .color(theme::TEXT_SECONDARY),
    )
    .center_x(Fill)
    .padding(Padding {
        top: 12.0,
        right: 0.0,
        bottom: 20.0,
        left: 0.0,
    });

    // Round "..." menu button (top-right)
    let menu_dot_button: Element<'a, LoginMessage> = button(
        container(icons::THREE_DOTS.render(16.0, theme::ACCENT))
            .width(36)
            .height(36)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .on_press(LoginMessage::AccountSwitcher(
        AccountSwitcherMessage::ToggleDropdown,
    ))
    .padding(0)
    .style(|_theme, status| {
        let bg = match status {
            button::Status::Hovered => theme::ITEM_HOVER,
            _ => theme::HEADER_BG,
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: theme::ACCENT,
            border: iced::Border {
                radius: 18.0.into(),
                ..Default::default()
            },
            shadow: iced::Shadow::default(),
            snap: false,
        }
    })
    .into();

    // Top row: logo left, ... button right
    let top_row = row![
        logo_row,
        Space::new().width(Fill),
        container(menu_dot_button).padding([16, 24]),
    ]
    .align_y(Alignment::Center);

    // Main foreground content
    let foreground = column![
        top_row,
        Space::new().height(Length::Fixed(40.0)),
        container(center_content).center_x(Fill),
        Space::new().height(Fill),
        status_bar,
    ]
    .width(Fill)
    .height(Fill);

    // Account switcher dropdown overlay (top-right, below the ... button)
    let dropdown_layer: Element<'a, LoginMessage> = if dropdown_open {
        let dd = account_switcher::dropdown(email, accounts).map(LoginMessage::AccountSwitcher);
        container(
            column![
                Space::new().height(Length::Fixed(56.0)),
                container(dd).align_right(Fill).padding([0, 16]),
            ]
            .width(Fill),
        )
        .width(Fill)
        .height(Fill)
        .into()
    } else {
        Space::new().width(0).height(0).into()
    };

    // Stack: background illustrations, foreground content, dropdown overlay
    let layered = stack![bg_illustrations, foreground, dropdown_layer];

    container(layered)
        .width(Fill)
        .height(Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::CARD_BG)),
            ..Default::default()
        })
        .into()
}
