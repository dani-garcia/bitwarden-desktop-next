// The generated `bootstrap_icons_generated.rs` below defines an `Icon`
// constant per Bootstrap glyph; most are unused. `expect` would require
// every constant to be genuinely unused, but many are referenced.
#![allow(dead_code)]

use iced::{
    Element, Font,
    widget::{text, text_input},
};

/// Bootstrap Icons font, loaded in main.rs via `.font()`.
pub const FONT: Font = Font::new("bootstrap-icons");

/// A typed icon identifier wrapping a Bootstrap Icons codepoint.
#[derive(Copy, Clone)]
pub struct Icon(char);

impl Icon {
    pub fn render<'a, M: 'a, Theme: text::Catalog + 'a>(
        &self,
        size: f32,
        color: iced::Color,
    ) -> Element<'a, M, Theme>
    where
        Theme::Class<'a>: From<text::StyleFn<'a, Theme>>,
    {
        text(self.0).font(FONT).size(size).color(color).into()
    }

    pub fn input_icon(self, size: f32, side: text_input::Side) -> text_input::Icon<Font> {
        text_input::Icon {
            font: FONT,
            code_point: self.0,
            size: Some(size.into()),
            spacing: 8.0,
            side,
        }
    }

    /// Raw codepoint — prefer `render()` or `input_icon()` in public APIs.
    /// Used by `toast` for inline icon-in-styled-text composition and by
    /// `title_bar::window_chrome` platform-specific chrome char constants.
    pub(crate) const fn char(self) -> char {
        self.0
    }
}

// Auto-generated from the CSS at build time. To update: replace
// assets/bootstrap-icons-*.css and *.ttf, bump FONT_VERSION in build.rs.
include!(concat!(env!("OUT_DIR"), "/bootstrap_icons_generated.rs"));

// ── Bitwarden Icons (bwi) font ─────────────────────────────────────────────

pub const BWI_FONT: Font = Font::new("bwi-font");

#[derive(Copy, Clone)]
pub struct BwiIcon(char);

impl BwiIcon {
    pub fn render<'a, M: 'a, Theme: text::Catalog + 'a>(
        &self,
        size: f32,
        color: iced::Color,
    ) -> Element<'a, M, Theme>
    where
        Theme::Class<'a>: From<text::StyleFn<'a, Theme>>,
    {
        text(self.0).font(BWI_FONT).size(size).color(color).into()
    }

    /// Raw codepoint — prefer `render()` in public APIs.
    pub(crate) const fn char(self) -> char {
        self.0
    }
}

// Codepoints from clients/libs/angular/src/scss/bwicons/styles/style.scss
pub const BWI_VAULT: BwiIcon = BwiIcon('\u{f106}');
pub const BWI_USER: BwiIcon = BwiIcon('\u{f107}');
pub const BWI_SEND: BwiIcon = BwiIcon('\u{f119}');
pub const BWI_GENERATE: BwiIcon = BwiIcon('\u{f138}');
pub const BWI_IMPORT: BwiIcon = BwiIcon('\u{f108}');
pub const BWI_DOWNLOAD: BwiIcon = BwiIcon('\u{f146}');
pub const BWI_STAR: BwiIcon = BwiIcon('\u{f110}');
pub const BWI_LOGIN: BwiIcon = BwiIcon('\u{f12a}');
pub const BWI_CREDIT_CARD: BwiIcon = BwiIcon('\u{f14d}');
pub const BWI_IDENTITY: BwiIcon = BwiIcon('\u{f132}');
pub const BWI_NOTE: BwiIcon = BwiIcon('\u{f123}');
pub const BWI_KEY: BwiIcon = BwiIcon('\u{f130}');
pub const BWI_ARCHIVE: BwiIcon = BwiIcon('\u{f167}');
pub const BWI_TRASH: BwiIcon = BwiIcon('\u{f14b}');
pub const BWI_ANGLE_LEFT: BwiIcon = BwiIcon('\u{f16b}');
pub const BWI_ANGLE_RIGHT: BwiIcon = BwiIcon('\u{f16a}');
pub const BWI_ANGLE_DOWN: BwiIcon = BwiIcon('\u{f16c}');
pub const BWI_ANGLE_UP: BwiIcon = BwiIcon('\u{f168}');
pub const BWI_COPY: BwiIcon = BwiIcon('\u{f14e}');
pub const BWI_CLOSE: BwiIcon = BwiIcon('\u{f151}');
pub const BWI_EXTERNAL_LINK: BwiIcon = BwiIcon('\u{f13e}');
pub const BWI_EYE: BwiIcon = BwiIcon('\u{f104}');
pub const BWI_EYE_SLASH: BwiIcon = BwiIcon('\u{f105}');
pub const BWI_EDIT: BwiIcon = BwiIcon('\u{f142}');
pub const BWI_LOCK: BwiIcon = BwiIcon('\u{f12b}');
pub const BWI_HANDSHAKE: BwiIcon = BwiIcon('\u{f135}');
