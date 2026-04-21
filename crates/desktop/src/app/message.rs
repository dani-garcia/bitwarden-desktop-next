use std::sync::Arc;

use crate::{
    sdk::ClientManager,
    views::{
        about::AboutMessage, login::LoginMessage, settings::SettingsMessage,
        title_bar::TitleBarMessage, vault::VaultMessage,
    },
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
    Settings(SettingsMessage),
    Window(WindowMessage),
    System(SystemMessage),
}

/// Per-window OS events. Every variant carries `window::Id` so the router
/// can dispatch to the correct window in daemon (multi-window) mode.
#[derive(Debug, Clone)]
pub enum WindowMessage {
    Opened(iced::window::Id),
    GotRawId(iced::window::Id, u64),
    /// OS emitted a close request (X button on native title bar, Alt+F4,
    /// window-list "Close", etc.). Funnels into the same `WindowAction::Close`
    /// path the custom title-bar X-button uses, so close-to-tray applies
    /// uniformly. `exit_on_close_request: false` on the main window defers
    /// the real close to our handler.
    CloseRequested(iced::window::Id),
    Closed(iced::window::Id),
    KeyPressed(iced::window::Id, iced::keyboard::Event),
    Resized(iced::window::Id, iced::Size),
}

/// Global signals that aren't tied to a specific window.
#[derive(Debug, Clone)]
pub enum SystemMessage {
    /// A muda-managed menu item was activated — either from the native app
    /// menu or the tray context menu (they share one global receiver). The
    /// handler looks the id up on `NativeMenuHandle` / `TrayHandle` to
    /// decide which action to run.
    MudaEvent(muda::MenuEvent),
    /// Left-click released on the tray icon — emitted by the pump in
    /// [`crate::tray::click_stream`] which already filters to the
    /// click-to-toggle case.
    TrayClick(crate::tray::TrayAction),
    /// OS-level light/dark theme changed. `ThemePreference::System` follows
    /// this; explicit Light/Dark preferences ignore it.
    ThemeChanged,
    /// User dismissed a toast via the x button or auto-dismiss expiry.
    CloseToast(usize),
    /// The background `ClientManager::load` task finished. Swaps the placeholder
    /// `ClientManager::empty()` for the fully-populated one and transitions out
    /// of `Screen::Loading`.
    ClientManagerLoaded(Arc<ClientManager>),
    /// A second launch of the app was attempted; the single-instance listener
    /// forwarded a "show" signal. Surface the main window.
    InstanceWakeRequested,
}

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
