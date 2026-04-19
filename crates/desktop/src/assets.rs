//! Compile-time embedded assets. All binary resources in one place.

// Fonts
pub const FONT_MEDIUM: &[u8] = include_bytes!("../../../assets/inter/static/Inter_18pt-Medium.ttf");
pub const FONT_BOLD: &[u8] = include_bytes!("../../../assets/inter/static/Inter_18pt-Bold.ttf");
pub const BWI_FONT: &[u8] = include_bytes!("../../../assets/bwi-font.ttf");

// App icon
pub const ICON_PNG: &[u8] = include_bytes!("../../../assets/icon.png");

// SVGs
pub const LOGO_WHITE: &[u8] = include_bytes!("../../../assets/logo-white.svg");
pub const BG_LEFT: &[u8] = include_bytes!("../../../assets/bg-left.svg");
pub const BG_RIGHT: &[u8] = include_bytes!("../../../assets/bg-right.svg");
pub const LOCK_ICON: &[u8] = include_bytes!("../../../assets/lock-icon.svg");
pub const VAULT_ICON: &[u8] = include_bytes!("../../../assets/vault-icon.svg");
pub const WAVE_ICON: &[u8] = include_bytes!("../../../assets/wave-icon.svg");
pub const BITWARDEN_SHIELD: &[u8] = include_bytes!("../../../assets/bitwarden-shield.svg");
pub const PASSWORD_MANAGER_LOGO: &[u8] =
    include_bytes!("../../../assets/password-manager-logo.svg");

// Tray icon — vendored from `clients/apps/desktop/src/images/`. Per-OS so we
// get the best asset (ICO on Windows for multi-res, template PNG on macOS for
// menubar theming, regular PNG on Linux).
#[cfg(target_os = "windows")]
pub const TRAY_ICON: &[u8] = include_bytes!("../../../assets/tray/icon.ico");
#[cfg(target_os = "macos")]
pub const TRAY_ICON: &[u8] = include_bytes!("../../../assets/tray/icon-template.png");
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const TRAY_ICON: &[u8] = include_bytes!("../../../assets/tray/icon.png");
