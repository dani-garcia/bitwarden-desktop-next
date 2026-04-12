mod handlers;
mod helpers;
mod message;

pub use message::{Message, SystemMessage, WindowMessage};
use message::{WindowInfo, WindowKind};

use std::{collections::HashMap, sync::Arc};

use iced::{Element, Subscription, Task, time};

use crate::{
    components::{
        account_switcher::AccountEntry,
        toast::{self, Toast},
    },
    sdk::ClientManager,
    state::{AppState, Screen, UnlockMethod, UserSession},
    theme::{AppTheme, ThemePreference},
    views::{
        login::{self, AuthPage},
        title_bar::{self, TitleBarMessage},
        vault,
    },
};

use helpers::main_window_platform_specific;

pub struct App {
    // ── Domain state ───────────────────────────────────────────────────────
    pub(super) state: AppState,

    // ── Sub-views (each owns its own state + async lifecycle) ──────────────
    pub(super) login_view: login::LoginView,
    pub(super) vault_view: vault::VaultView,
    pub(super) title_bar: title_bar::TitleBarState,

    // ── External dependencies ──────────────────────────────────────────────
    pub(super) client_manager: Arc<ClientManager>,

    // ── Derived cache ──────────────────────────────────────────────────────
    // Exists because iced's `view()` returns an `Element<'_, ...>` that
    // borrows from `&self`; locally-computed Vecs inside `view()` would be
    // dropped before the Element. Cached fields survive the borrow. See
    // `docs/architecture.md` → "Why Things Are the Way They Are".
    pub(super) cache: ViewCache,

    // ── Theme (preference + resolved instance + OS observer) ───────────────
    pub(super) theme: ThemeState,

    // ── Per-window state (keyed by iced window::Id) ─────────────────────────
    pub(super) windows: HashMap<iced::window::Id, WindowInfo>,

    // ── Native menu bridge (macOS muda on attach, polled each tick) ────────
    pub(super) native_menu: Option<crate::menu::NativeMenuHandle>,

    // ── Cross-cutting UI overlay queue ─────────────────────────────────────
    pub(super) toasts: Vec<Toast>,
}

/// Derived data cached across `view()` rebuilds. Recomputed by
/// `refresh_cache()` after every `update()`. Fields are reads of
/// `AppState` / `LoginView::auth_page` and don't represent independent
/// state — they exist only to survive the iced view borrow.
#[derive(Default)]
pub(super) struct ViewCache {
    pub email: String,
    pub server: String,
    pub accounts: Vec<AccountEntry>,
    pub unlock_alternatives: Vec<UnlockMethod>,
}

/// Theme state bundle: user preference, resolved `AppTheme` instance,
/// and the OS observer we subscribe to for system-theme changes.
pub struct ThemeState {
    pub(super) preference: ThemePreference,
    pub current: AppTheme,
    pub(super) system: Arc<system_theme::SystemTheme>,
}

// ── Construction + iced daemon callbacks ───────────────────────────────────

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let client_manager = Arc::new(ClientManager::load());

        // Build the per-user `UserSession` map from `ClientManager` metadata. Vault items
        // are loaded lazily after unlock via `client_manager.list_ciphers(uid)`.
        let mut users = std::collections::HashMap::new();
        for (uid, meta) in client_manager.users() {
            users.insert(
                uid.clone(),
                UserSession {
                    email: meta.email.clone(),
                    display_name: meta.display_name.clone(),
                    server_url: meta.server_url.clone(),
                    locked: true,
                    unlock_methods: meta.unlock_methods.clone(),
                },
            );
        }
        let active_user = users.keys().next().cloned();

        let system_theme = Arc::new(
            system_theme::SystemTheme::new().expect("failed to initialize system theme observer"),
        );
        let initial_theme = ThemePreference::System.resolve(system_theme.get_scheme());

        // Open the main window via `window::open` — daemon mode doesn't
        // create a window automatically (unlike `iced::application`).
        let (main_id, open_task) = iced::window::open(iced::window::Settings {
            size: iced::Size::new(1024.0, 800.0),
            min_size: Some(iced::Size::new(800.0, 750.0)),
            decorations: crate::menu::should_use_native_title_bar(),
            platform_specific: main_window_platform_specific(),
            icon: iced::window::icon::from_file_data(
                crate::assets::ICON_PNG,
                Some(image::ImageFormat::Png),
            )
            .ok(),
            ..Default::default()
        });

        let mut windows = HashMap::new();
        windows.insert(main_id, WindowInfo::new(WindowKind::Main));

        let mut app = Self {
            state: AppState {
                users,
                active_user,
                screen: Screen::Login,
            },
            login_view: login::LoginView::new(),
            vault_view: vault::VaultView::new(),
            title_bar: title_bar::TitleBarState::new(),
            client_manager,
            cache: ViewCache::default(),
            theme: ThemeState {
                preference: ThemePreference::System,
                current: initial_theme,
                system: system_theme,
            },
            windows,
            native_menu: None,
            toasts: Vec::new(),
        };
        // Set initial unlock method based on active user's preferred method
        let preferred = app
            .active_session()
            .map(|s| s.unlock_methods.preferred())
            .unwrap_or(UnlockMethod::MasterPassword);
        app.login_view.auth_page = AuthPage::Unlock(preferred);
        app.refresh_cache();

        (
            app,
            open_task.map(|id| Message::Window(WindowMessage::Opened(id))),
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // Window close events — needed for daemon mode lifecycle. When the
        // main window closes we call `iced::exit()` to terminate the daemon;
        // closing a child window (About) just removes it from the map.
        let close_sub =
            iced::window::close_events().map(|id| Message::Window(WindowMessage::Closed(id)));

        // Keyboard events are routed through `event::listen_with` (not
        // `keyboard::listen`) so the `window::Id` is provided natively by
        // the listener. `Subscription::map` requires a non-capturing
        // closure (iced enforces this at const-eval time), and we need
        // the id bound into the emitted message for daemon-readiness.
        let keyboard_sub = iced::event::listen_with(|event, _status, id| {
            if let iced::Event::Keyboard(ev) = event {
                Some(Message::Window(WindowMessage::KeyPressed(id, ev)))
            } else {
                None
            }
        });

        let native_menu_sub = if self.native_menu.is_some() {
            time::every(std::time::Duration::from_millis(16))
                .map(|_| Message::System(SystemMessage::PollNativeMenu))
        } else {
            Subscription::none()
        };

        let theme_sub = Subscription::run_with(self.theme.system.clone(), |st| st.subscribe())
            .map(|_| Message::System(SystemMessage::ThemeChanged));

        Subscription::batch([close_sub, keyboard_sub, native_menu_sub, theme_sub])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        // LOAD-BEARING: cross-view overlay dismissal. Any message directed
        // at a view (or the title bar) should close the *other* view's
        // overlays so an accidental menu click from the vault doesn't leave
        // the account-switcher dropdown open, and vice versa. Keep this
        // policy at the router level so sub-views don't need to know about
        // each other.
        match &message {
            Message::Login(_) | Message::Vault(_) => self.title_bar.dismiss_menu(),
            Message::TitleBar(_) => {
                self.login_view.dismiss_dropdowns();
                self.vault_view.dismiss_dropdowns();
            }
            Message::About(_) | Message::Window(_) | Message::System(_) => {}
        }

        let task = match message {
            Message::Login(m) => {
                let (task, ev) = self.login_view.update(
                    m,
                    &self.client_manager,
                    self.state.active_user.as_ref(),
                );
                let task = task.map(Message::Login);
                let ev_task = ev
                    .map(|e| self.handle_login_event(e))
                    .unwrap_or_else(Task::none);
                Task::batch([task, ev_task])
            }
            Message::Vault(m) => {
                let (task, ev) = self.vault_view.update(
                    m,
                    &self.client_manager,
                    self.state.active_user.as_ref(),
                );
                let task = task.map(Message::Vault);
                let ev_task = ev
                    .map(|e| self.handle_vault_event(e))
                    .unwrap_or_else(Task::none);
                Task::batch([task, ev_task])
            }
            Message::TitleBar(m) => {
                let (task, ev) = self.title_bar.update(m);
                let task = task.map(Message::TitleBar);
                let ev_task = ev
                    .map(|e| self.handle_titlebar_event(e))
                    .unwrap_or_else(Task::none);
                Task::batch([task, ev_task])
            }
            Message::About(m) => self.handle_about_message(m),
            Message::Window(m) => self.handle_window_message(m),
            Message::System(m) => self.handle_system_message(m),
        };

        self.post_update();
        task
    }

    pub fn view(&self, window_id: iced::window::Id) -> Element<'_, Message, AppTheme> {
        match self.windows.get(&window_id).map(|w| w.kind) {
            Some(WindowKind::Main) => self.view_main(),
            Some(WindowKind::About) => {
                crate::views::about::view(&self.theme.current.colors).map(Message::About)
            }
            // Defensive: all windows are eagerly inserted at creation time,
            // but if iced calls view for an id we somehow don't know about,
            // an empty space is a safe no-op.
            None => iced::widget::Space::new().into(),
        }
    }

    pub fn title(&self, window_id: iced::window::Id) -> String {
        match self.windows.get(&window_id).map(|w| w.kind) {
            Some(WindowKind::About) => "About Bitwarden".to_string(),
            _ => "Bitwarden [Next]".to_string(),
        }
    }

    pub fn theme(&self, _window_id: iced::window::Id) -> AppTheme {
        self.theme.current.clone()
    }

    fn view_main(&self) -> Element<'_, Message, AppTheme> {
        let colors = &self.theme.current.colors;

        let page: Element<'_, Message, AppTheme> = match self.state.screen {
            Screen::Login => self
                .login_view
                .view(
                    &self.cache.email,
                    &self.cache.server,
                    &self.cache.accounts,
                    &self.cache.unlock_alternatives,
                    colors,
                )
                .map(Message::Login),
            Screen::Vault => self
                .vault_view
                .view(&self.cache.email, &self.cache.accounts, colors)
                .map(Message::Vault),
        };

        let close_toast = |idx| Message::System(SystemMessage::CloseToast(idx));

        if crate::menu::should_use_custom_menu_bar() {
            let menu_state = self.menu_state();
            let tb = self
                .title_bar
                .view(self.main_window_maximized(), &menu_state, colors)
                .map(Message::TitleBar);
            let content: Element<'_, Message, AppTheme> =
                iced::widget::column![tb, page].height(iced::Fill).into();
            let with_toasts: Element<'_, Message, AppTheme> =
                toast::Manager::new(content, &self.toasts, close_toast).into();

            title_bar::resize_wrapper(with_toasts, |dir| {
                Message::TitleBar(TitleBarMessage::ResizeEdge(dir))
            })
        } else {
            let tb = title_bar::TitleBarState::view_empty().map(Message::TitleBar);
            let content: Element<'_, Message, AppTheme> =
                iced::widget::column![tb, page].height(iced::Fill).into();
            toast::Manager::new(content, &self.toasts, close_toast).into()
        }
    }
}
