use std::sync::Arc;

use crate::{
    components::sidebar::SidebarMessage,
    services::{favicon::FaviconMessage, sdk::ClientManager},
    views::{
        about::AboutMessage, login::LoginMessage, send::SendMessage, settings::SettingsMessage,
        title_bar::TitleBarMessage, vault::VaultMessage,
    },
};

// ── Top-level Message ──────────────────────────────────────────────────────
//
// Split into two groups. The `View` variant carries messages for the
// compositional sub-views — all of them need an `UpdateCtx`, and the router
// factors out one shared construction site. The remaining variants are
// app-level: `About` handles the About child window (stateless), `Window`
// carries per-window OS events (every variant takes a `window::Id`),
// `System` carries global signals, and `Favicon` is a fire-and-forget redraw
// trigger.

#[derive(Debug, Clone)]
pub enum Message {
    View(ViewMessage),
    About(AboutMessage),
    Window(WindowMessage),
    System(SystemMessage),
    /// Sidebar chrome — collapse/expand, section/filter selection. Handled
    /// at the App level because the sidebar persists across authenticated
    /// screens.
    Sidebar(SidebarMessage),
    /// Favicon service progress — one per completed fetch. Arrival alone
    /// triggers the redraw; the handler only logs.
    Favicon(FaviconMessage),
}

/// Messages that dispatch into a compositional sub-view's `update()`. All
/// need a shared `UpdateCtx` at routing time; bundling them under one
/// variant means `App::update` builds `UpdateCtx` in a single place and the
/// borrow of `App::open_overlay` has exactly one scope.
#[derive(Debug, Clone)]
pub enum ViewMessage {
    Login(LoginMessage),
    Vault(VaultMessage),
    Send(SendMessage),
    TitleBar(TitleBarMessage),
    Settings(SettingsMessage),
}

// Convenience constructors so call sites can keep using `fn`-pointer form
// (`.map(Message::login)`, `.dispatch(Message::vault, ...)`) without the
// `Message::View(ViewMessage::Login(..))` double-wrap at every use.
impl Message {
    pub fn login(m: LoginMessage) -> Self {
        Self::View(ViewMessage::Login(m))
    }

    pub fn vault(m: VaultMessage) -> Self {
        Self::View(ViewMessage::Vault(m))
    }

    pub fn send(m: SendMessage) -> Self {
        Self::View(ViewMessage::Send(m))
    }

    pub fn title_bar(m: TitleBarMessage) -> Self {
        Self::View(ViewMessage::TitleBar(m))
    }

    pub fn settings(m: SettingsMessage) -> Self {
        Self::View(ViewMessage::Settings(m))
    }
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
    /// [`crate::services::tray::click_stream`] which already filters to the
    /// click-to-toggle case.
    TrayClick(crate::services::tray::TrayAction),
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
