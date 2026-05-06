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
                NavSection::Import => self.open_import_modal(),
                NavSection::Export => self.open_export_modal(),
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

    /// Transition to a list screen (Vault or Send) and focus the search input.
    /// Same-screen calls (filter switch) preserve the query so the user can
    /// refine inside the new filter; cross-screen calls land via
    /// `apply_filter` reset. Clicking the sidebar while on Login mustn't flip
    /// the screen underneath the login flow.
    fn switch_to_list_screen(&mut self, target: Screen) -> Task<Message> {
        let (auto_focus, focus, load) = match target {
            Screen::Vault => (
                self.views.vault.auto_focus_task().map(Message::vault),
                self.views.vault.focus_search_task().map(Message::vault),
                self.active_user.map(|uid| self.load_vault_list_task(uid)),
            ),
            Screen::Send => (
                self.views.send.auto_focus_task().map(Message::send),
                self.views.send.focus_search_task().map(Message::send),
                self.active_user.map(|uid| self.load_send_list_task(uid)),
            ),
            Screen::Loading | Screen::Login => return Task::none(),
        };

        if self.screen == target {
            return auto_focus;
        }
        if !matches!(self.screen, Screen::Vault | Screen::Send) {
            return Task::none();
        }
        self.set_screen(target);
        match load {
            Some(load) => Task::batch([load, focus]),
            None => focus,
        }
    }

    fn switch_to_vault(&mut self) -> Task<Message> {
        self.switch_to_list_screen(Screen::Vault)
    }

    fn switch_to_send(&mut self) -> Task<Message> {
        self.switch_to_list_screen(Screen::Send)
    }
}
