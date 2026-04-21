//! Platform / OS integration handlers — window chrome, system events, menu
//! dispatch, and tray actions. Kept together because menu/tray actions
//! frequently manipulate the window (minimize to tray, hide-to-tray on close,
//! show on IPC wake), and the window-message handler dispatches menu actions
//! on keyboard shortcuts.

use iced::Task;

use crate::{
    clipboard::Sensitivity,
    state::Screen,
    views::{
        about::{self, AboutMessage},
        settings::SettingsSnapshot,
        title_bar::{TitleBarEvent, WindowCommand},
        vault::widgets::search_bar,
    },
};

use super::super::{App, Message, SystemMessage, WindowInfo, WindowKind, WindowMessage};

impl App {
    // ── Sub-view event handlers ────────────────────────────────────────────

    pub(in crate::app) fn handle_titlebar_event(
        &mut self,
        event: TitleBarEvent,
    ) -> Task<Message> {
        match event {
            TitleBarEvent::MenuInvoked(menu_action) => self.handle_menu_action(menu_action),
            TitleBarEvent::Window(cmd) => self.handle_window_command(cmd),
        }
    }

    pub(in crate::app) fn handle_about_message(&mut self, msg: AboutMessage) -> Task<Message> {
        match msg {
            AboutMessage::CopyInfo => {
                self.clipboard.copy(about::info_string(), Sensitivity::Normal);
                Task::none()
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

    // ── Window lifecycle ───────────────────────────────────────────────────

    fn handle_window_command(&mut self, cmd: WindowCommand) -> Task<Message> {
        let id = self.main_window_id();
        match cmd {
            WindowCommand::Minimize => {
                if self.settings.minimize_to_tray && self.ensure_tray() {
                    self.hide_main_window()
                } else {
                    iced::window::minimize(id, true)
                }
            }
            WindowCommand::Maximize => {
                if let Some(info) = self.windows.get_mut(&id) {
                    info.maximized = !info.maximized;
                }
                iced::window::toggle_maximize(id)
            }
            WindowCommand::Close => {
                if self.settings.close_to_tray && self.ensure_tray() {
                    self.hide_main_window()
                } else {
                    iced::window::close(id)
                }
            }
            WindowCommand::Drag => iced::window::drag(id),
            WindowCommand::ResizeEdge(dir) => iced::window::drag_resize(id, dir),
        }
    }

    pub(in crate::app) fn handle_window_message(&mut self, msg: WindowMessage) -> Task<Message> {
        match msg {
            WindowMessage::Opened(id) => {
                // Native menu only attaches to the main window.
                if id == self.main_window_id() {
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
            WindowMessage::CloseRequested(id) => {
                // OS-native close (Alt+F4, macOS red button, native title-bar
                // X) — route to the same handler as the custom title-bar X
                // so close-to-tray applies uniformly. Other windows (About)
                // close normally.
                if id == self.main_window_id() {
                    self.handle_window_command(WindowCommand::Close)
                } else {
                    iced::window::close(id)
                }
            }
            WindowMessage::Closed(id) => {
                let was_main = id == self.main_window_id();
                self.windows.remove(&id);
                if was_main { iced::exit() } else { Task::none() }
            }
            WindowMessage::Resized(id, size) => {
                if let Some(info) = self.windows.get_mut(&id) {
                    info.size = size;
                }
                Task::none()
            }
            WindowMessage::KeyPressed(id, ev) => {
                // Keyboard shortcuts only affect the main window — Ctrl+F
                // in the About window must not trigger vault search.
                if id != self.main_window_id() {
                    return Task::none();
                }
                let iced::keyboard::Event::KeyPressed { key, modifiers, .. } = ev else {
                    return Task::none();
                };
                // Escape closes the settings modal before any menu shortcut
                // lookup — otherwise the user's Escape press would fall
                // through to widgets behind the modal.
                if matches!(key, iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape))
                    && self.settings_view.open
                {
                    self.settings_view.close();
                    return Task::none();
                }
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

    // ── System signals ─────────────────────────────────────────────────────

    pub(in crate::app) fn handle_system_message(&mut self, msg: SystemMessage) -> Task<Message> {
        match msg {
            SystemMessage::MudaEvent(event) => {
                // The muda receiver is shared between the native app menu
                // and the tray context menu — resolve against both. Native
                // menu matches always win; tray menu is the fallback.
                if let Some(handle) = &self.native_menu
                    && let Some(action) = handle.resolve(&event.id)
                {
                    return self.handle_menu_action(action);
                }
                if let Some(handle) = &self.tray
                    && let Some(action) = handle.resolve(&event.id)
                {
                    return self.handle_tray_action(action);
                }
                Task::none()
            }
            SystemMessage::TrayClick(action) => self.handle_tray_action(action),
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
            SystemMessage::InstanceWakeRequested => self.show_main_window(),
        }
    }

    // ── Menu + tray dispatch ───────────────────────────────────────────────

    fn handle_menu_action(&mut self, action: crate::menu::MenuAction) -> Task<Message> {
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
                let id = self.main_window_id();
                if let Some(info) = self.windows.get_mut(&id) {
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
                return iced::window::minimize(self.main_window_id(), true);
            }
            MenuAction::Close => {
                return iced::window::close(self.main_window_id());
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
            MenuAction::HideToTray => {
                if self.ensure_tray() {
                    return self.hide_main_window();
                }
            }
            MenuAction::ToggleAlwaysOnTop => {}
            MenuAction::Settings => {
                if let Some(uid) = self.active_user {
                    // The caller paths (muda poll, Ctrl+,) don't pass
                    // through the router dismiss block, so close any open
                    // dropdowns here before the modal appears on top.
                    self.dismiss_all_overlays();

                    let snap = SettingsSnapshot {
                        settings: self.settings.clone(),
                        prefs: self.settings.preferences_for(&uid),
                    };
                    self.settings_view.open_with(snap);
                }
            }
            MenuAction::About => {
                // Re-focus existing About window if already open.
                if let Some(id) = self.about_window_id() {
                    return iced::window::gain_focus(id);
                }

                let about_size = iced::Size::new(400.0, 280.0);
                let (about_id, open_task) = iced::window::open(iced::window::Settings {
                    size: about_size,
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
                    .insert(about_id, WindowInfo::new(WindowKind::About, about_size));

                return open_task.map(|id| Message::Window(WindowMessage::Opened(id)));
            }
        }
        Task::none()
    }

    fn handle_tray_action(&mut self, action: crate::tray::TrayAction) -> Task<Message> {
        use crate::tray::TrayAction;
        match action {
            TrayAction::ToggleShowHide => self.toggle_main_window_visibility(),
            TrayAction::LockVault => {
                self.handle_menu_action(crate::menu::MenuAction::LockAllVaults)
            }
            TrayAction::Exit => iced::exit(),
        }
    }

    // ── Window visibility helpers (tray + IPC wake) ────────────────────────

    fn hide_main_window(&mut self) -> Task<Message> {
        iced::window::set_mode(self.main_window_id(), iced::window::Mode::Hidden)
    }

    fn show_main_window(&mut self) -> Task<Message> {
        let id = self.main_window_id();
        Task::batch([
            iced::window::set_mode(id, iced::window::Mode::Windowed),
            iced::window::gain_focus(id),
        ])
    }

    /// Ask iced for the current mode and flip it. `Task::then` defers the
    /// decision until the mode resolves — one message round-trip slower than
    /// reading a local bool would be, imperceptible for a tray click. Iced
    /// is the single source of truth, so we can't drift out of sync with
    /// the OS-level window state. `Fullscreen` counts as visible.
    fn toggle_main_window_visibility(&mut self) -> Task<Message> {
        let id = self.main_window_id();
        iced::window::mode(id).then(move |mode| match mode {
            iced::window::Mode::Hidden => Task::batch([
                iced::window::set_mode(id, iced::window::Mode::Windowed),
                iced::window::gain_focus(id),
            ]),
            iced::window::Mode::Windowed | iced::window::Mode::Fullscreen => {
                iced::window::set_mode(id, iced::window::Mode::Hidden)
            }
        })
    }

    /// Lazily build the tray if needed. Returns `true` if a tray is available
    /// after the call — callers that would hide the window to the tray must
    /// check this and fall through to a normal minimize/close on `false`,
    /// otherwise the window becomes unrecoverable (hidden with no tray icon).
    fn ensure_tray(&mut self) -> bool {
        if self.tray.is_none() {
            self.tray = crate::tray::build();
            if self.tray.is_none() {
                tracing::warn!("tray requested but failed to initialise; continuing without tray");
            }
        }
        self.tray.is_some()
    }
}
