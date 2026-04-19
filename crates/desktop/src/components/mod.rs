pub mod account_switcher;
pub mod bottom_sheet;
pub mod buttons;
pub mod drop_down;
pub mod icons;
pub mod inputs;
pub mod modal;
pub mod spinner;
pub mod toast;
pub mod totp;
pub mod virtual_list;

use iced::{
    Color, Element, Fill, Shadow,
    widget::{container, rule},
};

use crate::theme::{AppTheme, RADIUS_LG};

/// Horizontal separator line with border color and full width.
pub fn separator_h<'a, M: 'a>() -> Element<'a, M, AppTheme> {
    rule::horizontal(1)
        .style(|theme: &AppTheme| rule::Style {
            color: theme.colors.border,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: false,
        })
        .into()
}

/// Vertical separator line with border color and full height.
pub fn separator_v<'a, M: 'a>() -> Element<'a, M, AppTheme> {
    rule::vertical(1)
        .style(|theme: &AppTheme| rule::Style {
            color: theme.colors.border,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: false,
        })
        .into()
}

/// Card container with background color and rounded corners.
pub fn styled_card<'a, M: 'a>(content: Element<'a, M, AppTheme>) -> Element<'a, M, AppTheme> {
    container(content)
        .padding([12, 16])
        .width(Fill)
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.background)
                .border(iced::border::rounded(RADIUS_LG))
                .shadow(Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.20),
                    offset: iced::Vector::new(0.0, 1.0),
                    blur_radius: 2.0,
                })
        })
        .into()
}
