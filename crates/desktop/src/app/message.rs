use std::sync::Arc;

use crate::{
    components::sidebar::SidebarMessage,
    services::{favicon::FaviconMessage, sdk::ClientManager},
    views::{
        about::AboutMessage, export::ExportMessage, generator::GeneratorMessage,
        import::ImportMessage, login::LoginMessage, magnify::MagnifyMessage, send::SendMessage,
        settings::SettingsMessage, title_bar::TitleBarMessage, vault::VaultMessage,
    },
};

// ── Top-level Message ──────────────────────────────────────────────────────
// `View` carries sub-view messages (all need `UpdateCtx`, factored into one
// shared construction site). The rest are app-level signals.

#[derive(Debug, Clone)]
pub enum Message {
    View(ViewMessage),
    About(AboutMessage),
    Window(WindowMessage),
    System(SystemMessage),
    /// Sidebar chrome (collapse, section/filter selection). Handled at App
    /// level because it persists across authenticated screens.
    Sidebar(SidebarMessage),
    /// One per completed fetch; arrival alone triggers the redraw.
    Favicon(FaviconMessage),
    /// Per-frame tick from `iced::window::frames()` while a transition is
    /// in flight. No-op handler — the redraw is the point. Subscribed only
    /// while something is animating, so idle wake rate is zero.
    AnimationTick,
    /// Routed at the top level (not via `ViewMessage`) because the launcher
    /// owns its own window and can't share `UpdateCtx` with screen views.
    Magnify(MagnifyMessage),
}

/// Sub-view messages, bundled so `App::update` builds `UpdateCtx` in one
/// place and the `&mut App::open_overlay` borrow has a single scope.
#[derive(Debug, Clone)]
pub enum ViewMessage {
    Login(LoginMessage),
    Vault(VaultMessage),
    Send(SendMessage),
    TitleBar(TitleBarMessage),
    Settings(SettingsMessage),
    Generator(GeneratorMessage),
    Import(ImportMessage),
    Export(ExportMessage),
}

// Convenience constructors so call sites can use fn-pointer form
// (`.map(Message::login)`) without the `View(ViewMessage::Login(..))` wrap.
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

    pub fn generator(m: GeneratorMessage) -> Self {
        Self::View(ViewMessage::Generator(m))
    }

    pub fn import(m: ImportMessage) -> Self {
        Self::View(ViewMessage::Import(m))
    }

    pub fn export(m: ExportMessage) -> Self {
        Self::View(ViewMessage::Export(m))
    }
}

/// Per-window OS events. Every variant carries `window::Id` for daemon
/// multi-window dispatch.
#[derive(Debug, Clone)]
pub enum WindowMessage {
    Opened(iced::window::Id),
    GotRawId(iced::window::Id, u64),
    /// OS-emitted close (Alt+F4, native X, window-list "Close"). Funnels
    /// into the same `WindowAction::Close` path the custom title-bar X uses,
    /// so close-to-tray applies uniformly. `exit_on_close_request: false`
    /// on the main window defers the real close to our handler.
    CloseRequested(iced::window::Id),
    Closed(iced::window::Id),
    KeyPressed(iced::window::Id, iced::keyboard::Event),
    Resized(iced::window::Id, iced::Size),
    /// Used by Magnify for click-outside-to-dismiss; other windows ignore it.
    Unfocused(iced::window::Id),
}

/// Global signals that aren't tied to a specific window.
#[derive(Debug, Clone)]
pub enum SystemMessage {
    /// Muda menu item activated — native app menu and tray context menu
    /// share one global receiver; the handler resolves the id on whichever
    /// handle owns it.
    MudaEvent(muda::MenuEvent),
    /// Tray icon left-click; the pump in [`crate::services::tray::click_stream`]
    /// already filters to the toggle case.
    TrayClick(crate::services::tray::TrayAction),
    /// OS-level light/dark scheme changed. Only acted on under
    /// `ThemePreference::System`.
    ThemeChanged,
    CloseToast(usize),
    /// Background `ClientManager::load` finished. Swaps the placeholder
    /// `ClientManager::empty()` for the populated one.
    ClientManagerLoaded(Arc<ClientManager>),
    /// Single-instance listener forwarded a "show" signal from a second launch.
    InstanceWakeRequested,
    /// OS session state change (screen lock/unlock, suspend/resume). The
    /// handler locks every vault when any user has
    /// [`UserPreferences::lock_on_system_lock`][crate::services::preferences::UserPreferences::lock_on_system_lock]
    /// enabled.
    SessionEvent(session_events::SessionEvent),
    /// `iced::system::information()` resolved with the actual graphics backend
    /// wgpu picked. Used to populate `Settings::wgpu_backend_verified` so the
    /// next launch can skip multi-backend enumeration. Empty/unknown payloads
    /// are dropped by the handler.
    #[cfg(feature = "gpu")]
    WgpuBackendDiscovered(String),
}
