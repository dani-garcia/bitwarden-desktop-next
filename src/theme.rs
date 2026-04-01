use iced::Color;

// Helper macro: hex color from literal bytes
macro_rules! hex {
    ($r:literal, $g:literal, $b:literal) => {
        Color::from_rgb($r as f32 / 255.0, $g as f32 / 255.0, $b as f32 / 255.0)
    };
}

// Colors picked from the actual running Bitwarden app with a color picker.
// Note: SCSS values in variables.scss differ from rendered colors — always verify visually.

/// Main window background — #070b18
pub const BACKGROUND: Color = hex!(0x07, 0x0b, 0x18);

/// Sidebar / nav background — #1d293d
pub const SIDEBAR_BG: Color = hex!(0x1d, 0x29, 0x3d);

/// Header / account switcher bar background — #1e2939
pub const HEADER_BG: Color = hex!(0x1e, 0x29, 0x39);

/// Card / box background — #101828
pub const CARD_BG: Color = hex!(0x10, 0x18, 0x28);

/// Box/item hover background — #3c424e
pub const ITEM_HOVER: Color = hex!(0x3c, 0x42, 0x4e);

/// Primary brand color — brand-400 #6baefa
pub const ACCENT: Color = hex!(0x6b, 0xae, 0xfa);

/// Primary text — #ffffff
pub const TEXT_PRIMARY: Color = Color::WHITE;

/// Secondary text — #bac0ce
pub const TEXT_SECONDARY: Color = hex!(0xba, 0xc0, 0xce);

/// Muted / disabled text — #6e788a
pub const TEXT_MUTED: Color = hex!(0x6e, 0x78, 0x8a);

/// Primary border — #4c525f
pub const BORDER: Color = hex!(0x4c, 0x52, 0x5f);

/// Input background — #1f242e
pub const INPUT_BG: Color = hex!(0x1f, 0x24, 0x2e);

/// Selected item / active accent — brand-700 #175ddc
pub const SELECTED_BG: Color = hex!(0x17, 0x5d, 0xdc);
