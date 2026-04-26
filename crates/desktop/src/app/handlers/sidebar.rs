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
                    // Generator is a modal, not a screen — leave
                    // `active_section` pointing at the underlying screen
                    // so the sidebar highlight tracks where the user
                    // returns when the modal closes.
                    self.open_generator_modal()
                }
                NavSection::Import | NavSection::Export => {
                    // Placeholders — no screen switch until those views
                    // are implemented. Only the highlight changes.
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
                // Already-on-Vault filter switch keeps the user's query so
                // they can refine inside the new filter. View changes
                // (handled in `switch_to_vault`) clear the query because
                // crossing into a different list semantically resets search.
                let switch = self.switch_to_vault();
                let focus = self.views.vault.auto_focus_task().map(Message::vault);
                Task::batch([switch, focus])
            }
            SidebarMessage::SendFilterSelected(filter) => {
                self.sidebar.active_send_filter = filter;
                self.sidebar.active_section = NavSection::Send;
                if let Some(uid) = self.active_user {
                    self.views.send.apply_filter(&uid, filter);
                }
                let switch = self.switch_to_send();
                let focus = self.views.send.auto_focus_task().map(Message::send);
                Task::batch([switch, focus])
            }
        }
    }

    /// Transition to the Vault screen. No-op when already there; otherwise
    /// refresh the cipher list so it reflects any changes that happened
    /// while the user was on a different screen, clear the search query,
    /// and focus the search input.
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
        let focus = self.views.vault.focus_search_task().map(Message::vault);
        match self.active_user {
            Some(uid) => Task::batch([self.load_vault_list_task(uid), focus]),
            None => focus,
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
        let focus = self.views.send.focus_search_task().map(Message::send);
        match self.active_user {
            Some(uid) => Task::batch([self.load_send_list_task(uid), focus]),
            None => focus,
        }
    }
}
