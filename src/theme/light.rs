use iced::Color;

use super::AppColors;

// Helper macro: hex color from literal bytes
macro_rules! hex {
    ($r:literal, $g:literal, $b:literal) => {
        Color::from_rgb($r as f32 / 255.0, $g as f32 / 255.0, $b as f32 / 255.0)
    };
}

impl AppColors {
    /// Light theme — placeholder with inverted/lightened dark palette.
    /// TODO: pick real colors from the Bitwarden app's light theme.
    pub fn light() -> Self {
        Self {
            background: hex!(0xf2, 0xf3, 0xf5),
            header_bg: hex!(0xe8, 0xea, 0xed),
            card_bg: Color::WHITE,
            item_hover: hex!(0xda, 0xdd, 0xe3),
            accent: hex!(0x17, 0x5d, 0xdc),
            text_primary: hex!(0x1a, 0x1a, 0x1a),
            text_secondary: hex!(0x5a, 0x62, 0x70),
            text_muted: hex!(0x7a, 0x83, 0x92),
            border: hex!(0xd0, 0xd4, 0xdb),
            sidebar_selected: hex!(0xdf, 0xe3, 0xeb),
            button_primary: hex!(0x17, 0x5d, 0xdc),
            button_primary_hover: hex!(0x12, 0x4a, 0xb0),
            button_hover_subtle: hex!(0xe3, 0xe7, 0xef),
            avatar_bg: hex!(0x2c, 0xd8, 0xd5),
            table_header: hex!(0x7a, 0x83, 0x92),
            titlebar_close_hover: hex!(0xe8, 0x11, 0x23),
            titlebar_btn_hover: hex!(0xd8, 0xdb, 0xe2),
        }
    }
}
