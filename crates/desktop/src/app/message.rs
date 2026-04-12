use crate::views::{
    about::AboutMessage, login::LoginMessage, title_bar::TitleBarMessage, vault::VaultMessage,
};

// ── Top-level Message ──────────────────────────────────────────────────────
//
// Six variants. The three sub-view wrappers route user interaction + async
// completions into the view that owns the underlying state. `About` handles
// the About child window (stateless). `Window` carries per-window OS events
// (every variant takes a `window::Id` for multi-window dispatch). `System`
// carries global signals that aren't tied to a specific window.

#[derive(Debug, Clone)]
pub enum Message {
    Login(LoginMessage),
    Vault(VaultMessage),
    TitleBar(TitleBarMessage),
    About(AboutMessage),
    Window(WindowMessage),
    System(SystemMessage),
}

/// Per-window OS events. Every variant carries `window::Id` so the router
/// can dispatch to the correct window in daemon (multi-window) mode.
#[derive(Debug, Clone)]
pub enum WindowMessage {
    Opened(iced::window::Id),
    GotRawId(iced::window::Id, u64),
    Closed(iced::window::Id),
    KeyPressed(iced::window::Id, iced::keyboard::Event),
}

/// Global signals that aren't tied to a specific window.
#[derive(Debug, Clone)]
pub enum SystemMessage {
    /// 16ms tick driving the muda native menu poll. Only emitted when a
    /// native menu handle is attached.
    PollNativeMenu,
    /// OS-level light/dark theme changed. `ThemePreference::System` follows
    /// this; explicit Light/Dark preferences ignore it.
    ThemeChanged,
    /// User dismissed a toast via the x button or auto-dismiss expiry.
    CloseToast(usize),
}

/// Per-window metadata. Keyed by `iced::window::Id` in a `HashMap` on `App`.
#[derive(Debug)]
pub struct WindowInfo {
    pub kind: WindowKind,
    pub fullscreen: bool,
    pub maximized: bool,
}

impl WindowInfo {
    pub fn new(kind: WindowKind) -> Self {
        Self {
            kind,
            fullscreen: false,
            maximized: false,
        }
    }
}

/// Discriminant for each kind of window the app can have open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    Main,
    About,
}
