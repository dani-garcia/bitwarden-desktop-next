use iced::{Element, Subscription};

use crate::mock;
use crate::state::{AppState, CipherItem, Screen, SidebarFilter};
use crate::views::login::{self, LoginMessage};
use crate::views::vault::{self, VaultMessage};
use crate::widgets::account_switcher::{AccountEntry, AccountSwitcherMessage};
use crate::widgets::item_list::ItemListMessage;
use crate::widgets::search_bar::SearchMessage;
use crate::widgets::sidebar::SidebarMessage;

#[derive(Debug, Clone)]
pub enum Message {
    Login(LoginMessage),
    Vault(VaultMessage),
    WindowOpened(iced::window::Id),
    GotRawId(u64),
}

pub struct App {
    state: AppState,
    password_input: String,
    show_password: bool,
    search_query: String,
    active_filter: SidebarFilter,
    selected_item: Option<usize>,
    dropdown_open: bool,
    // Cached/computed data stored to avoid lifetime issues in view()
    cached_email: String,
    cached_server: String,
    cached_accounts: Vec<AccountEntry>,
    cached_items: Vec<CipherItem>,
    menu_attached: bool,
}

impl App {
    pub fn new() -> (Self, iced::Task<Message>) {
        let (mut users, active_user) = mock::mock_users();

        // DEV_SCREEN=vault skips straight to vault view (avoids recompilation for screenshots)
        let dev_screen = std::env::var("DEV_SCREEN").unwrap_or_default();
        let start_screen = if dev_screen.eq_ignore_ascii_case("vault") {
            if let Some(session) = users.get_mut(&active_user) {
                session.locked = false;
            }
            Screen::Vault
        } else {
            Screen::Login
        };

        let mut app = Self {
            state: AppState {
                users,
                active_user: Some(active_user),
                screen: start_screen,
            },
            password_input: String::new(),
            show_password: false,
            search_query: String::new(),
            active_filter: SidebarFilter::AllItems,
            selected_item: None,
            dropdown_open: false,
            cached_email: String::new(),
            cached_server: String::new(),
            cached_accounts: Vec::new(),
            cached_items: Vec::new(),
            menu_attached: false,
        };
        app.refresh_cache();

        (app, iced::Task::none())
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.menu_attached {
            return Subscription::none();
        }
        iced::event::listen_with(|event, _status, _id| {
            if let iced::Event::Window(iced::window::Event::Opened { .. }) = event {
                Some(Message::WindowOpened(_id))
            } else {
                None
            }
        })
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::Login(msg) => self.handle_login(msg),
            Message::Vault(msg) => self.handle_vault(msg),
            Message::WindowOpened(id) => {
                self.menu_attached = true;
                return iced::window::raw_id::<Message>(id).map(Message::GotRawId);
            }
            Message::GotRawId(raw_id) => {
                crate::menu::attach_menu(raw_id);
                return iced::Task::none();
            }
        }
        self.refresh_cache();
        iced::Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.state.screen {
            Screen::Login => login::view(
                &self.cached_email,
                &self.cached_server,
                &self.password_input,
                self.show_password,
                &self.cached_accounts,
                self.dropdown_open,
            )
            .map(Message::Login),
            Screen::Vault => vault::view(
                &self.cached_email,
                &self.cached_server,
                &self.cached_items,
                self.selected_item,
                self.active_filter,
                &self.search_query,
                &self.cached_accounts,
                self.dropdown_open,
            )
            .map(Message::Vault),
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
            VaultMessage::Sidebar(SidebarMessage::FilterSelected(filter)) => {
                self.active_filter = filter;
                self.selected_item = None;
            }
            VaultMessage::ItemList(ItemListMessage::ItemSelected(idx)) => {
                self.selected_item = Some(idx);
            }
            VaultMessage::Search(SearchMessage::QueryChanged(query)) => {
                self.search_query = query;
                self.selected_item = None;
            }
            VaultMessage::AccountSwitcher(asm) => self.handle_account_switcher(asm),
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
                let locked = self
                    .state
                    .users
                    .get(&uid)
                    .map(|s| s.locked)
                    .unwrap_or(true);
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

        self.cached_items = self.compute_filtered_items();
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

        let items = session.vault_items.iter().filter(|item| match self.active_filter {
            SidebarFilter::AllItems => true,
            SidebarFilter::Favorites => false,
            SidebarFilter::Category(cat) => item.category == cat,
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
}
