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
    state::{Screen, UnlockMethod, UserId},
    theme::{AppTheme, ThemePreference},
    views::{
        login,
        title_bar::{self, TitleBarMessage},
        vault,
    },
};

use helpers::main_window_platform_specific;

pub struct App {
    // ── Domain state ───────────────────────────────────────────────────────
    pub(super) active_user: Option<UserId>,

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

    // ── Window chrome ─────────────────────────────────────────────────────
    pub(super) screen: Screen,
    pub(super) theme: ThemeState,
    pub(super) windows: HashMap<iced::window::Id, WindowInfo>,
    pub(super) native_menu: Option<crate::menu::NativeMenuHandle>,

    // ── Cross-cutting UI overlay queue ─────────────────────────────────────
    pub(super) toasts: Vec<Toast>,
}

/// Derived data cached across `view()` rebuilds. Recomputed by
/// `refresh_cache()` after every `update()`. All fields are derived
/// from `ClientManager` (the SDK) — they exist only to survive the
/// iced view borrow, not as independent state.
#[derive(Default)]
pub(super) struct ViewCache {
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

impl ThemeState {
    pub fn new(preference: ThemePreference) -> Self {
        let system = Arc::new(
            system_theme::SystemTheme::new().expect("failed to initialize system theme observer"),
        );
        let current = preference.resolve(system.get_scheme());
        Self {
            preference,
            current,
            system,
        }
    }

    pub fn refresh(&mut self) {
        if self.preference == ThemePreference::System {
            self.current = self.preference.resolve(self.system.get_scheme());
        }
    }
}

// ── Construction + iced daemon callbacks ───────────────────────────────────

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let user_theme = ThemePreference::Light;

        // Open the main window via `window::open` — daemon mode doesn't
        // create a window automatically (unlike `iced::application`).
        let main_size = iced::Size::new(1024.0, 800.0);
        let (main_id, open_task) = iced::window::open(iced::window::Settings {
            size: main_size,
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
        windows.insert(main_id, WindowInfo::new(WindowKind::Main, main_size));

        // Discover users in `<workspace-root>/data/` and open one SQLite DB
        // per user. Runs as a regular async task on iced's tokio multi-thread
        // runtime; the `ClientManagerLoaded` handler swaps the Arc when done.
        let load_task = Task::perform(async { Arc::new(ClientManager::load().await) }, |mgr| {
            Message::System(SystemMessage::ClientManagerLoaded(mgr))
        });

        let app = Self {
            active_user: None,
            screen: Screen::Loading,
            login_view: login::LoginView::new(),
            vault_view: vault::VaultView::new(),
            title_bar: title_bar::TitleBarState::new(),
            client_manager: Arc::new(ClientManager::empty()),
            cache: ViewCache::default(),
            theme: ThemeState::new(user_theme),
            windows,
            native_menu: None,
            toasts: Vec::new(),
        };

        (
            app,
            Task::batch([
                open_task.map(|id| Message::Window(WindowMessage::Opened(id))),
                load_task,
            ]),
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
        // The same listener also forwards `window::Event::Resized` so the
        // app can drive a responsive layout (vault detail pane vs sheet).
        let event_sub = iced::event::listen_with(|event, _status, id| match event {
            iced::Event::Keyboard(ev) => Some(Message::Window(WindowMessage::KeyPressed(id, ev))),
            iced::Event::Window(iced::window::Event::Resized(size)) => {
                Some(Message::Window(WindowMessage::Resized(id, size)))
            }
            _ => None,
        });

        let native_menu_sub = if self.native_menu.is_some() {
            time::every(std::time::Duration::from_millis(16))
                .map(|_| Message::System(SystemMessage::PollNativeMenu))
        } else {
            Subscription::none()
        };

        let theme_sub = Subscription::run_with(self.theme.system.clone(), |st| st.subscribe())
            .map(|_| Message::System(SystemMessage::ThemeChanged));

        Subscription::batch([close_sub, event_sub, native_menu_sub, theme_sub])
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
                let (task, ev) =
                    self.login_view
                        .update(m, &self.client_manager, self.active_user.as_ref());
                let task = task.map(Message::Login);
                let ev_task = ev
                    .map(|e| self.handle_login_event(e))
                    .unwrap_or_else(Task::none);
                Task::batch([task, ev_task])
            }
            Message::Vault(m) => {
                let (task, ev) =
                    self.vault_view
                        .update(m, &self.client_manager, self.active_user.as_ref());
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

        let active = self.active_account_entry();
        let email = active.map(|a| a.email.as_str()).unwrap_or("No account");
        let server = active.map(|a| a.server_url.as_str()).unwrap_or("");

        let main_window_width = self
            .main_window_id()
            .and_then(|id| self.windows.get(&id))
            .map(|info| info.size.width)
            .unwrap_or(1024.0);

        let page: Element<'_, Message, AppTheme> = match self.screen {
            Screen::Loading => {
                iced::widget::center(crate::components::spinner::spinner(48.0, colors.accent))
                    .into()
            }
            Screen::Login => self
                .login_view
                .view(
                    email,
                    server,
                    &self.cache.accounts,
                    &self.cache.unlock_alternatives,
                    colors,
                )
                .map(Message::Login),
            Screen::Vault => self
                .vault_view
                .view(
                    self.active_user.as_ref(),
                    email,
                    &self.cache.accounts,
                    colors,
                    main_window_width,
                )
                .map(Message::Vault),
        };

        let close_toast = |idx| Message::System(SystemMessage::CloseToast(idx));

        // Bottom-sheet overlay for the narrow vault layout. Hoisted to the
        // app level so it covers the sidebar AND the title bar.
        let sheet: Option<Element<'_, Message, AppTheme>> = if self.screen == Screen::Vault {
            self.vault_view
                .sheet_view(colors, main_window_width)
                .map(|el| el.map(Message::Vault))
        } else {
            None
        };

        if crate::menu::should_use_custom_menu_bar() {
            let menu_state = self.menu_state();
            let tb = self
                .title_bar
                .view(self.main_window_maximized(), &menu_state, colors)
                .map(Message::TitleBar);
            let main_column: Element<'_, Message, AppTheme> =
                iced::widget::column![tb, page].height(iced::Fill).into();
            let with_sheet: Element<'_, Message, AppTheme> = match sheet {
                Some(sheet) => iced::widget::stack![main_column, sheet].into(),
                None => main_column,
            };
            let with_toasts: Element<'_, Message, AppTheme> =
                toast::Manager::new(with_sheet, &self.toasts, close_toast).into();

            title_bar::resize_wrapper(with_toasts, |dir| {
                Message::TitleBar(TitleBarMessage::ResizeEdge(dir))
            })
        } else {
            let tb = title_bar::TitleBarState::view_empty().map(Message::TitleBar);
            let main_column: Element<'_, Message, AppTheme> =
                iced::widget::column![tb, page].height(iced::Fill).into();
            let with_sheet: Element<'_, Message, AppTheme> = match sheet {
                Some(sheet) => iced::widget::stack![main_column, sheet].into(),
                None => main_column,
            };
            toast::Manager::new(with_sheet, &self.toasts, close_toast).into()
        }
    }
}
