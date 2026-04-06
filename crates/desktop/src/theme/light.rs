use iced::Color;

use super::AppColors;

// Helper macro: hex color from literal bytes
macro_rules! hex {
    ($r:literal, $g:literal, $b:literal) => {
        Color::from_rgb($r as f32 / 255.0, $g as f32 / 255.0, $b as f32 / 255.0)
    };
}

impl AppColors {
    /// Light theme — colors picked from `designs/Desktop 2025/vault-view item.png`.
    pub fn light() -> Self {
        Self {
            background: Color::WHITE,
            header_bg: hex!(0x17, 0x37, 0x92),
            card_bg: hex!(0xf4, 0xf6, 0xf9),
            item_hover: hex!(0xe7, 0xe9, 0xef),
            accent: hex!(0x16, 0x5d, 0xdc),
            text_primary: hex!(0x1a, 0x20, 0x29),
            text_secondary: hex!(0x5a, 0x6d, 0x91),
            text_muted: hex!(0x5a, 0x6d, 0x91),
            border: hex!(0xe7, 0xe9, 0xef),
            sidebar_selected: hex!(0x01, 0x10, 0x66),
            button_primary: hex!(0x16, 0x5d, 0xdc),
            button_primary_hover: hex!(0x12, 0x48, 0xb0),
            button_hover_subtle: hex!(0xe7, 0xe9, 0xef),
            avatar_bg: hex!(0x2c, 0xd8, 0xd5),
            table_header: hex!(0x5a, 0x6d, 0x91),
            titlebar_close_hover: hex!(0xe8, 0x11, 0x23),
            titlebar_btn_hover: hex!(0x01, 0x10, 0x66),
            nav_text: Color::WHITE,
            nav_item_hover: hex!(0x0a, 0x28, 0x78),
            toast_info_bg: hex!(0x17, 0x5d, 0xdc),
            toast_success_bg: hex!(0x3b, 0xb3, 0x60),
            toast_warning_bg: hex!(0xbf, 0x8b, 0x00),
            toast_error_bg: hex!(0xc8, 0x3c, 0x3c),
        }
    }
}
