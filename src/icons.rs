use iced::widget::text;
use iced::{Element, Font};

/// Bootstrap Icons font, loaded in main.rs via `.font()`
pub const FONT: Font = Font::with_name("bootstrap-icons");

/// A typed icon identifier wrapping a Bootstrap Icons codepoint.
#[derive(Copy, Clone)]
pub struct Icon(char);

impl Icon {
    /// Render this icon as an iced Element at the given size and color.
    pub fn render<'a, M: 'a>(&self, size: f32, color: iced::Color) -> Element<'a, M> {
        text(self.0).font(FONT).size(size).color(color).into()
    }
}

// All icons + FONT_BYTES, auto-generated from the CSS at build time.
// To update: replace assets/bootstrap-icons-*.css and *.ttf, update FONT_VERSION in build.rs.
include!(concat!(env!("OUT_DIR"), "/bootstrap_icons_generated.rs"));
