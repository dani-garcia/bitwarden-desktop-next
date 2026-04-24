//! Per-window metadata stored on `App`. Separate from `message.rs` because
//! these types carry state, not messages — `WindowInfo` lives in a map
//! keyed by `iced::window::Id` and is mutated on resize / maximize /
//! fullscreen transitions.

/// Per-window metadata. Keyed by `iced::window::Id` in a `HashMap` on `App`.
#[derive(Debug)]
pub struct WindowInfo {
    pub kind: WindowKind,
    pub fullscreen: bool,
    pub maximized: bool,
    /// Last reported logical size. Initialized from `window::Settings.size`
    /// at creation; updated on `window::Event::Resized`.
    pub size: iced::Size,
}

impl WindowInfo {
    pub fn new(kind: WindowKind, size: iced::Size) -> Self {
        Self {
            kind,
            fullscreen: false,
            maximized: false,
            size,
        }
    }
}

/// Discriminant for each kind of window the app can have open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    Main,
    About,
}
