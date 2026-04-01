use iced::Color;

// Helper macro: hex color from literal bytes
macro_rules! hex {
    ($r:literal, $g:literal, $b:literal) => {
        Color::from_rgb($r as f32 / 255.0, $g as f32 / 255.0, $b as f32 / 255.0)
    };
}

// All values sourced from the official Bitwarden dark theme.
// SCSS: clients/apps/desktop/src/scss/variables.scss (dark theme map, lines 107-166)
// Tailwind: clients/libs/components/src/tw-theme.css (dark mode section)

/// Main window background — picked from actual app: #070b18
pub const BACKGROUND: Color = hex!(0x07, 0x0b, 0x18);

/// Sidebar / nav background — `--color-nav-bg-primary` → gray-800 #1d293d
pub const SIDEBAR_BG: Color = hex!(0x1d, 0x29, 0x3d);

/// Header / account switcher bar background — picked from actual app: #1e2939
pub const HEADER_BG: Color = hex!(0x1e, 0x29, 0x39);

/// Card / box background — picked from actual app: #101828
pub const CARD_BG: Color = hex!(0x10, 0x18, 0x28);

/// Box/item hover background — `boxBackgroundHoverColor` #3c424e
pub const ITEM_HOVER: Color = hex!(0x3c, 0x42, 0x4e);

/// Primary brand color (dark mode) — `--color-bg-brand` → brand-400 #6baefa
pub const ACCENT: Color = hex!(0x6b, 0xae, 0xfa);

/// Primary brand hover — `--color-bg-brand-strong` → brand-200 #bedbff
pub const ACCENT_HOVER: Color = hex!(0xbe, 0xdb, 0xff);

/// Primary text — `textColor` #ffffff
pub const TEXT_PRIMARY: Color = Color::WHITE;

/// Secondary / muted text — `mutedColor` #bac0ce
pub const TEXT_SECONDARY: Color = hex!(0xba, 0xc0, 0xce);

/// Heading text (same as muted in dark) — `headingColor` #bac0ce
pub const TEXT_MUTED: Color = hex!(0x6e, 0x78, 0x8a);

/// Primary border — `boxBorderColor` / `inputBorderColor` #4c525f
pub const BORDER: Color = hex!(0x4c, 0x52, 0x5f);

/// Header border — `--color-nav-bg-primary-strong` → gray-600 #45556c
pub const HEADER_BORDER: Color = hex!(0x45, 0x55, 0x6c);

/// Danger color — `dangerColor` #ff8d85
pub const DANGER: Color = hex!(0xff, 0x8d, 0x85);

/// Input background — darker than card so it's visible against card bg
pub const INPUT_BG: Color = hex!(0x1f, 0x24, 0x2e);

/// Button background (secondary) — `buttonBackgroundColor` #272b32
pub const BUTTON_BG: Color = hex!(0x27, 0x2b, 0x32);

/// Button border — `buttonBorderColor` #4c525f
pub const BUTTON_BORDER: Color = hex!(0x4c, 0x52, 0x5f);

/// Button text — `buttonColor` #bac0ce
pub const BUTTON_TEXT: Color = hex!(0xba, 0xc0, 0xce);

/// Selected item / active accent — brand-700 #175ddc
pub const SELECTED_BG: Color = hex!(0x17, 0x5d, 0xdc);

/// List item background — `listItemBackgroundColor` #1f242e (same as main bg)
pub const LIST_ITEM_BG: Color = hex!(0x1f, 0x24, 0x2e);

/// Account switcher background — `accountSwitcherBackgroundColor` #2f343d
pub const ACCOUNT_SWITCHER_BG: Color = hex!(0x2f, 0x34, 0x3d);
