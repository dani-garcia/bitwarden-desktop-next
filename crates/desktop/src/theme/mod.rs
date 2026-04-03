mod catalog;
mod dark;
mod light;

use iced::Color;

/// Structural border-radius constants (not theme-dependent).
pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 8.0;
pub const RADIUS_PILL: f32 = 20.0;

/// The application's custom theme, carrying a full set of semantic colors.
#[derive(Debug, Clone)]
pub struct AppTheme {
    pub colors: AppColors,
    name: String,
    mode: iced::theme::Mode,
}

/// User preference for which theme to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(dead_code)] // Light/Dark used when settings UI is wired
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

impl ThemePreference {
    /// Resolve the preference to a concrete `AppTheme`.
    /// For `System`, uses the given scheme from the OS (defaults to light if unavailable).
    pub fn resolve(self, scheme: Result<system_theme::ThemeScheme, system_theme::error::Error>) -> AppTheme {
        match self {
            ThemePreference::Light => AppTheme::light(),
            ThemePreference::Dark => AppTheme::dark(),
            ThemePreference::System => match scheme {
                Ok(system_theme::ThemeScheme::Dark) => AppTheme::dark(),
                _ => AppTheme::light(),
            },
        }
    }
}

impl AppTheme {
    pub fn dark() -> Self {
        Self {
            colors: AppColors::dark(),
            name: "Bitwarden Dark".into(),
            mode: iced::theme::Mode::Dark,
        }
    }

    pub fn light() -> Self {
        Self {
            colors: AppColors::light(),
            name: "Bitwarden Light".into(),
            mode: iced::theme::Mode::Light,
        }
    }
}

impl iced::theme::Base for AppTheme {
    fn default(preference: iced::theme::Mode) -> Self {
        match preference {
            iced::theme::Mode::Light => AppTheme::light(),
            _ => AppTheme::dark(),
        }
    }

    fn mode(&self) -> iced::theme::Mode {
        self.mode
    }

    fn base(&self) -> iced::theme::Style {
        iced::theme::Style {
            background_color: self.colors.background,
            text_color: self.colors.text_primary,
        }
    }

    fn palette(&self) -> Option<iced::theme::palette::Palette> {
        None
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// All semantic color tokens for the application.
#[derive(Debug, Clone, Copy)]
pub struct AppColors {
    /// Main window / card background
    pub background: Color,
    /// Header / title bar / sidebar background
    pub header_bg: Color,
    /// Detail pane / dark panel background
    pub card_bg: Color,
    /// Box/item hover background
    pub item_hover: Color,
    /// Primary brand color
    pub accent: Color,
    /// Primary text
    pub text_primary: Color,
    /// Secondary text
    pub text_secondary: Color,
    /// Muted / label text
    pub text_muted: Color,
    /// Separators / borders
    pub border: Color,
    /// Sidebar selected item background
    pub sidebar_selected: Color,
    /// Primary action button background
    pub button_primary: Color,
    /// Primary action button hover
    pub button_primary_hover: Color,
    /// Subtle button hover (toggle, logout)
    pub button_hover_subtle: Color,
    /// Avatar circle background
    pub avatar_bg: Color,
    /// Table column header text
    pub table_header: Color,
    /// Title bar close button hover
    pub titlebar_close_hover: Color,
    /// Title bar minimize/maximize button hover
    pub titlebar_btn_hover: Color,
    /// Navigation sidebar text/icon color (white on dark sidebar)
    pub nav_text: Color,
    /// Navigation sidebar item hover background
    pub nav_item_hover: Color,
}
