//! Sidebar message routing. Section clicks may transition between
//! `Screen::Vault` and `Screen::Send`; filter changes are stored on
//! `self.sidebar` and pushed down to the affected view via its
//! `apply_filter` method.

use iced::Task;

use crate::{
    app::{App, Message},
    components::sidebar::{NavSection, SidebarMessage, SidebarMode},
    domain::Screen,
};

impl App {
    pub(crate) fn handle_sidebar_message(&mut self, msg: SidebarMessage) -> Task<Message> {
        match msg {
            SidebarMessage::ToggleSidebarMode => {
                self.sidebar.mode = match self.sidebar.mode {
                    SidebarMode::Collapsed => SidebarMode::Expanded,
                    SidebarMode::Expanded => SidebarMode::Collapsed,
                };
                Task::none()
            }
            SidebarMessage::ToggleVaultTree => {
                self.sidebar.vault_tree_open = !self.sidebar.vault_tree_open;
                Task::none()
            }
            SidebarMessage::ToggleSendTree => {
                self.sidebar.send_tree_open = !self.sidebar.send_tree_open;
                Task::none()
            }
            SidebarMessage::SectionSelected(section) => {
                self.sidebar.active_section = section;
                match section {
                    NavSection::Vault => self.switch_to_vault(),
                    NavSection::Send => self.switch_to_send(),
                    NavSection::Generator | NavSection::Import | NavSection::Export => {
                        // Placeholders — no screen switch until those views
                        // are implemented. Only the highlight changes.
                        Task::none()
                    }
                }
            }
            SidebarMessage::VaultFilterSelected(filter) => {
                self.sidebar.active_vault_filter = filter;
                self.sidebar.active_section = NavSection::Vault;
                if let Some(uid) = self.active_user {
                    self.views.vault.apply_filter(&uid, filter);
                }
                self.switch_to_vault()
            }
            SidebarMessage::SendFilterSelected(filter) => {
                self.sidebar.active_send_filter = filter;
                self.sidebar.active_section = NavSection::Send;
                if let Some(uid) = self.active_user {
                    self.views.send.apply_filter(&uid, filter);
                }
                self.switch_to_send()
            }
        }
    }

    /// Transition to the Vault screen. No-op when already there; otherwise
    /// refresh the cipher list so it reflects any changes that happened
    /// while the user was on a different screen.
    fn switch_to_vault(&mut self) -> Task<Message> {
        if self.screen == Screen::Vault {
            return Task::none();
        }
        // Only switch while authenticated — clicking the sidebar while on
        // Login shouldn't flip the screen underneath the login flow.
        if !matches!(self.screen, Screen::Vault | Screen::Send) {
            return Task::none();
        }
        self.set_screen(Screen::Vault);
        match self.active_user {
            Some(uid) => self.load_vault_list_task(uid),
            None => Task::none(),
        }
    }

    fn switch_to_send(&mut self) -> Task<Message> {
        if self.screen == Screen::Send {
            return Task::none();
        }
        if !matches!(self.screen, Screen::Vault | Screen::Send) {
            return Task::none();
        }
        self.set_screen(Screen::Send);
        match self.active_user {
            Some(uid) => self.load_send_list_task(uid),
            None => Task::none(),
        }
    }
}
