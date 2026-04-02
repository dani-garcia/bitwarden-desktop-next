use iced::{
    Background, Border, Color, Element, Fill, Shadow,
    widget::{button, container, rule},
};

use crate::theme;

/// Horizontal separator line with BORDER color and full width.
pub fn separator_h<'a, M: 'a>() -> Element<'a, M> {
    rule::horizontal(1)
        .style(|_theme| rule::Style {
            color: theme::BORDER,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: false,
        })
        .into()
}

/// Vertical separator line with BORDER color and full height.
pub fn separator_v<'a, M: 'a>() -> Element<'a, M> {
    rule::vertical(1)
        .style(|_theme| rule::Style {
            color: theme::BORDER,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: false,
        })
        .into()
}

/// Standard button style: transparent bg, ITEM_HOVER on hover, optional active state.
pub fn hover_button_style(
    status: button::Status,
    is_active: bool,
    active_bg: Color,
    radius: f32,
) -> button::Style {
    let bg = if is_active {
        active_bg
    } else {
        match status {
            button::Status::Hovered => theme::ITEM_HOVER,
            _ => Color::TRANSPARENT,
        }
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: theme::TEXT_PRIMARY,
        border: Border {
            radius: radius.into(),
            ..Default::default()
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

/// Card container with BACKGROUND color and rounded corners.
pub fn styled_card<'a, M: 'a>(content: Element<'a, M>) -> Element<'a, M> {
    container(content)
        .padding([12, 16])
        .width(Fill)
        .style(|_theme| container::Style {
            background: Some(Background::Color(theme::BACKGROUND)),
            border: Border {
                radius: theme::RADIUS_LG.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
