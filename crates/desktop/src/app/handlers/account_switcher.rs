//! Account-switcher event routing. Bubbles up from both the login and the
//! vault/send views via their own `*Event::AccountSwitcher` variants, so the
//! semantics live in exactly one place.

use iced::Task;

use crate::{
    app::{App, Message},
    components::account_switcher::AccountSwitcherEvent,
    domain::{Screen, UserId},
    services::menu::MenuAction,
};

impl App {
    pub(crate) fn handle_account_switcher_event(
        &mut self,
        event: AccountSwitcherEvent,
    ) -> Task<Message> {
        match event {
            AccountSwitcherEvent::SwitchUser { uid } => self.handle_user_switch(uid),
            AccountSwitcherEvent::AddAccount => {
                self.views.login.reset_to_email_entry();
                self.set_screen(Screen::Login);
                Task::none()
            }
            AccountSwitcherEvent::LockAll => self.handle_menu_action(MenuAction::LockAllVaults),
            AccountSwitcherEvent::Settings => self.handle_menu_action(MenuAction::Settings),
            AccountSwitcherEvent::LockActive => self.handle_lock_active(),
            AccountSwitcherEvent::LogOut => self.handle_log_out(),
        }
    }

    /// Switch the active user. Clears the previous user's vault/send state
    /// and, if the new user is unlocked, returns the task that repopulates
    /// both lists.
    pub(crate) fn handle_user_switch(&mut self, uid: UserId) -> Task<Message> {
        self.active_user = Some(uid);
        // Reset the sidebar filters on user switch — "AllItems" is the most
        // neutral entry point for a freshly-active user.
        self.sidebar.active_vault_filter = crate::components::sidebar::VaultFilter::AllItems;
        self.sidebar.active_send_filter = crate::components::sidebar::SendFilter::AllItems;
        self.views
            .vault
            .reset(&uid, self.sidebar.active_vault_filter);
        self.views.send.reset(&uid, self.sidebar.active_send_filter);
        // Re-apply the new user's clipboard clear delay so the app-global
        // clipboard manager matches their preference.
        let delay = self.settings.preferences_for(&uid).clear_clipboard;
        self.clipboard.set_timeout(delay.as_duration());

        if !self.client_manager.is_unlocked(&uid) {
            self.views
                .login
                .show_unlock_for(Some(&uid), &self.client_manager);
            self.set_screen(Screen::Login);
            Task::none()
        } else {
            self.set_screen(Screen::Vault);
            Task::batch([self.load_vault_list_task(uid), self.load_send_list_task(uid)])
        }
    }
}
