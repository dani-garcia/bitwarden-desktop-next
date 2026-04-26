use iced::{
    Background, Border, Color, Element, Shadow,
    widget::{Button, button},
};

use crate::{
    components::icons,
    theme::{AppColors, AppTheme, RADIUS_PILL, RADIUS_SM},
};

/// Shared builder for `button::Style`. Centralizes `shadow: Shadow::default()`
/// and `snap: false` — the latter is a mandatory field whose omission produces
/// a confusing compile error pointing at the struct literal.
fn style(bg: Color, text_color: Color, border: Border) -> button::Style {
    button::Style {
        background: Some(Background::Color(bg)),
        text_color,
        border,
        shadow: Shadow::default(),
        snap: false,
    }
}

/// Filled primary action button — blue bg, dark text, pill shape.
pub fn primary<'a, M: 'a>(content: impl Into<Element<'a, M, AppTheme>>) -> Button<'a, M, AppTheme> {
    button(content).style(|theme: &AppTheme, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => theme.colors.button_primary_hover,
            _ => theme.colors.button_primary,
        };
        style(
            bg,
            theme.colors.card_bg,
            Border::default().rounded(RADIUS_PILL),
        )
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
        style(
            bg,
            theme.colors.accent,
            Border::default()
                .color(theme.colors.accent)
                .width(1.0)
                .rounded(RADIUS_PILL),
        )
    })
}

/// Transparent button with hover highlight, used for sidebar items, icon
/// actions, menu items, etc.
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
        style(
            bg,
            theme.colors.text_primary,
            Border::default().rounded(radius),
        )
    })
}

/// Small icon-only ghost button (transparent, hover highlight, small radius).
pub fn ghost_icon<'a, M: 'a>(
    content: impl Into<Element<'a, M, AppTheme>>,
    hover_bg: Color,
) -> Button<'a, M, AppTheme> {
    ghost(content, false, Color::TRANSPARENT, hover_bg, RADIUS_SM)
}

/// Fully transparent button with no hover effect; used for composite
/// clickable areas (e.g. account switcher avatar).
pub fn transparent<'a, M: 'a>(
    content: impl Into<Element<'a, M, AppTheme>>,
) -> Button<'a, M, AppTheme> {
    button(content).style(|theme: &AppTheme, _status| {
        style(
            Color::TRANSPARENT,
            theme.colors.text_primary,
            Border::default(),
        )
    })
}

/// Small 18 px icon button with ghost-style hover; used for the copy /
/// launch / delete icon buttons in list rows and fields.
pub fn icon_button<'a, M: 'a + Clone>(
    icon: icons::BwiIcon,
    msg: M,
    colors: &AppColors,
) -> Element<'a, M, AppTheme> {
    ghost_icon(icon.render(18.0, colors.text_primary), colors.item_hover)
        .on_press(msg)
        .padding([6, 6])
        .into()
}
