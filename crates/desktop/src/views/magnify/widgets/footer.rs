//! Bottom hint bar shown in the searching state — keyboard shortcut legend
//! plus a right-aligned shield logo, matching `designs/magnify/searching.jpg`.

use iced::{
    Alignment, Element, Fill, Length,
    widget::{Space, container, row, svg},
};

use crate::{
    theme::{AppColors, AppTheme},
    views::magnify::{
        message::MagnifyMessage,
        widgets::chip::{hint, keybind_text},
    },
};

pub fn view<'a>(colors: &'a AppColors) -> Element<'a, MagnifyMessage, AppTheme> {
    let row = row![
        hint(
            keybind_text("\u{2191}\u{2193}", colors),
            crate::fl!("magnify-hint-navigate"),
            colors,
        ),
        hint(
            keybind_text("Ctrl+C", colors),
            crate::fl!("magnify-copy-password"),
            colors,
        ),
        hint(
            keybind_text("Ctrl+\u{21E7}C", colors),
            crate::fl!("magnify-copy-username"),
            colors,
        ),
        Space::new().width(Fill),
        // Tint the shield to the theme's muted text color so it stays
        // legible on both dark and light surfaces (the asset itself has
        // `fill="white"` baked in).
        svg(svg::Handle::from_memory(crate::assets::BITWARDEN_SHIELD))
            .width(14)
            .height(14)
            .style(|theme: &AppTheme, _status| svg::Style {
                color: Some(theme.colors.text_muted),
            }),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    container(row)
        .padding([6, 16])
        .width(Fill)
        .height(Length::Fixed(crate::views::magnify::dims::FOOTER_HEIGHT))
        .into()
}
