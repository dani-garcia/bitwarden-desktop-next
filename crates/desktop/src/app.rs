use std::sync::Arc;

use iced::{Element, Subscription, time};

use crate::{
    components::account_switcher::AccountEntry,
    mock,
    state::{AppState, CipherItem, Screen, UnlockMethod, UserSession},
    theme::{AppTheme, ThemePreference},
    views::{
        login::{self, AuthPage, LoginMessage},
        title_bar::{self, TitleBarAction},
        vault::{self, VaultMessage},
    },
};

#[derive(Debug, Clone)]
pub enum Message {
    Login(LoginMessage),
    Vault(VaultMessage),
    TitleBar(title_bar::TitleBarMessage),
    PollNativeMenu,
    KeyPressed(iced::keyboard::Event),
    WindowOpened(iced::window::Id),
    GotRawId(u64),
    SystemThemeChanged,
}

pub struct App {
    state: AppState,
    pub theme_preference: ThemePreference,
    pub current_theme: AppTheme,
    login_view: login::LoginView,
    vault_view: vault::VaultView,
    title_bar: title_bar::TitleBarState,
    fullscreen: bool,
    maximized: bool,
    cached_email: String,
    cached_server: String,
    cached_accounts: Vec<AccountEntry>,
    cached_unlock_alternatives: Vec<UnlockMethod>,
    system_theme: Arc<system_theme::SystemTheme>,
    native_menu: Option<crate::menu::NativeMenuHandle>,
    menu_attached: bool,
    window_id: Option<iced::window::Id>,
}

impl App {
    pub fn new() -> (Self, iced::Task<Message>) {
        let (users, active_user) = mock::mock_users();

        let system_theme = Arc::new(
            system_theme::SystemTheme::new()
                .expect("failed to initialize system theme observer"),
        );
        let initial_theme = ThemePreference::System.resolve(system_theme.get_scheme());

        let mut app = Self {
            state: AppState {
                users,
                active_user: Some(active_user),
                screen: Screen::Login,
            },
            theme_preference: ThemePreference::System,
            current_theme: initial_theme,
            login_view: login::LoginView::new(),
            vault_view: vault::VaultView::new(),
            title_bar: title_bar::TitleBarState::new(),
            fullscreen: false,
            maximized: false,
            cached_email: String::new(),
            cached_server: String::new(),
            cached_accounts: Vec::new(),
            cached_unlock_alternatives: Vec::new(),
            system_theme,
            native_menu: None,
            menu_attached: false,
            window_id: None,
        };
        // Set initial unlock method based on active user's preferred method
        let preferred = app
            .active_session()
            .map(|s| s.unlock_methods.preferred())
            .unwrap_or(UnlockMethod::MasterPassword);
        app.login_view.auth_page = AuthPage::Unlock(preferred);
        app.refresh_cache();

        (app, iced::Task::none())
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let window_sub = if !self.menu_attached {
            iced::event::listen_with(|event, _status, id| {
                if let iced::Event::Window(iced::window::Event::Opened { .. }) = event {
                    Some(Message::WindowOpened(id))
                } else {
                    None
                }
            })
        } else {
            Subscription::none()
        };

        let keyboard_sub = iced::keyboard::listen().map(Message::KeyPressed);

        let native_menu_sub = if self.native_menu.is_some() {
            time::every(std::time::Duration::from_millis(16)).map(|_| Message::PollNativeMenu)
        } else {
            Subscription::none()
        };

        let theme_sub = Subscription::run_with(
            self.system_theme.clone(),
            |st| st.subscribe(),
        )
        .map(|_| Message::SystemThemeChanged);

        Subscription::batch([window_sub, keyboard_sub, native_menu_sub, theme_sub])
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        let mut extra_task = iced::Task::none();
        match message {
            Message::Login(msg) => {
                self.title_bar.dismiss_menu();
                for action in self.login_view.update(msg) {
                    match action {
                        login::LoginAction::Unlock
                        | login::LoginAction::UnlockWithPin
                        | login::LoginAction::UnlockWithBiometrics => {
                            if let Some(ref uid) = self.state.active_user
                                && let Some(session) = self.state.users.get_mut(uid)
                            {
                                session.locked = false;
                            }
                            self.state.screen = Screen::Vault;
                        }
                        login::LoginAction::LogOut => {
                            if let Some(ref uid) = self.state.active_user {
                                self.state.users.remove(uid);
                            }
                            self.state.active_user = self.state.users.keys().next().cloned();
                        }
                        login::LoginAction::SwitchUser(uid) => {
                            self.handle_user_switch(uid);
                        }
                        login::LoginAction::Login { email, password } => {
                            // TODO: actual login via SDK
                            let _ = (email, password);
                        }
                        login::LoginAction::NavigateToAddAccount => {
                            self.login_view.auth_page = AuthPage::LoginEmail;
                            self.login_view.email_input.clear();
                            self.login_view.login_password_input.clear();
                        }
                    }
                }
            }
            Message::Vault(msg) => {
                self.title_bar.dismiss_menu();
                let is_search = matches!(msg, VaultMessage::Search(_));
                for action in self.vault_view.update(msg) {
                    match action {
                        vault::VaultAction::SwitchUser(uid) => {
                            self.handle_user_switch(uid);
                        }
                        vault::VaultAction::FocusSearch => {
                            extra_task = iced::widget::operation::focus(
                                iced::widget::Id::new("vault-search"),
                            );
                        }
                        vault::VaultAction::AddAccount => {
                            self.state.screen = Screen::Login;
                            self.login_view.auth_page = AuthPage::LoginEmail;
                            self.login_view.email_input.clear();
                            self.login_view.login_password_input.clear();
                        }
                    }
                }
                if is_search {
                    let items: Vec<CipherItem> = self.all_vault_items().to_vec();
                    self.vault_view.refresh(&items);
                }
            }
            Message::TitleBar(msg) => {
                match self.state.screen {
                    Screen::Login => {
                        self.login_view.dropdown_open = false;
                        self.login_view.server_selector_open = false;
                    }
                    Screen::Vault => self.vault_view.dropdown_open = false,
                }
                for action in self.title_bar.update(msg) {
                    match action {
                        TitleBarAction::MenuAction(menu_action) => {
                            return self.handle_menu_action(menu_action);
                        }
                        TitleBarAction::Minimize => {
                            if let Some(id) = self.window_id {
                                return iced::window::minimize(id, true);
                            }
                        }
                        TitleBarAction::Maximize => {
                            if let Some(id) = self.window_id {
                                self.maximized = !self.maximized;
                                return iced::window::toggle_maximize(id);
                            }
                        }
                        TitleBarAction::Close => {
                            if let Some(id) = self.window_id {
                                return iced::window::close(id);
                            }
                        }
                        TitleBarAction::Drag => {
                            if let Some(id) = self.window_id {
                                return iced::window::drag(id);
                            }
                        }
                        TitleBarAction::ResizeEdge(dir) => {
                            if let Some(id) = self.window_id {
                                return iced::window::drag_resize(id, dir);
                            }
                        }
                    }
                }
            }
            Message::KeyPressed(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                let state = self.menu_state();
                if let Some(action) = crate::menu::find_shortcut_action(&key, modifiers, &state) {
                    self.title_bar.dismiss_menu();
                    return self.handle_menu_action(action);
                }
            }
            Message::KeyPressed(_) => {}
            Message::WindowOpened(id) => {
                self.menu_attached = true;
                self.window_id = Some(id);
                return iced::window::raw_id::<Message>(id).map(Message::GotRawId);
            }
            Message::GotRawId(raw_id) => {
                self.native_menu = crate::menu::attach_menu(raw_id);
                return iced::Task::none();
            }
            Message::PollNativeMenu => {
                if let Some(ref handle) = self.native_menu
                    && let Some(action) = crate::menu::poll_native_event(handle)
                {
                    return self.handle_menu_action(action);
                }
            }
            Message::SystemThemeChanged => {
                if self.theme_preference == ThemePreference::System {
                    self.current_theme = self.theme_preference.resolve(self.system_theme.get_scheme());
                }
            }
        }
        self.refresh_cache();
        extra_task
    }

    pub fn view(&self) -> Element<'_, Message, AppTheme> {
        let colors = &self.current_theme.colors;

        let page: Element<'_, Message, AppTheme> = match self.state.screen {
            Screen::Login => login::view(
                &self.login_view,
                &self.cached_email,
                &self.cached_server,
                &self.cached_accounts,
                &self.cached_unlock_alternatives,
                colors,
            )
            .map(Message::Login),
            Screen::Vault => vault::view(
                &self.cached_email,
                &self.cached_server,
                &self.vault_view.cached_items,
                &self.vault_view.all_items,
                self.vault_view.selected_item,
                self.vault_view.selected_id.as_deref(),
                self.vault_view.active_filter,
                &self.vault_view.search_query,
                &self.cached_accounts,
                self.vault_view.dropdown_open,
                self.vault_view.sidebar_mode,
                self.vault_view.active_section,
                self.vault_view.vault_tree_open,
                self.vault_view.send_tree_open,
                &self.vault_view.pane_state,
                self.vault_view.list_pane,
                self.vault_view.detail_pane,
                colors,
            )
            .map(Message::Vault),
        };

        if crate::menu::should_use_custom_menu_bar() {
            let menu_state = self.menu_state();
            let tb = title_bar::view(
                self.title_bar.open_menu,
                self.title_bar.open_submenu,
                self.maximized,
                &menu_state,
                colors,
            )
            .map(Message::TitleBar);
            let content: Element<'_, Message, AppTheme> =
                iced::widget::column![tb, page].height(iced::Fill).into();

            title_bar::resize_wrapper(content, |dir| {
                Message::TitleBar(title_bar::TitleBarMessage::ResizeEdge(dir))
            })
        } else {
            let tb = title_bar::view_empty().map(Message::TitleBar);
            iced::widget::column![tb, page].height(iced::Fill).into()
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn active_session(&self) -> Option<&UserSession> {
        self.state
            .active_user
            .as_ref()
            .and_then(|uid| self.state.users.get(uid))
    }

    fn all_vault_items(&self) -> &[CipherItem] {
        self.active_session()
            .map(|s| s.vault_items.as_slice())
            .unwrap_or(&[])
    }

    fn refresh_cache(&mut self) {
        let active_session = self.active_session().cloned();

        self.cached_email = active_session
            .as_ref()
            .map(|s| s.email.clone())
            .unwrap_or_else(|| "No account".into());

        self.cached_server = active_session
            .as_ref()
            .map(|s| s.server_url.clone())
            .unwrap_or_default();

        self.cached_accounts = self
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
        self.cached_unlock_alternatives =
            if let AuthPage::Unlock(method) = self.login_view.auth_page {
                active_session
                    .as_ref()
                    .map(|s| s.unlock_methods.alternatives(method))
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

        let vault_items = active_session
            .as_ref()
            .map(|s| s.vault_items.as_slice())
            .unwrap_or(&[]);
        self.vault_view.refresh(vault_items);

        if let Some(ref handle) = self.native_menu {
            crate::menu::sync_native_enabled(handle, &self.menu_state());
        }
    }

    fn handle_user_switch(&mut self, uid: String) {
        self.state.active_user = Some(uid.clone());
        let session = self.state.users.get(&uid);
        let locked = session.map(|s| s.locked).unwrap_or(true);
        if locked {
            self.state.screen = Screen::Login;
            let preferred = session
                .map(|s| s.unlock_methods.preferred())
                .unwrap_or(UnlockMethod::MasterPassword);
            self.login_view.auth_page = AuthPage::Unlock(preferred);
            self.login_view.password_input.clear();
            self.login_view.pin_input.clear();
            self.login_view.show_password = false;
        } else {
            self.state.screen = Screen::Vault;
        }
        self.vault_view.reset();
    }

    fn handle_menu_action(&mut self, action: crate::menu::MenuAction) -> iced::Task<Message> {
        use crate::menu::MenuAction;
        match action {
            MenuAction::Quit => {
                std::process::exit(0);
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
                if let Some(id) = self.window_id {
                    self.fullscreen = !self.fullscreen;
                    let mode = if self.fullscreen {
                        iced::window::Mode::Fullscreen
                    } else {
                        iced::window::Mode::Windowed
                    };
                    return iced::window::set_mode(id, mode);
                }
            }
            MenuAction::Minimize => {
                if let Some(id) = self.window_id {
                    return iced::window::minimize(id, true);
                }
            }
            MenuAction::Close => {
                if let Some(id) = self.window_id {
                    return iced::window::close(id);
                }
            }
            MenuAction::SearchVault => {
                if self.state.screen == Screen::Vault {
                    self.vault_view.search_query.clear();
                    self.refresh_cache();
                }
            }
            MenuAction::SyncNow | MenuAction::Reload => {
                self.refresh_cache();
            }
            MenuAction::HideToTray | MenuAction::ToggleAlwaysOnTop => {}
            MenuAction::About => {}
        }
        iced::Task::none()
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
