use iced::Color;

use super::AppColors;

// Helper macro: hex color from literal bytes
macro_rules! hex {
    ($r:literal, $g:literal, $b:literal) => {
        Color::from_rgb($r as f32 / 255.0, $g as f32 / 255.0, $b as f32 / 255.0)
    };
}

impl AppColors {
    /// Dark theme — colors picked from the actual running Bitwarden app.
    pub fn dark() -> Self {
        Self {
            background: hex!(0x20, 0x27, 0x33),
            header_bg: hex!(0x30, 0x39, 0x46),
            card_bg: hex!(0x12, 0x1a, 0x27),
            item_hover: hex!(0x3c, 0x42, 0x4e),
            accent: hex!(0x6b, 0xae, 0xfa),
            text_primary: Color::WHITE,
            text_secondary: hex!(0xba, 0xc0, 0xce),
            text_muted: hex!(0x88, 0x98, 0xb5),
            border: hex!(0x30, 0x39, 0x46),
            sidebar_selected: hex!(0x12, 0x1a, 0x27),
            button_primary: hex!(0x65, 0xab, 0xff),
            button_primary_hover: hex!(0xaa, 0xc3, 0xef),
            button_hover_subtle: hex!(0x1f, 0x2a, 0x3c),
            avatar_bg: hex!(0x2c, 0xd8, 0xd5),
            table_header: hex!(0x88, 0x98, 0xb5),
            titlebar_close_hover: hex!(0xe8, 0x11, 0x23),
            titlebar_btn_hover: hex!(0x2d, 0x37, 0x48),
        }
    }
}
