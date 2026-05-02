use iced::{Color, color};

use super::AppColors;

impl AppColors {
    /// Light theme — colors picked from `designs/Desktop 2025/vault-view item.png`.
    pub fn light() -> Self {
        Self {
            background: Color::WHITE,
            header_bg: color!(0x173792),
            card_bg: color!(0xf4f6f9),
            item_hover: color!(0xe7e9ef),
            accent: color!(0x165ddc),
            text_primary: color!(0x1a2029),
            text_secondary: color!(0x5a6d91),
            text_muted: color!(0x5a6d91),
            border: color!(0xe7e9ef),
            sidebar_selected: color!(0x011066),
            magnify_selected: color!(0xa4d0fc),
            surface_selected: color!(0xd1d9e8),
            button_primary: color!(0x165ddc),
            button_primary_hover: color!(0x1248b0),
            button_hover_subtle: color!(0xe7e9ef),
            avatar_bg: color!(0x2cd8d5),
            table_header: color!(0x5a6d91),
            titlebar_close_hover: color!(0xe81123),
            titlebar_btn_hover: color!(0x011066),
            nav_text: Color::WHITE,
            nav_item_hover: color!(0x0a2878),
            toast_info_bg: color!(0x175ddc),
            toast_success_bg: color!(0x3bb360),
            toast_warning_bg: color!(0xbf8b00),
            toast_error_bg: color!(0xc83c3c),
            scrollbar_thumb: color!(0xc4c9d4),
        }
    }
}
