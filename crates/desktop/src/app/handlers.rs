use iced::Task;

use crate::{
    state::Screen,
    views::{
        about::AboutMessage,
        login::LoginEvent,
        title_bar::{TitleBarEvent, WindowCommand},
        vault::{VaultEvent, widgets::search_bar},
    },
};

use super::{App, Message, SystemMessage, WindowInfo, WindowKind, WindowMessage};

impl App {
    // ── Sub-view event handlers ────────────────────────────────────────────
    //
    // One `handle_*_event` per sub-view. Each handler receives a declarative
    // event from its view's `update()` and translates it into App-level side
    // effects (state mutation, screen switches, follow-on async tasks).

    pub(super) fn handle_login_event(&mut self, event: LoginEvent) -> Task<Message> {
        match event {
            LoginEvent::Unlocked { uid } | LoginEvent::LoggedIn { uid } => {
                if self.active_user != Some(uid) {
                    tracing::debug!(
                        %uid,
                        "unlock event dropped: active user changed while in flight"
                    );
                    return Task::none();
                }
                self.screen = Screen::Vault;
                tracing::info!(%uid, "unlock succeeded; loading vault list");
                self.load_vault_list_task(uid)
            }
            LoginEvent::SignOutRequested => {
                if let Some(ref uid) = self.active_user {
                    self.vault_view.remove_user_items(uid);
                    // TODO: remove user from ClientManager (requires interior mutability)
                }
                let next_uid = self
                    .client_manager
                    .user_ids()
                    .find(|id| self.active_user.as_ref() != Some(id))
                    .cloned();
                match next_uid {
                    Some(uid) => self.handle_user_switch(uid),
                    None => {
                        self.active_user = None;
                        self.screen = Screen::Login;
                        self.login_view.reset_to_email_entry();
                        Task::none()
                    }
                }
            }
            LoginEvent::UserSelected { uid } => self.handle_user_switch(uid),
            LoginEvent::ToastRequested(t) => {
                self.push_toast(t);
                Task::none()
            }
        }
    }

    pub(super) fn handle_vault_event(&mut self, event: VaultEvent) -> Task<Message> {
        match event {
            VaultEvent::UserSelected { uid } => self.handle_user_switch(uid),
            VaultEvent::AddAccountRequested => {
                self.screen = Screen::Login;
                self.login_view.reset_to_email_entry();
                Task::none()
            }
            VaultEvent::ToastRequested(t) => {
                self.push_toast(t);
                Task::none()
            }
        }
    }

    pub(super) fn handle_titlebar_event(&mut self, event: TitleBarEvent) -> Task<Message> {
        match event {
            TitleBarEvent::MenuInvoked(menu_action) => self.handle_menu_action(menu_action),
            TitleBarEvent::Window(cmd) => self.handle_window_command(cmd),
        }
    }

    pub(super) fn handle_about_message(&mut self, msg: AboutMessage) -> Task<Message> {
        match msg {
            AboutMessage::CopyInfo => {
                iced::clipboard::write(crate::views::about::info_string()).discard()
            }
            AboutMessage::Close => {
                if let Some(id) = self.about_window_id() {
                    iced::window::close(id)
                } else {
                    Task::none()
                }
            }
        }
    }

    // ── Window + system handlers ───────────────────────────────────────────

    pub(super) fn handle_window_command(&mut self, cmd: WindowCommand) -> Task<Message> {
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

    pub(super) fn handle_window_message(&mut self, msg: WindowMessage) -> Task<Message> {
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

    pub(super) fn handle_system_message(&mut self, msg: SystemMessage) -> Task<Message> {
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
                self.theme.refresh();
                Task::none()
            }
            SystemMessage::CloseToast(idx) => {
                if idx < self.toasts.len() {
                    self.toasts.remove(idx);
                }
                Task::none()
            }
            SystemMessage::ClientManagerLoaded(mgr) => {
                self.client_manager = mgr;
                self.active_user = self.client_manager.user_ids().next().cloned();
                self.login_view
                    .show_unlock_for(self.active_user.as_ref(), &self.client_manager);
                self.screen = Screen::Login;
                Task::none()
            }
        }
    }

    // ── Menu action handler ────────────────────────────────────────────────

    pub(super) fn handle_menu_action(&mut self, action: crate::menu::MenuAction) -> Task<Message> {
        use crate::menu::MenuAction;
        match action {
            MenuAction::Quit => {
                return iced::exit();
            }
            MenuAction::LockAllVaults => {
                self.client_manager.lock_all();
                self.screen = Screen::Login;
                self.login_view
                    .show_unlock_for(self.active_user.as_ref(), &self.client_manager);
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
                if self.screen == Screen::Vault {
                    self.vault_view.search_query.clear();
                    self.refresh_cache();
                    return iced::widget::operation::focus(search_bar::SEARCH_ID);
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
}
