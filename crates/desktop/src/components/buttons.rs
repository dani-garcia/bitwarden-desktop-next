use iced::{
    Background, Border, Color, Element, Shadow,
    widget::{Button, button},
};

use crate::theme::{AppTheme, RADIUS_PILL, RADIUS_SM};

/// Filled primary action button — blue bg, dark text, pill shape.
///
/// Returns a `Button` so callers can chain `.on_press()`, `.width()`, `.padding()`, etc.
pub fn primary<'a, M: 'a>(content: impl Into<Element<'a, M, AppTheme>>) -> Button<'a, M, AppTheme> {
    button(content).style(|theme: &AppTheme, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => theme.colors.button_primary_hover,
            _ => theme.colors.button_primary,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: theme.colors.card_bg,
            border: Border::default().rounded(RADIUS_PILL),
            shadow: Shadow::default(),
            snap: false,
        }
    })
}

/// Outlined secondary button — transparent bg, accent border+text, pill shape.
pub fn secondary<'a, M: 'a>(
    content: impl Into<Element<'a, M, AppTheme>>,
) -> Button<'a, M, AppTheme> {
    button(content).style(|theme: &AppTheme, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => theme.colors.button_hover_subtle,
            _ => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: theme.colors.accent,
            border: Border::default()
                .color(theme.colors.accent)
                .width(1.0)
                .rounded(RADIUS_PILL),
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
/// - `hover_bg`: background color on hover (e.g. `colors.item_hover` or `colors.nav_item_hover`)
/// - `radius`: border radius
pub fn ghost<'a, M: 'a>(
    content: impl Into<Element<'a, M, AppTheme>>,
    is_active: bool,
    active_bg: Color,
    hover_bg: Color,
    radius: f32,
) -> Button<'a, M, AppTheme> {
    button(content).style(move |theme: &AppTheme, status| {
        let bg = if is_active {
            active_bg
        } else {
            match status {
                button::Status::Hovered | button::Status::Pressed => hover_bg,
                _ => Color::TRANSPARENT,
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: theme.colors.text_primary,
            border: Border::default().rounded(radius),
            shadow: Shadow::default(),
            snap: false,
        }
    })
}

/// Small icon-only ghost button (transparent, hover highlight, small radius).
/// Shorthand for `ghost()` with no active state and content-area hover color.
pub fn ghost_icon<'a, M: 'a>(
    content: impl Into<Element<'a, M, AppTheme>>,
    hover_bg: Color,
) -> Button<'a, M, AppTheme> {
    ghost(content, false, Color::TRANSPARENT, hover_bg, RADIUS_SM)
}

/// Fully transparent button with no hover effect.
/// Use for composite clickable areas (e.g. account switcher avatar).
pub fn transparent<'a, M: 'a>(
    content: impl Into<Element<'a, M, AppTheme>>,
) -> Button<'a, M, AppTheme> {
    button(content).style(|theme: &AppTheme, _status| button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: theme.colors.text_primary,
        border: Border::default(),
        shadow: Shadow::default(),
        snap: false,
    })
}
