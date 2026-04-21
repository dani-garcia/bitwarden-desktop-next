mod handlers;
mod helpers;
mod message;

pub use message::{Message, SystemMessage, WindowMessage};
use message::{WindowInfo, WindowKind};

use std::{collections::HashMap, sync::Arc};

use iced::{Element, Subscription, Task, time};

use crate::{
    clipboard::ClipboardManager,
    components::{
        account_switcher::AccountEntry,
        toast::{self, Toast},
    },
    sdk::ClientManager,
    settings::Settings,
    state::{Screen, UnlockMethod, UserId},
    theme::{AppTheme, ThemePreference},
    tray::TrayHandle,
    views::{
        login, settings as settings_view,
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
    pub(super) settings_view: settings_view::SettingsView,
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
    /// Cached id of the main window — always present from `App::new` until
    /// `iced::exit`. Child windows (About) are not tracked here.
    pub(super) main_window: iced::window::Id,
    pub(super) native_menu: Option<crate::menu::NativeMenuHandle>,

    // ── Tray + user settings ───────────────────────────────────────────────
    // `settings` is read once at startup from `data/settings.json`. A future
    // settings view mutates fields directly; all tray / lifecycle branches
    // re-read `self.settings.<field>` at event time (never cached), so live
    // changes apply without a restart.
    pub(super) settings: Settings,
    pub(super) tray: Option<TrayHandle>,

    // ── Cross-cutting UI overlay queue ─────────────────────────────────────
    pub(super) toasts: Vec<Toast>,

    // ── Clipboard manager ──────────────────────────────────────────────────
    // Single writer: every clipboard `set` flows through here so the 30 s
    // auto-clear bookkeeping sees every write. See `clipboard.rs`.
    pub(super) clipboard: ClipboardManager,
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

pub(super) const MAIN_WINDOW_SIZE: iced::Size = iced::Size::new(1024.0, 800.0);

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let settings = Settings::load();
        let user_theme = settings.theme;

        // Apply the persisted language preference before any `fl!()` call
        // resolves user-visible strings. Empty string = "follow OS locale"
        // which the `i18n::init()` earlier in `main` already honoured.
        if !settings.language.is_empty()
            && let Ok(tag) = settings.language.parse()
        {
            crate::i18n::set_language(tag);
        }

        // Build the tray up-front if any tray-related setting is on, so
        // `start_to_tray` has something to live in and user clicks find it
        // immediately. Tray build failure → log + fall through without.
        let mut tray = None;
        if settings.wants_tray() {
            tray = crate::tray::build();
            if tray.is_none() {
                tracing::warn!("tray requested by settings but failed to initialise");
            }
        }

        // Always open the main window; when `start_to_tray` is on (and the
        // tray actually initialised), open it hidden so the user sees only
        // the tray. Iced plumbs `visible: false` through winit's
        // `with_visible(false)` — the window never flashes on screen, iced
        // still owns the id and keeps firing `view()`. Toggling later is a
        // cheap `Mode::Windowed` / `gain_focus`.
        let start_hidden = settings.start_to_tray && tray.is_some();
        let (main_id, open_task) = iced::window::open(iced::window::Settings {
            size: MAIN_WINDOW_SIZE,
            min_size: Some(iced::Size::new(800.0, 750.0)),
            decorations: crate::menu::should_use_native_title_bar(),
            visible: !start_hidden,
            // Required for tray "close to tray": without this, the OS-sent
            // `CloseRequested` event would close the window before our
            // `WindowCommand::Close` handler can choose to hide instead.
            // The About window keeps the default `true` (never hides).
            exit_on_close_request: false,
            platform_specific: main_window_platform_specific(),
            icon: iced::window::icon::from_file_data(
                crate::assets::ICON_PNG,
                Some(image::ImageFormat::Png),
            )
            .ok(),
            ..Default::default()
        });
        let mut windows = HashMap::new();
        windows.insert(main_id, WindowInfo::new(WindowKind::Main, MAIN_WINDOW_SIZE));

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
            settings_view: settings_view::SettingsView::new(),
            title_bar: title_bar::TitleBarState::new(),
            client_manager: Arc::new(ClientManager::empty()),
            cache: ViewCache::default(),
            theme: ThemeState::new(user_theme),
            windows,
            main_window: main_id,
            native_menu: None,
            settings,
            tray,
            toasts: Vec::new(),
            clipboard: ClipboardManager::new(),
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
        // app can drive a responsive layout, and `CloseRequested` so
        // close-to-tray can intercept OS-native close actions.
        let event_sub = iced::event::listen_with(|event, _status, id| match event {
            iced::Event::Keyboard(ev) => Some(Message::Window(WindowMessage::KeyPressed(id, ev))),
            iced::Event::Window(iced::window::Event::Resized(size)) => {
                Some(Message::Window(WindowMessage::Resized(id, size)))
            }
            iced::Event::Window(iced::window::Event::CloseRequested) => {
                Some(Message::Window(WindowMessage::CloseRequested(id)))
            }
            _ => None,
        });

        // muda's `MenuEvent::receiver()` is global — the native app menu and
        // the tray context menu both dispatch to it. A single pump thread
        // (bound inside the stream) forwards events here; the handler filters
        // by `MenuId` to decide whether it's an app-menu or tray-menu click.
        let muda_sub = if self.native_menu.is_some() || self.tray.is_some() {
            Subscription::run(crate::menu::muda_event_stream)
                .map(|ev| Message::System(SystemMessage::MudaEvent(ev)))
        } else {
            Subscription::none()
        };

        // Tray icon left-click events come through a separate receiver. The
        // stream filters to the click-to-toggle case before emitting, so the
        // handler just runs the action.
        let tray_sub = if self.tray.is_some() {
            Subscription::run(crate::tray::click_stream)
                .map(|action| Message::System(SystemMessage::TrayClick(action)))
        } else {
            Subscription::none()
        };

        let theme_sub = Subscription::run_with(self.theme.system.clone(), |st| st.subscribe())
            .map(|_| Message::System(SystemMessage::ThemeChanged));

        // 1 Hz tick — only active while the detail pane shows a login with
        // a TOTP secret, so the code + countdown ring refresh live.
        let totp_sub = if self.screen == Screen::Vault && self.vault_view.has_totp_selected() {
            time::every(std::time::Duration::from_secs(1))
                .map(|_| Message::Vault(vault::VaultMessage::TotpTick))
        } else {
            Subscription::none()
        };

        // Second-launch wake-up: the listener is bound once, inside this
        // stream, and kept alive across `update()` cycles because iced
        // hashes the subscription identity from the `fn` pointer.
        let wake_sub = Subscription::run(crate::instance_lock::wake_stream)
            .map(|_| Message::System(SystemMessage::InstanceWakeRequested));

        Subscription::batch([
            close_sub, event_sub, muda_sub, tray_sub, theme_sub, totp_sub, wake_sub,
        ])
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
            // Interacting with the settings modal should close any other
            // overlays (title-bar menus, account-switcher dropdown) that
            // would otherwise paint beside the modal.
            Message::Settings(_) => self.dismiss_all_overlays(),
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
            Message::Settings(m) => {
                let (task, ev) = self.settings_view.update(m);
                let task = task.map(Message::Settings);
                let ev_task = ev
                    .map(|e| self.handle_settings_event(e))
                    .unwrap_or_else(Task::none);
                Task::batch([task, ev_task])
            }
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
            Some(WindowKind::About) => crate::fl!("about-window-title"),
            _ => crate::fl!("app-title"),
        }
    }

    pub fn theme(&self, _window_id: iced::window::Id) -> AppTheme {
        self.theme.current.clone()
    }

    fn view_main(&self) -> Element<'_, Message, AppTheme> {
        let colors = &self.theme.current.colors;

        let active = self.active_account_entry();
        let email = active.map(|a| a.email.as_str());
        let server = active.map(|a| a.server_url.as_str()).unwrap_or("");

        let main_window_width = self
            .windows
            .get(&self.main_window_id())
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
                    // Invariant: `screen == Vault` implies an active user is set
                    // (we never flip to Vault without one). The unwraps here
                    // make that contract explicit to the view.
                    self.active_user
                        .as_ref()
                        .expect("Screen::Vault without active_user"),
                    email.expect("Screen::Vault without active email"),
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

        // Delete-confirmation modal — same hoisting rule as the sheet.
        let modal: Option<Element<'_, Message, AppTheme>> = if self.screen == Screen::Vault {
            self.vault_view
                .modal_view(colors)
                .map(|el| el.map(Message::Vault))
        } else {
            None
        };

        // Settings modal — composed above the vault modal so it sits on top
        // of any other overlays.
        let settings_modal: Option<Element<'_, Message, AppTheme>> = self
            .settings_view
            .modal_view(colors)
            .map(|el| el.map(Message::Settings));

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
            let with_modal: Element<'_, Message, AppTheme> = match modal {
                Some(modal) => iced::widget::stack![with_sheet, modal].into(),
                None => with_sheet,
            };
            let with_settings: Element<'_, Message, AppTheme> = match settings_modal {
                Some(m) => iced::widget::stack![with_modal, m].into(),
                None => with_modal,
            };
            let with_toasts: Element<'_, Message, AppTheme> =
                toast::Manager::new(with_settings, &self.toasts, close_toast).into();

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
            let with_modal: Element<'_, Message, AppTheme> = match modal {
                Some(modal) => iced::widget::stack![with_sheet, modal].into(),
                None => with_sheet,
            };
            let with_settings: Element<'_, Message, AppTheme> = match settings_modal {
                Some(m) => iced::widget::stack![with_modal, m].into(),
                None => with_modal,
            };
            toast::Manager::new(with_settings, &self.toasts, close_toast).into()
        }
    }
}
