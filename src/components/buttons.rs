use iced::{
    Background, Border, Color, Element, Shadow,
    widget::{button, Button},
};

use crate::theme::{AppTheme, RADIUS_PILL, RADIUS_SM};

/// Filled primary action button — blue bg, dark text, pill shape.
///
/// Returns a `Button` so callers can chain `.on_press()`, `.width()`, `.padding()`, etc.
pub fn primary<'a, M: 'a>(
    content: impl Into<Element<'a, M, AppTheme>>,
) -> Button<'a, M, AppTheme> {
    button(content)
        .style(|theme: &AppTheme, status| {
            let bg = match status {
                button::Status::Hovered | button::Status::Pressed => theme.colors.button_primary_hover,
                _ => theme.colors.button_primary,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: theme.colors.card_bg,
                border: Border {
                    radius: RADIUS_PILL.into(),
                    ..Default::default()
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
}

/// Outlined secondary button — transparent bg, accent border+text, pill shape.
pub fn secondary<'a, M: 'a>(
    content: impl Into<Element<'a, M, AppTheme>>,
) -> Button<'a, M, AppTheme> {
    button(content)
        .style(|theme: &AppTheme, status| {
            let bg = match status {
                button::Status::Hovered | button::Status::Pressed => theme.colors.button_hover_subtle,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: theme.colors.accent,
                border: Border {
                    color: theme.colors.accent,
                    width: 1.0,
                    radius: RADIUS_PILL.into(),
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
}

/// Transparent button with hover highlight.
/// Use for sidebar items, icon actions, menu items, etc.
///
/// - `is_active`: whether to show the active/selected background
/// - `active_bg`: background color when active (e.g. `colors.sidebar_selected`)
/// - `radius`: border radius
pub fn ghost<'a, M: 'a>(
    content: impl Into<Element<'a, M, AppTheme>>,
    is_active: bool,
    active_bg: Color,
    radius: f32,
) -> Button<'a, M, AppTheme> {
    button(content)
        .style(move |theme: &AppTheme, status| {
            let bg = if is_active {
                active_bg
            } else {
                match status {
                    button::Status::Hovered | button::Status::Pressed => theme.colors.item_hover,
                    _ => Color::TRANSPARENT,
                }
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: theme.colors.text_primary,
                border: Border {
                    radius: radius.into(),
                    ..Default::default()
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
}

/// Small icon-only ghost button (transparent, hover highlight, small radius).
/// Shorthand for `ghost(content, false, Color::TRANSPARENT, RADIUS_SM)`.
pub fn ghost_icon<'a, M: 'a>(
    content: impl Into<Element<'a, M, AppTheme>>,
) -> Button<'a, M, AppTheme> {
    ghost(content, false, Color::TRANSPARENT, RADIUS_SM)
}

/// Fully transparent button with no hover effect.
/// Use for composite clickable areas (e.g. account switcher avatar).
pub fn transparent<'a, M: 'a>(
    content: impl Into<Element<'a, M, AppTheme>>,
) -> Button<'a, M, AppTheme> {
    button(content)
        .style(|theme: &AppTheme, _status| button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: theme.colors.text_primary,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: false,
        })
}
