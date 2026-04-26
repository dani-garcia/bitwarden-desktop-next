//! Sidebar message routing. Section clicks may flip between `Screen::Vault`
//! and `Screen::Send`; filter changes are stored on `self.sidebar` and pushed
//! down via the affected view's `apply_filter`.

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
            SidebarMessage::SectionSelected(section) => match section {
                NavSection::Vault => {
                    self.sidebar.active_section = section;
                    self.switch_to_vault()
                }
                NavSection::Send => {
                    self.sidebar.active_section = section;
                    self.switch_to_send()
                }
                NavSection::Generator => {
                    // Modal, not a screen — leave `active_section` pointing
                    // at the underlying screen so the highlight tracks where
                    // the user returns when the modal closes.
                    self.open_generator_modal()
                }
                NavSection::Import | NavSection::Export => {
                    // Placeholders — only the highlight changes.
                    self.sidebar.active_section = section;
                    Task::none()
                }
            },
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

    /// Transition to the Vault screen and focus the search input. Same-screen
    /// calls (filter switch) preserve the query so the user can refine inside
    /// the new filter; cross-screen calls land via `apply_filter` reset.
    fn switch_to_vault(&mut self) -> Task<Message> {
        if self.screen == Screen::Vault {
            return self.views.vault.auto_focus_task().map(Message::vault);
        }
        // Clicking the sidebar while on Login mustn't flip the screen
        // underneath the login flow.
        if !matches!(self.screen, Screen::Vault | Screen::Send) {
            return Task::none();
        }
        self.set_screen(Screen::Vault);
        let focus = self.views.vault.focus_search_task().map(Message::vault);
        match self.active_user {
            Some(uid) => Task::batch([self.load_vault_list_task(uid), focus]),
            None => focus,
        }
    }

    fn switch_to_send(&mut self) -> Task<Message> {
        if self.screen == Screen::Send {
            return self.views.send.auto_focus_task().map(Message::send);
        }
        if !matches!(self.screen, Screen::Vault | Screen::Send) {
            return Task::none();
        }
        self.set_screen(Screen::Send);
        let focus = self.views.send.focus_search_task().map(Message::send);
        match self.active_user {
            Some(uid) => Task::batch([self.load_send_list_task(uid), focus]),
            None => focus,
        }
    }
}
