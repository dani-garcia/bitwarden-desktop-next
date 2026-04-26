mod catalog;
mod dark;
mod light;

use iced::Color;

/// Structural border-radius constants (not theme-dependent).
pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 8.0;
/// Larger surface radius — used by floating launcher-style windows whose
/// outer container is the only painted region (Magnify).
pub const RADIUS_XL: f32 = 12.0;
pub const RADIUS_PILL: f32 = 20.0;

/// Surface translucency for the Magnify launcher window. Theme-invariant
/// today; promote to a per-theme `AppColors` token if a future theme needs
/// a different blend.
pub const MAGNIFY_SURFACE_ALPHA: f32 = 0.95;

/// The application's custom theme, carrying a full set of semantic colors.
#[derive(Debug, Clone)]
pub struct AppTheme {
    pub colors: AppColors,
    name: &'static str,
    mode: iced::theme::Mode,
    /// When `true`, [`Base::base`] returns a transparent window background
    /// so the OS-level window can show through outside any container fills.
    /// Used by the Magnify launcher window so its rounded outer container
    /// is the only visible region.
    transparent_background: bool,
}

/// User preference for which theme to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePreference {
    /// Resolve the preference to a concrete `AppTheme`. For `System`, uses
    /// the given OS scheme (defaults to light if unavailable).
    pub fn resolve(
        self,
        scheme: Result<system_theme::ThemeScheme, system_theme::error::Error>,
    ) -> AppTheme {
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
            name: "Bitwarden Dark",
            mode: iced::theme::Mode::Dark,
            transparent_background: false,
        }
    }

    pub fn light() -> Self {
        Self {
            colors: AppColors::light(),
            name: "Bitwarden Light",
            mode: iced::theme::Mode::Light,
            transparent_background: false,
        }
    }

    /// Return a clone with `transparent_background` set so
    /// `iced::theme::Base::base()` paints a transparent window background.
    pub fn with_transparent_background(mut self) -> Self {
        self.transparent_background = true;
        self
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
            background_color: if self.transparent_background {
                Color::TRANSPARENT
            } else {
                self.colors.background
            },
            text_color: self.colors.text_primary,
        }
    }

    fn seed(&self) -> Option<iced::theme::palette::Seed> {
        None
    }

    fn name(&self) -> &str {
        self.name
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
    /// Highlight color for the selected row in the Magnify launcher.
    /// Brighter than `accent` so a row reads as strongly highlighted
    /// against the launcher's translucent dark surface.
    pub magnify_selected: Color,
    /// Selected row background in light-surface contexts (e.g. the settings
    /// modal sidebar). Darker than `item_hover` so selection reads as
    /// distinct from hover. Separate from `sidebar_selected` (tuned for
    /// dark navy nav rails).
    pub surface_selected: Color,
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
    /// Toast background — informational severity
    pub toast_info_bg: Color,
    /// Toast background — success severity
    pub toast_success_bg: Color,
    /// Toast background — warning severity
    pub toast_warning_bg: Color,
    /// Toast background — error severity
    pub toast_error_bg: Color,
    /// Scrollbar thumb (the draggable part of the scrollable rail)
    pub scrollbar_thumb: Color,
}
