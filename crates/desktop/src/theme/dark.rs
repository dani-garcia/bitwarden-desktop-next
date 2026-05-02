use iced::{Color, color};

use super::AppColors;

impl AppColors {
    /// Dark theme — colors picked from the running Bitwarden app.
    pub fn dark() -> Self {
        Self {
            background: color!(0x202733),
            header_bg: color!(0x303946),
            card_bg: color!(0x121a27),
            item_hover: color!(0x3c424e),
            accent: color!(0x6baefa),
            text_primary: Color::WHITE,
            text_secondary: color!(0xbac0ce),
            text_muted: color!(0x8898b5),
            border: color!(0x303946),
            sidebar_selected: color!(0x121a27),
            magnify_selected: color!(0x215fb4),
            surface_selected: color!(0x2b3447),
            button_primary: color!(0x65abff),
            button_primary_hover: color!(0xaac3ef),
            button_hover_subtle: color!(0x1f2a3c),
            avatar_bg: color!(0x2cd8d5),
            table_header: color!(0x8898b5),
            titlebar_close_hover: color!(0xe81123),
            titlebar_btn_hover: color!(0x2d3748),
            nav_text: Color::WHITE,
            nav_item_hover: color!(0x3c424e),
            toast_info_bg: color!(0x175ddc),
            toast_success_bg: color!(0x3bb360),
            toast_warning_bg: color!(0xbf8b00),
            toast_error_bg: color!(0xc83c3c),
            danger: color!(0xc83c3c),
            scrollbar_thumb: color!(0x556379),
        }
    }
}
