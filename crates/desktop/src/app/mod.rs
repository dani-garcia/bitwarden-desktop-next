//! Top-level application state + iced daemon integration.
//!
//! ## Layout
//!
//! - This file: [`App`] / [`Views`] / [`ThemeState`] / [`ViewCache`] +
//!   their constructors. Public re-exports.
//! - [`lifecycle`] — [`App::new`] (construction) + [`App::subscription`]
//!   (event sources).
//! - [`update`] — [`App::update`] dispatch.
//! - [`view`] — [`App::view`] / [`App::title`] / [`App::theme`] /
//!   [`App::scale_factor`] (iced daemon callbacks).

mod ctx;
mod handlers;
mod helpers;
mod lifecycle;
mod message;
mod update;
mod view;
mod window;

pub use ctx::{Outcome, Overlay, RenderCtx, UpdateCtx, ViewTypes};
pub use message::{Message, SystemMessage, ViewMessage, WindowMessage};

use std::{collections::HashMap, rc::Rc};

use crate::{
    components::{sidebar::SidebarState, toast::Toast},
    domain::{Screen, UserId},
    services::{
        clipboard::ClipboardManager, sdk::AccountEntry, sdk::ClientManager, settings::Settings,
        tray::TrayHandle,
    },
    theme::{AppTheme, ThemePreference},
    views::{
        export as export_view, generator as generator_view, import as import_view, login, magnify,
        new_folder as new_folder_view, send, settings as settings_view, title_bar, vault,
    },
};

use window::WindowInfo;

pub(super) const MAIN_WINDOW_SIZE: iced::Size = iced::Size::new(1024.0, 800.0);

pub struct App {
    // ── Session ───────────────────────────────────────────────────────────
    pub(super) active_user: Option<UserId>,
    pub(super) client_manager: ClientManager,
    /// Read once at startup. Lifecycle branches re-read `self.settings.<field>`
    /// at event time (never cached) so live changes apply without a restart.
    pub(super) settings: Settings,

    // ── Navigation ────────────────────────────────────────────────────────
    pub(super) screen: Screen,
    /// Persists across screen transitions so collapse / filter selection
    /// isn't reset when navigating between Vault and Send.
    pub(super) sidebar: SidebarState,

    // ── Sub-views ─────────────────────────────────────────────────────────
    pub(super) views: Views,

    // ── Windows ───────────────────────────────────────────────────────────
    pub(super) windows: HashMap<iced::window::Id, WindowInfo>,
    /// Always present from `App::new` until `iced::exit`. Child windows
    /// (About) are not tracked here.
    pub(super) main_window: iced::window::Id,
    pub(super) magnify: magnify::MagnifyView,

    // ── Native chrome ─────────────────────────────────────────────────────
    pub(super) theme: ThemeState,
    pub(super) native_menu: Option<crate::services::menu::NativeMenuHandle>,
    pub(super) tray: Option<TrayHandle>,

    // ── Cross-cutting services ────────────────────────────────────────────
    /// Single writer: every clipboard `set` flows through here so the 30 s
    /// auto-clear bookkeeping sees every write.
    pub(super) clipboard: ClipboardManager,
    pub(super) favicon: crate::services::favicon::FaviconService,

    // ── Transient UI ──────────────────────────────────────────────────────
    /// Source of truth for which dropdown/menu is open. Writing auto-
    /// dismisses any other overlay by construction.
    pub(super) open_overlay: Option<Overlay>,
    pub(super) toasts: Vec<Toast>,
    /// Account → Fingerprint phrase modal. Closed unless the user explicitly
    /// opened it via the menu.
    pub(super) fingerprint: crate::views::fingerprint_phrase::FingerprintModal,

    // ── Derived ───────────────────────────────────────────────────────────
    pub(super) cache: ViewCache,
}

/// Derived data cached across `view()` rebuilds. Refreshed only by handlers
/// that mutate the inputs (login, logout, lock, unlock, user switch, manager
/// load). Cached on App because `view()` returns an `Element<'_, ...>`
/// borrowing `&self` — slices handed to children can't be built inline.
#[derive(Default)]
pub(super) struct ViewCache {
    pub accounts: Vec<AccountEntry>,
}

/// Sub-views grouped into their own struct so `App::update` can split its
/// borrow: the view-dispatch arm takes `&mut self.views` plus `&mut
/// self.open_overlay` via `UpdateCtx`, while event handlers reborrow other
/// App fields (client_manager, settings, etc.) without overlap.
pub struct Views {
    pub(super) login: login::LoginView,
    pub(super) vault: vault::VaultView,
    pub(super) send: send::SendView,
    pub(super) settings: settings_view::SettingsView,
    pub(super) generator: generator_view::GeneratorView,
    pub(super) import: import_view::ImportView,
    pub(super) export: export_view::ExportView,
    pub(super) new_folder: new_folder_view::NewFolderView,
    pub(super) title_bar: title_bar::TitleBarView,
}

impl Views {
    pub fn new() -> Self {
        Self {
            login: login::LoginView::new(),
            vault: vault::VaultView::new(),
            send: send::SendView::new(),
            settings: settings_view::SettingsView::new(),
            generator: generator_view::GeneratorView::new(),
            import: import_view::ImportView::new(),
            export: export_view::ExportView::new(),
            new_folder: new_folder_view::NewFolderView::new(),
            title_bar: title_bar::TitleBarView::new(),
        }
    }
}

/// Theme state bundle: user preference, resolved `AppTheme` instance,
/// and the OS observer we subscribe to for system-theme changes. The
/// observer is `None` on platforms where construction failed (headless
/// CI, sandboxed installs); the resolver falls back to a static theme
/// when the OS scheme can't be queried.
pub struct ThemeState {
    pub(super) preference: ThemePreference,
    pub current: AppTheme,
    pub(super) system: Option<Rc<system_theme::SystemTheme>>,
}

impl ThemeState {
    pub fn new(preference: ThemePreference) -> Self {
        let system = match system_theme::SystemTheme::new() {
            Ok(s) => Some(Rc::new(s)),
            Err(err) => {
                tracing::warn!(%err, "failed to init system theme observer; using static fallback");
                None
            }
        };
        let current = preference.resolve(system.as_deref());
        Self {
            preference,
            current,
            system,
        }
    }

    pub fn refresh(&mut self) {
        if self.preference == ThemePreference::System {
            self.current = self.preference.resolve(self.system.as_deref());
        }
    }
}
