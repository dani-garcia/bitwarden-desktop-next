use iced::{
    Alignment, Background, Border, Color, Element, Fill, Length, Padding, Shadow,
    theme::Base,
    widget::{Space, button, column, container, row, stack, svg, text, text_input},
};

use crate::{
    components::{
        account_switcher::{self, AccountEntry, AccountSwitcherMessage},
        buttons, icons,
    },
    theme::{AppColors, AppTheme},
};

#[derive(Debug, Clone)]
pub enum LoginMessage {
    PasswordChanged(String),
    TogglePasswordVisibility,
    Unlock,
    LogOut,
    AccountSwitcher(AccountSwitcherMessage),
}

#[derive(Debug, Clone)]
pub enum LoginAction {
    Unlock,
    LogOut,
    SwitchUser(String),
}

pub struct LoginView {
    pub password_input: String,
    pub show_password: bool,
    pub dropdown_open: bool,
}

impl LoginView {
    pub fn new() -> Self {
        Self {
            password_input: String::new(),
            show_password: false,
            dropdown_open: false,
        }
    }

    pub fn update(
        &mut self,
        msg: LoginMessage,
    ) -> Vec<LoginAction> {
        let mut actions = Vec::new();
        match msg {
            LoginMessage::PasswordChanged(pw) => self.password_input = pw,
            LoginMessage::TogglePasswordVisibility => self.show_password = !self.show_password,
            LoginMessage::Unlock => {
                self.password_input.clear();
                self.show_password = false;
                actions.push(LoginAction::Unlock);
            }
            LoginMessage::LogOut => {
                actions.push(LoginAction::LogOut);
            }
            LoginMessage::AccountSwitcher(asm) => match asm {
                AccountSwitcherMessage::ToggleDropdown => {
                    self.dropdown_open = !self.dropdown_open;
                }
                AccountSwitcherMessage::SwitchUser(uid) => {
                    self.dropdown_open = false;
                    actions.push(LoginAction::SwitchUser(uid));
                }
            },
        }
        actions
    }
}

pub fn view<'a>(
    email: &'a str,
    server: &'a str,
    password: &'a str,
    show_password: bool,
    accounts: &'a [AccountEntry],
    dropdown_open: bool,
    colors: &'a AppColors,
) -> Element<'a, LoginMessage, AppTheme> {
    let logo = svg(svg::Handle::from_path("assets/logo-white.svg"))
        .width(209)
        .height(35)
        .style(|theme: &AppTheme, _status| svg::Style {
            color: if theme.mode() == iced::theme::Mode::Light {
                Some(theme.colors.button_primary)
            } else {
                None
            },
        });

    // Lock icon SVG
    let lock_icon = svg(svg::Handle::from_path("assets/lock-icon.svg"))
        .width(64)
        .height(60);

    let title = text("Your vault is locked")
        .size(28)
        .color(colors.text_primary);

    let email_label = text(email).size(16).color(colors.text_secondary);

    // Password input — the whole row is wrapped in a styled container for a unified border
    let password_input = {
        let mut input = text_input("", password)
            .on_input(LoginMessage::PasswordChanged)
            .on_submit(LoginMessage::Unlock)
            .size(16)
            .padding([10, 12])
            .width(Fill)
            .style(|theme: &AppTheme, _status| text_input::Style {
                background: Background::Color(Color::TRANSPARENT),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                icon: theme.colors.text_muted,
                placeholder: theme.colors.text_secondary,
                value: theme.colors.text_primary,
                selection: theme.colors.accent,
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
    .render(16.0, colors.text_secondary);

    let toggle_button = buttons::ghost_icon(toggle_icon, colors.item_hover)
        .on_press(LoginMessage::TogglePasswordVisibility)
        .padding([10, 12]);

    // Floating label that sits on the top border of the input
    let floating_label = container(
        text("Master password (required)")
            .size(14)
            .color(colors.text_secondary),
    )
    .padding([0, 4])
    .style(|theme: &AppTheme| container::Style {
        background: Some(Background::Color(theme.colors.background)),
        ..Default::default()
    });

    // Input border container
    let input_border = container(row![password_input, toggle_button].align_y(Alignment::Center))
        .width(Fill)
        .style(|theme: &AppTheme| container::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: Border {
                color: theme.colors.border,
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

    let unlock_button = buttons::primary(
        container(text("Unlock").size(16)).center_x(Fill).padding([4, 8]),
    )
    .on_press(LoginMessage::Unlock)
    .width(Fill);

    let logout_button = buttons::secondary(
        container(text("Log out").size(16)).center_x(Fill).padding([4, 8]),
    )
    .on_press(LoginMessage::LogOut)
    .width(Fill);

    // Card only contains the form elements
    let card = container(
        column![
            password_field,
            unlock_button,
            text("or").size(14).color(colors.text_primary),
            logout_button,
        ]
        .spacing(12)
        .align_x(Alignment::Center),
    )
    .max_width(450)
    .padding(32)
    .style(|theme: &AppTheme| container::Style {
        background: Some(Background::Color(theme.colors.background)),
        border: Border {
            color: theme.colors.border,
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
            .color(colors.text_secondary),
    )
    .center_x(Fill)
    .padding(Padding {
        top: 12.0,
        right: 0.0,
        bottom: 20.0,
        left: 0.0,
    });

    // Round "..." menu button with account switcher dropdown
    let menu_dot_trigger = button(
        container(icons::THREE_DOTS.render(16.0, Color::WHITE))
            .width(36)
            .height(36)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
    )
    .on_press(LoginMessage::AccountSwitcher(
        AccountSwitcherMessage::ToggleDropdown,
    ))
    .padding(0)
    .style(|theme: &AppTheme, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => theme.colors.button_primary_hover,
            _ => theme.colors.button_primary,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: Color::WHITE,
            border: Border {
                radius: 18.0.into(),
                ..Default::default()
            },
            shadow: Shadow::default(),
            snap: false,
        }
    });

    let dd_panel =
        account_switcher::dropdown(email, accounts, colors).map(LoginMessage::AccountSwitcher);
    let menu_dot_dropdown: Element<'a, LoginMessage, AppTheme> =
        crate::components::drop_down::DropDown::new(menu_dot_trigger, dd_panel, dropdown_open)
            .on_dismiss(LoginMessage::AccountSwitcher(
                AccountSwitcherMessage::ToggleDropdown,
            ))
            .alignment(crate::components::drop_down::Alignment::BelowRight)
            .width(240.0)
            .offset(4.0)
            .into();

    // Top row: logo left, ... button right
    let top_row = row![
        logo_row,
        Space::new().width(Fill),
        container(menu_dot_dropdown).padding([16, 24]),
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

    // Stack: background illustrations, foreground content
    let layered = stack![bg_illustrations, foreground];

    container(layered)
        .width(Fill)
        .height(Fill)
        .style(|theme: &AppTheme| container::Style {
            background: Some(Background::Color(theme.colors.card_bg)),
            ..Default::default()
        })
        .into()
}
