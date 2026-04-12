use std::{collections::HashMap, sync::Arc};

use iced::{Element, Subscription, Task, time};

use crate::{
    components::{
        account_switcher::AccountEntry,
        toast::{self, Toast},
    },
    sdk::ClientManager,
    state::{AppState, Screen, UnlockMethod, UserId, UserSession},
    theme::{AppTheme, ThemePreference},
    views::{
        about::AboutMessage,
        login::{self, AuthPage, LoginEvent, LoginMessage},
        title_bar::{self, TitleBarEvent, TitleBarMessage, WindowCommand},
        vault::{self, VaultEvent, VaultMessage},
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
    /// User dismissed a toast via the × button or auto-dismiss expiry.
    CloseToast(usize),
}

pub struct App {
    // ── Domain state ───────────────────────────────────────────────────────
    state: AppState,

    // ── Sub-views (each owns its own state + async lifecycle) ──────────────
    login_view: login::LoginView,
    vault_view: vault::VaultView,
    title_bar: title_bar::TitleBarState,

    // ── External dependencies ──────────────────────────────────────────────
    client_manager: Arc<ClientManager>,

    // ── Derived cache ──────────────────────────────────────────────────────
    // Exists because iced's `view()` returns an `Element<'_, ...>` that
    // borrows from `&self`; locally-computed Vecs inside `view()` would be
    // dropped before the Element. Cached fields survive the borrow. See
    // `docs/architecture.md` → "Why Things Are the Way They Are".
    cache: ViewCache,

    // ── Theme (preference + resolved instance + OS observer) ───────────────
    pub theme: ThemeState,

    // ── Per-window state (keyed by iced window::Id) ─────────────────────────
    windows: HashMap<iced::window::Id, WindowInfo>,

    // ── Native menu bridge (macOS muda on attach, polled each tick) ────────
    native_menu: Option<crate::menu::NativeMenuHandle>,

    // ── Cross-cutting UI overlay queue ─────────────────────────────────────
    toasts: Vec<Toast>,
}

/// Derived data cached across `view()` rebuilds. Recomputed by
/// `refresh_cache()` after every `update()`. Fields are reads of
/// `AppState` / `LoginView::auth_page` and don't represent independent
/// state — they exist only to survive the iced view borrow.
#[derive(Default)]
struct ViewCache {
    email: String,
    server: String,
    accounts: Vec<AccountEntry>,
    unlock_alternatives: Vec<UnlockMethod>,
}

/// Theme state bundle: user preference, resolved `AppTheme` instance,
/// and the OS observer we subscribe to for system-theme changes.
pub struct ThemeState {
    preference: ThemePreference,
    pub current: AppTheme,
    system: Arc<system_theme::SystemTheme>,
}

/// Per-window metadata. Keyed by `iced::window::Id` in a `HashMap` on `App`.
#[derive(Debug)]
struct WindowInfo {
    kind: WindowKind,
    fullscreen: bool,
    maximized: bool,
}

impl WindowInfo {
    fn new(kind: WindowKind) -> Self {
        Self {
            kind,
            fullscreen: false,
            maximized: false,
        }
    }
}

/// Discriminant for each kind of window the app can have open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowKind {
    Main,
    About,
}

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

    // ── Event handlers ──────────────────────────────────────────────────────
    //
    // One `handle_*_event` per sub-view. Each handler receives a declarative
    // event from its view's `update()` and translates it into App-level side
    // effects (state mutation, screen switches, follow-on async tasks).

    fn handle_login_event(&mut self, event: LoginEvent) -> Task<Message> {
        match event {
            LoginEvent::Unlocked { uid } | LoginEvent::LoggedIn { uid } => {
                if let Some(session) = self.state.users.get_mut(&uid) {
                    session.locked = false;
                }
                if self.state.active_user.as_deref() != Some(&uid) {
                    tracing::debug!(
                        %uid,
                        "unlock event dropped: active user changed while in flight"
                    );
                    return Task::none();
                }
                self.state.screen = Screen::Vault;
                tracing::info!(%uid, "unlock succeeded; loading vault list");
                self.load_vault_list_task(uid)
            }
            LoginEvent::SignOutRequested => {
                if let Some(ref uid) = self.state.active_user {
                    self.state.users.remove(uid);
                }
                self.state.active_user = self.state.users.keys().next().cloned();
                Task::none()
            }
            LoginEvent::UserSelected { uid } => self.handle_user_switch(uid),
            LoginEvent::ToastRequested(t) => {
                self.push_toast(t);
                Task::none()
            }
        }
    }

    fn handle_vault_event(&mut self, event: VaultEvent) -> Task<Message> {
        match event {
            VaultEvent::UserSelected { uid } => self.handle_user_switch(uid),
            VaultEvent::AddAccountRequested => {
                self.state.screen = Screen::Login;
                self.login_view.reset_to_email_entry();
                Task::none()
            }
            VaultEvent::SearchFocusRequested => {
                // Focus ops don't emit messages so they can't live inside
                // the view as a `Task<VaultMessage>`. The widget ID constant
                // lives here with the one call site that produces it.
                iced::widget::operation::focus(iced::widget::Id::new("vault-search"))
            }
            VaultEvent::ToastRequested(t) => {
                self.push_toast(t);
                Task::none()
            }
        }
    }

    fn handle_titlebar_event(&mut self, event: TitleBarEvent) -> Task<Message> {
        match event {
            TitleBarEvent::MenuInvoked(menu_action) => self.handle_menu_action(menu_action),
            TitleBarEvent::Window(cmd) => self.handle_window_command(cmd),
        }
    }

    fn handle_about_message(&mut self, msg: AboutMessage) -> Task<Message> {
        match msg {
            AboutMessage::CopyInfo => iced::clipboard::write(crate::views::about::info_string()),
            AboutMessage::Close => {
                if let Some(id) = self.about_window_id() {
                    iced::window::close(id)
                } else {
                    Task::none()
                }
            }
        }
    }

    fn handle_window_command(&mut self, cmd: WindowCommand) -> Task<Message> {
        let Some(id) = self.main_window_id() else {
            return Task::none();
        };
        match cmd {
            WindowCommand::Minimize => iced::window::minimize(id, true),
            WindowCommand::Maximize => {
                if let Some(info) = self.windows.get_mut(&id) {
                    info.maximized = !info.maximized;
                }
                iced::window::toggle_maximize(id)
            }
            WindowCommand::Close => iced::window::close(id),
            WindowCommand::Drag => iced::window::drag(id),
            WindowCommand::ResizeEdge(dir) => iced::window::drag_resize(id, dir),
        }
    }

    fn handle_window_message(&mut self, msg: WindowMessage) -> Task<Message> {
        match msg {
            WindowMessage::Opened(id) => {
                // Native menu only attaches to the main window.
                if Some(id) == self.main_window_id() {
                    iced::window::raw_id::<Message>(id)
                        .map(move |raw| Message::Window(WindowMessage::GotRawId(id, raw)))
                } else {
                    Task::none()
                }
            }
            WindowMessage::GotRawId(_id, raw_id) => {
                self.native_menu = crate::menu::attach_menu(raw_id);
                Task::none()
            }
            WindowMessage::Closed(id) => {
                let was_main = Some(id) == self.main_window_id();
                self.windows.remove(&id);
                if was_main { iced::exit() } else { Task::none() }
            }
            WindowMessage::KeyPressed(id, ev) => {
                // Keyboard shortcuts only affect the main window — Ctrl+F
                // in the About window must not trigger vault search.
                if Some(id) != self.main_window_id() {
                    return Task::none();
                }
                let iced::keyboard::Event::KeyPressed { key, modifiers, .. } = ev else {
                    return Task::none();
                };
                let state = self.menu_state();
                let Some(action) = crate::menu::find_shortcut_action(&key, modifiers, &state)
                else {
                    return Task::none();
                };
                self.title_bar.dismiss_menu();
                self.handle_menu_action(action)
            }
        }
    }

    fn handle_system_message(&mut self, msg: SystemMessage) -> Task<Message> {
        match msg {
            SystemMessage::PollNativeMenu => {
                if let Some(ref handle) = self.native_menu
                    && let Some(action) = crate::menu::poll_native_event(handle)
                {
                    return self.handle_menu_action(action);
                }
                Task::none()
            }
            SystemMessage::ThemeChanged => {
                if self.theme.preference == ThemePreference::System {
                    self.theme.current = self
                        .theme
                        .preference
                        .resolve(self.theme.system.get_scheme());
                }
                Task::none()
            }
            SystemMessage::CloseToast(idx) => {
                if idx < self.toasts.len() {
                    self.toasts.remove(idx);
                }
                Task::none()
            }
        }
    }

    fn handle_menu_action(&mut self, action: crate::menu::MenuAction) -> Task<Message> {
        use crate::menu::MenuAction;
        match action {
            MenuAction::Quit => {
                return iced::exit();
            }
            MenuAction::LockAllVaults => {
                for session in self.state.users.values_mut() {
                    session.locked = true;
                }
                self.state.screen = Screen::Login;
                let preferred = self
                    .active_session()
                    .map(|s| s.unlock_methods.preferred())
                    .unwrap_or(UnlockMethod::MasterPassword);
                self.login_view.auth_page = AuthPage::Unlock(preferred);
                self.login_view.password_input.clear();
                self.login_view.pin_input.clear();
                self.login_view.show_password = false;
                self.refresh_cache();
            }
            MenuAction::ToggleFullScreen => {
                if let Some(id) = self.main_window_id()
                    && let Some(info) = self.windows.get_mut(&id)
                {
                    info.fullscreen = !info.fullscreen;
                    let mode = if info.fullscreen {
                        iced::window::Mode::Fullscreen
                    } else {
                        iced::window::Mode::Windowed
                    };
                    return iced::window::set_mode(id, mode);
                }
            }
            MenuAction::Minimize => {
                if let Some(id) = self.main_window_id() {
                    return iced::window::minimize(id, true);
                }
            }
            MenuAction::Close => {
                if let Some(id) = self.main_window_id() {
                    return iced::window::close(id);
                }
            }
            MenuAction::SearchVault => {
                if self.state.screen == Screen::Vault {
                    self.vault_view.search_query.clear();
                    self.refresh_cache();
                    return iced::widget::operation::focus(iced::widget::Id::new("vault-search"));
                }
            }
            MenuAction::SyncNow | MenuAction::Reload => {
                self.refresh_cache();
            }
            MenuAction::HideToTray | MenuAction::ToggleAlwaysOnTop => {}
            MenuAction::About => {
                // Re-focus existing About window if already open.
                if let Some(id) = self.about_window_id() {
                    return iced::window::gain_focus(id);
                }

                let (about_id, open_task) = iced::window::open(iced::window::Settings {
                    size: iced::Size::new(400.0, 280.0),
                    min_size: Some(iced::Size::new(360.0, 260.0)),
                    position: iced::window::Position::Centered,
                    resizable: false,
                    decorations: true,
                    icon: iced::window::icon::from_file_data(
                        crate::assets::ICON_PNG,
                        Some(image::ImageFormat::Png),
                    )
                    .ok(),
                    ..Default::default()
                });

                self.windows
                    .insert(about_id, WindowInfo::new(WindowKind::About));

                return open_task.map(|id| Message::Window(WindowMessage::Opened(id)));
            }
        }
        Task::none()
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn main_window_id(&self) -> Option<iced::window::Id> {
        self.windows
            .iter()
            .find(|(_, w)| w.kind == WindowKind::Main)
            .map(|(id, _)| *id)
    }

    fn about_window_id(&self) -> Option<iced::window::Id> {
        self.windows
            .iter()
            .find(|(_, w)| w.kind == WindowKind::About)
            .map(|(id, _)| *id)
    }

    fn main_window_maximized(&self) -> bool {
        self.main_window_id()
            .and_then(|id| self.windows.get(&id))
            .is_some_and(|w| w.maximized)
    }

    fn post_update(&mut self) {
        self.refresh_cache();
    }

    fn push_toast(&mut self, toast: Toast) {
        self.toasts.push(toast);
    }

    fn active_session(&self) -> Option<&UserSession> {
        self.state
            .active_user
            .as_ref()
            .and_then(|uid| self.state.users.get(uid))
    }

    fn refresh_cache(&mut self) {
        let active_session = self.active_session().cloned();

        self.cache.email = active_session
            .as_ref()
            .map(|s| s.email.clone())
            .unwrap_or_else(|| "No account".into());

        self.cache.server = active_session
            .as_ref()
            .map(|s| s.server_url.clone())
            .unwrap_or_default();

        self.cache.accounts = self
            .state
            .users
            .iter()
            .map(|(uid, session)| AccountEntry {
                user_id: uid.clone(),
                email: session.email.clone(),
                server_url: session.server_url.clone(),
                locked: session.locked,
            })
            .collect();

        // Compute unlock alternatives for the current auth page
        self.cache.unlock_alternatives = if let AuthPage::Unlock(method) = self.login_view.auth_page
        {
            active_session
                .as_ref()
                .map(|s| s.unlock_methods.alternatives(method))
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        if let Some(ref handle) = self.native_menu {
            crate::menu::sync_native_enabled(handle, &self.menu_state());
        }
    }

    /// Switch the active user. Clears the previous user's vault state and,
    /// if the new user is unlocked, returns the task that repopulates the
    /// vault list.
    fn handle_user_switch(&mut self, uid: UserId) -> Task<Message> {
        self.state.active_user = Some(uid.clone());
        let session = self.state.users.get(&uid);
        let locked = session.map(|s| s.locked).unwrap_or(true);
        self.vault_view.reset();
        if locked {
            self.state.screen = Screen::Login;
            let preferred = session
                .map(|s| s.unlock_methods.preferred())
                .unwrap_or(UnlockMethod::MasterPassword);
            self.login_view.auth_page = AuthPage::Unlock(preferred);
            self.login_view.password_input.clear();
            self.login_view.pin_input.clear();
            self.login_view.show_password = false;
            Task::none()
        } else {
            self.state.screen = Screen::Vault;
            self.load_vault_list_task(uid)
        }
    }

    /// Build the task that decrypts the user's vault list and lands as
    /// `VaultMessage::ListLoaded`. Returns `Task<Message>` (already lifted
    /// via `.map(Message::Vault)`) so all callers can plumb the result
    /// without caring about the inner message type.
    fn load_vault_list_task(&self, uid: UserId) -> Task<Message> {
        let mgr = self.client_manager.clone();
        let uid_for_msg = uid.clone();
        Task::perform(
            async move {
                mgr.list_ciphers(&uid)
                    .await
                    .map(|items| items.into_iter().map(Arc::new).collect::<Vec<_>>())
            },
            move |result| VaultMessage::ListLoaded(uid_for_msg.clone(), result),
        )
        .map(Message::Vault)
    }

    fn menu_state(&self) -> crate::menu::MenuState {
        let has_accounts = !self.state.users.is_empty();
        let is_locked = self
            .state
            .active_user
            .as_ref()
            .and_then(|uid| self.state.users.get(uid))
            .map(|s| s.locked)
            .unwrap_or(true);
        let has_lockable = self.state.users.values().any(|s| !s.locked);

        crate::menu::MenuState {
            is_locked,
            has_accounts,
            has_lockable_accounts: has_lockable,
            has_authenticated_accounts: has_accounts,
        }
    }
}

// ── Platform-specific window settings ─────────────────────────────────────
//
// Moved from main.rs on the daemon migration: `iced::daemon` doesn't take
// a `.window(Settings)` — boot creates the window via `window::open`.

fn main_window_platform_specific() -> iced::window::settings::PlatformSpecific {
    #[cfg(target_os = "windows")]
    {
        use iced::window::settings::PlatformSpecific;
        PlatformSpecific {
            undecorated_shadow: true,
            corner_preference: iced::window::settings::platform::CornerPreference::Round,
            ..Default::default()
        }
    }

    #[cfg(target_os = "macos")]
    {
        use iced::window::settings::PlatformSpecific;
        PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        iced::window::settings::PlatformSpecific::default()
    }
}
