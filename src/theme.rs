use iced::Color;

// Helper macro: hex color from literal bytes
macro_rules! hex {
    ($r:literal, $g:literal, $b:literal) => {
        Color::from_rgb($r as f32 / 255.0, $g as f32 / 255.0, $b as f32 / 255.0)
    };
}

// Colors picked from the actual running Bitwarden app with a color picker.
// Note: SCSS values in variables.scss differ from rendered colors — always verify visually.

/// Main window / card background — #202733
pub const BACKGROUND: Color = hex!(0x20, 0x27, 0x33);

/// Header / title bar / sidebar background — #303946 (same value as BORDER)
pub const HEADER_BG: Color = hex!(0x30, 0x39, 0x46);

/// Detail pane / dark panel background — #121a27 (same value as SIDEBAR_SELECTED)
pub const CARD_BG: Color = hex!(0x12, 0x1a, 0x27);

/// Box/item hover background — #3c424e
pub const ITEM_HOVER: Color = hex!(0x3c, 0x42, 0x4e);

/// Primary brand color — brand-400 #6baefa
pub const ACCENT: Color = hex!(0x6b, 0xae, 0xfa);

/// Primary text — #ffffff
pub const TEXT_PRIMARY: Color = Color::WHITE;

/// Secondary text — #bac0ce
pub const TEXT_SECONDARY: Color = hex!(0xba, 0xc0, 0xce);

/// Muted / label text — #8898b5 (same value as TABLE_HEADER)
pub const TEXT_MUTED: Color = hex!(0x88, 0x98, 0xb5);

/// Separators / borders — #303946 (same value as HEADER_BG)
pub const BORDER: Color = hex!(0x30, 0x39, 0x46);

/// Sidebar selected item background — #121a27 (same value as CARD_BG)
pub const SIDEBAR_SELECTED: Color = hex!(0x12, 0x1a, 0x27);

/// Primary action button background — #65abff
pub const BUTTON_PRIMARY: Color = hex!(0x65, 0xab, 0xff);

/// Primary action button hover — #aac3ef
pub const BUTTON_PRIMARY_HOVER: Color = hex!(0xaa, 0xc3, 0xef);

/// Subtle button hover (toggle, logout) — #1f2a3c
pub const BUTTON_HOVER_SUBTLE: Color = hex!(0x1f, 0x2a, 0x3c);

/// Avatar circle background — cyan #2cd8d5
pub const AVATAR_BG: Color = hex!(0x2c, 0xd8, 0xd5);

/// Table column header text — #8898b5 (same value as TEXT_MUTED)
pub const TABLE_HEADER: Color = hex!(0x88, 0x98, 0xb5);

/// Title bar close button hover — standard Windows red #e81123
pub const TITLEBAR_CLOSE_HOVER: Color = hex!(0xe8, 0x11, 0x23);

/// Title bar minimize/maximize button hover — subtle highlight
pub const TITLEBAR_BTN_HOVER: Color = hex!(0x2d, 0x37, 0x48);

// Border radii
pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 8.0;
pub const RADIUS_PILL: f32 = 20.0;
