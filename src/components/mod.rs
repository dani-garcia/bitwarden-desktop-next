pub mod account_switcher;
pub mod buttons;
pub mod icons;

use iced::{
    Background, Border, Element, Fill,
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
        .style(|theme: &AppTheme| container::Style {
            background: Some(Background::Color(theme.colors.background)),
            border: Border {
                radius: RADIUS_LG.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
