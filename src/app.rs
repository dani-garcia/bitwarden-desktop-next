use iced::{Element, Subscription, theme::Base, widget::pane_grid};

use crate::{
    components::account_switcher::{AccountEntry, AccountSwitcherMessage},
    mock,
    state::{AppState, CipherItem, NavSection, Screen, SidebarFilter, SidebarMode},
    theme::AppTheme,
    views::{
        login::{self, LoginMessage},
        vault::{
            self, VaultMessage,
            widgets::{
                item_list::ItemListMessage,
                search_bar::SearchMessage,
                sidebar::SidebarMessage,
            },
        },
    },
};

#[derive(Debug, Clone)]
pub enum Message {
    Login(LoginMessage),
    Vault(VaultMessage),
    TitleBar(crate::views::title_bar::TitleBarMessage),
    #[allow(dead_code)]
    ToggleTheme,
    KeyPressed(iced::keyboard::Event),
    WindowOpened(iced::window::Id),
    GotRawId(u64),
}

pub struct App {
    state: AppState,
    pub current_theme: AppTheme,
    password_input: String,
    show_password: bool,
    search_query: String,
    active_filter: SidebarFilter,
    selected_item: Option<usize>,
    selected_id: Option<String>,
    dropdown_open: bool,
    sidebar_mode: SidebarMode,
    active_section: NavSection,
    vault_tree_open: bool,
    send_tree_open: bool,
    open_menu: Option<usize>,
    open_submenu: Option<usize>,
    fullscreen: bool,
    maximized: bool,
    // Cached/computed data stored to avoid lifetime issues in view()
    cached_email: String,
    cached_server: String,
    cached_accounts: Vec<AccountEntry>,
    cached_items: Vec<CipherItem>,
    all_items: Vec<CipherItem>,
    menu_attached: bool,
    window_id: Option<iced::window::Id>,
    // PaneGrid for item list / detail pane split
    pub pane_state: pane_grid::State<PaneKind>,
    pub list_pane: pane_grid::Pane,
    pub detail_pane: pane_grid::Pane,
}

#[derive(Debug, Clone, Copy)]
pub enum PaneKind {
    List,
    Detail,
}

impl App {
    pub fn new() -> (Self, iced::Task<Message>) {
        let (users, active_user) = mock::mock_users();

        let (mut pane_state, list_pane) = pane_grid::State::new(PaneKind::List);
        let (detail_pane, split_id) = pane_state
            .split(pane_grid::Axis::Vertical, list_pane, PaneKind::Detail)
            .unwrap();
        pane_state.resize(split_id, 0.4);

        let mut app = Self {
            state: AppState {
                users,
                active_user: Some(active_user),
                screen: Screen::Login,
            },
            current_theme: AppTheme::dark(),
            password_input: String::new(),
            show_password: false,
            search_query: String::new(),
            active_filter: SidebarFilter::AllItems,
            selected_item: None,
            selected_id: None,
            dropdown_open: false,
            sidebar_mode: SidebarMode::Expanded,
            active_section: NavSection::Vault,
            vault_tree_open: true,
            send_tree_open: false,
            open_menu: None,
            open_submenu: None,
            fullscreen: false,
            maximized: false,
            cached_email: String::new(),
            cached_server: String::new(),
            cached_accounts: Vec::new(),
            cached_items: Vec::new(),
            all_items: Vec::new(),
            menu_attached: false,
            window_id: None,
            pane_state,
            list_pane,
            detail_pane,
        };
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

        Subscription::batch([window_sub, keyboard_sub])
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        let mut extra_task = iced::Task::none();
        match message {
            Message::Login(msg) => {
                self.open_menu = None;
                self.open_submenu = None;
                self.handle_login(msg);
            }
            Message::Vault(msg) => {
                self.open_menu = None;
                self.open_submenu = None;
                if matches!(msg, VaultMessage::Search(_)) {
                    extra_task =
                        iced::widget::operation::focus(iced::widget::Id::new("vault-search"));
                }
                self.handle_vault(msg);
            }
            Message::ToggleTheme => {
                self.current_theme = match self.current_theme.mode() {
                    iced::theme::Mode::Dark => AppTheme::light(),
                    _ => AppTheme::dark(),
                };
            }
            Message::TitleBar(msg) => {
                self.dropdown_open = false;
                use crate::views::title_bar::TitleBarMessage;
                match msg {
                    TitleBarMessage::TopLevelClicked(i) => {
                        self.open_menu = if self.open_menu == Some(i) {
                            None
                        } else {
                            Some(i)
                        };
                        self.open_submenu = None;
                    }
                    TitleBarMessage::DismissMenu => {
                        self.open_menu = None;
                        self.open_submenu = None;
                    }
                    TitleBarMessage::TopLevelHovered(i) => {
                        self.open_menu = Some(i);
                        self.open_submenu = None;
                    }
                    TitleBarMessage::SubMenuHovered(_menu, item) => {
                        self.open_submenu = if item == usize::MAX { None } else { Some(item) };
                    }
                    TitleBarMessage::ItemClicked(menu, item) => {
                        self.open_menu = None;
                        self.open_submenu = None;
                        if let Some(action) = crate::menu::MENUS
                            .get(menu)
                            .and_then(|(_, entries)| entries.get(item))
                            .and_then(|e| e.action)
                        {
                            return self.handle_menu_action(action);
                        }
                    }
                    TitleBarMessage::SubMenuItemClicked(menu, parent, sub) => {
                        self.open_menu = None;
                        self.open_submenu = None;
                        if let Some(action) = crate::menu::MENUS
                            .get(menu)
                            .and_then(|(_, entries)| entries.get(parent))
                            .and_then(|e| e.children.get(sub))
                            .and_then(|e| e.action)
                        {
                            return self.handle_menu_action(action);
                        }
                    }
                    TitleBarMessage::MinimizeClicked => {
                        if let Some(id) = self.window_id {
                            return iced::window::minimize(id, true);
                        }
                    }
                    TitleBarMessage::MaximizeClicked => {
                        if let Some(id) = self.window_id {
                            self.maximized = !self.maximized;
                            return iced::window::toggle_maximize(id);
                        }
                    }
                    TitleBarMessage::CloseClicked => {
                        if let Some(id) = self.window_id {
                            return iced::window::close(id);
                        }
                    }
                    TitleBarMessage::DragStart => {
                        if let Some(id) = self.window_id {
                            return iced::window::drag(id);
                        }
                    }
                    TitleBarMessage::ResizeEdge(direction) => {
                        if let Some(id) = self.window_id {
                            return iced::window::drag_resize(id, direction);
                        }
                    }
                }
            }
            Message::KeyPressed(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                let state = self.menu_state();
                if let Some(action) = crate::menu::find_shortcut_action(&key, modifiers, &state) {
                    self.open_menu = None;
                    self.open_submenu = None;
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
                crate::menu::attach_menu(raw_id);
                return iced::Task::none();
            }
        }
        self.refresh_cache();
        extra_task
    }

    pub fn view(&self) -> Element<'_, Message, AppTheme> {
        let colors = &self.current_theme.colors;

        let page: Element<'_, Message, AppTheme> = match self.state.screen {
            Screen::Login => login::view(
                &self.cached_email,
                &self.cached_server,
                &self.password_input,
                self.show_password,
                &self.cached_accounts,
                self.dropdown_open,
                colors,
            )
            .map(Message::Login),
            Screen::Vault => vault::view(
                &self.cached_email,
                &self.cached_server,
                &self.cached_items,
                &self.all_items,
                self.selected_item,
                self.selected_id.as_deref(),
                self.active_filter,
                &self.search_query,
                &self.cached_accounts,
                self.dropdown_open,
                self.sidebar_mode,
                self.active_section,
                self.vault_tree_open,
                self.send_tree_open,
                &self.pane_state,
                self.list_pane,
                self.detail_pane,
                colors,
            )
            .map(Message::Vault),
        };

        if crate::menu::should_draw_title_bar() {
            let menu_state = self.menu_state();
            let title_bar = crate::views::title_bar::view(
                self.open_menu,
                self.open_submenu,
                self.maximized,
                &menu_state,
                colors,
            )
            .map(Message::TitleBar);
            let content: Element<'_, Message, AppTheme> =
                iced::widget::column![title_bar, page].height(iced::Fill).into();

            crate::views::title_bar::resize_wrapper(content, |dir| {
                Message::TitleBar(crate::views::title_bar::TitleBarMessage::ResizeEdge(dir))
            })
        } else {
            page
        }
    }

    fn handle_login(&mut self, msg: LoginMessage) {
        match msg {
            LoginMessage::PasswordChanged(pw) => self.password_input = pw,
            LoginMessage::TogglePasswordVisibility => self.show_password = !self.show_password,
            LoginMessage::Unlock => {
                if let Some(ref uid) = self.state.active_user
                    && let Some(session) = self.state.users.get_mut(uid)
                {
                    session.locked = false;
                }
                self.password_input.clear();
                self.show_password = false;
                self.state.screen = Screen::Vault;
            }
            LoginMessage::LogOut => {
                if let Some(ref uid) = self.state.active_user {
                    self.state.users.remove(uid);
                }
                let next = self.state.users.keys().next().cloned();
                self.state.active_user = next;
            }
            LoginMessage::AccountSwitcher(asm) => self.handle_account_switcher(asm),
        }
    }

    fn handle_vault(&mut self, msg: VaultMessage) {
        match msg {
            VaultMessage::Sidebar(sidebar_msg) => match sidebar_msg {
                SidebarMessage::FilterSelected(filter) => {
                    self.active_filter = filter;
                    self.selected_item = None;
                }
                SidebarMessage::ToggleSidebarMode => {
                    self.sidebar_mode = match self.sidebar_mode {
                        SidebarMode::Collapsed => SidebarMode::Expanded,
                        SidebarMode::Expanded => SidebarMode::Collapsed,
                    };
                }
                SidebarMessage::SectionSelected(section) => {
                    self.active_section = section;
                }
                SidebarMessage::ToggleVaultTree => {
                    self.vault_tree_open = !self.vault_tree_open;
                }
                SidebarMessage::ToggleSendTree => {
                    self.send_tree_open = !self.send_tree_open;
                }
            },
            VaultMessage::ItemList(item_msg) => match item_msg {
                ItemListMessage::ItemSelected(idx) => {
                    self.selected_item = Some(idx);
                    self.selected_id = self.cached_items.get(idx).map(|i| i.id.clone());
                }
                ItemListMessage::OpenExternal(_)
                | ItemListMessage::CopyUsername(_)
                | ItemListMessage::MoreOptions(_) => {} // stubs
            },
            VaultMessage::Search(SearchMessage::QueryChanged(query)) => {
                self.search_query = query;
            }
            VaultMessage::CloseDetailPane => {
                self.selected_item = None;
                self.selected_id = None;
            }
            VaultMessage::PaneResized(event) => {
                self.pane_state.resize(event.split, event.ratio);
            }
            VaultMessage::DetailPane(_) => {} // copy/open stubs
            VaultMessage::AccountSwitcher(asm) => self.handle_account_switcher(asm),
            VaultMessage::NewItem => {} // stub
        }
    }

    fn handle_account_switcher(&mut self, msg: AccountSwitcherMessage) {
        match msg {
            AccountSwitcherMessage::ToggleDropdown => {
                self.dropdown_open = !self.dropdown_open;
            }
            AccountSwitcherMessage::SwitchUser(uid) => {
                self.dropdown_open = false;
                self.state.active_user = Some(uid.clone());
                let locked = self.state.users.get(&uid).map(|s| s.locked).unwrap_or(true);
                if locked {
                    self.state.screen = Screen::Login;
                    self.password_input.clear();
                } else {
                    self.state.screen = Screen::Vault;
                }
                self.search_query.clear();
                self.selected_item = None;
                self.active_filter = SidebarFilter::AllItems;
            }
        }
    }

    fn refresh_cache(&mut self) {
        let active_session = self
            .state
            .active_user
            .as_ref()
            .and_then(|uid| self.state.users.get(uid));

        self.cached_email = active_session
            .map(|s| s.email.clone())
            .unwrap_or_else(|| "No account".into());

        self.cached_server = active_session
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

        self.all_items = active_session
            .map(|s| s.vault_items.clone())
            .unwrap_or_default();
        self.cached_items = self.compute_filtered_items();

        // Keep selected_item index in sync with selected_id after filtering
        if let Some(ref id) = self.selected_id {
            self.selected_item = self.cached_items.iter().position(|i| i.id == *id);
        }
    }

    fn compute_filtered_items(&self) -> Vec<CipherItem> {
        let Some(session) = self
            .state
            .active_user
            .as_ref()
            .and_then(|uid| self.state.users.get(uid))
        else {
            return vec![];
        };

        let items = session
            .vault_items
            .iter()
            .filter(|item| match self.active_filter {
                SidebarFilter::AllItems => true,
                SidebarFilter::Favorites => false,
                SidebarFilter::Category(cat) => item.category == cat,
                SidebarFilter::Archive => false,
                SidebarFilter::Trash => false,
            });

        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            items.cloned().collect()
        } else {
            items
                .filter(|item| {
                    item.name.to_lowercase().contains(&query)
                        || item
                            .username
                            .as_ref()
                            .is_some_and(|u| u.to_lowercase().contains(&query))
                        || item
                            .url
                            .as_ref()
                            .is_some_and(|u| u.to_lowercase().contains(&query))
                })
                .cloned()
                .collect()
        }
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
                self.password_input.clear();
                self.show_password = false;
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
                // If locked, can't search — do nothing
                if self.state.screen == Screen::Vault {
                    // Focus would go to search bar; for now just clear and let user type
                    self.search_query.clear();
                    self.refresh_cache();
                }
            }
            MenuAction::SyncNow | MenuAction::Reload => {
                // Stub: would trigger sync/reload in real app
                self.refresh_cache();
            }
            MenuAction::HideToTray | MenuAction::ToggleAlwaysOnTop => {
                // Stub: requires tray icon / window level support not yet implemented
            }
            MenuAction::About => {
                // Stub: would show about dialog
            }
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
