//! Account-switcher event routing. Both the login and vault/send views bubble
//! up `*Event::AccountSwitcher`, so the semantics live in one place.

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
                self.views.login.auto_focus_task().map(Message::login)
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
        // Magnify holds Arc clones of the previous user's decrypted ciphers
        // in `results`; drop them before flipping the active user so the
        // next summon doesn't restore them.
        self.magnify_reset_sticky();
        self.active_user = Some(uid);
        self.sidebar.active_vault_filter = crate::views::vault::VaultFilter::AllItems;
        self.sidebar.active_send_filter = crate::views::send::SendFilter::AllItems;
        self.views
            .vault
            .reset(&uid, self.sidebar.active_vault_filter);
        self.views.send.reset(&uid, self.sidebar.active_send_filter);
        let delay = self.settings.preferences_for(&uid).clear_clipboard;
        self.clipboard.set_timeout(delay.as_duration());

        // Becoming the active user counts as activity (parity with the
        // Electron client's `setAccountActivity` on `switchAccount`). Only
        // bump when already unlocked — the unlock-success path will
        // enroll otherwise.
        if self.client_manager.is_unlocked(&uid) {
            crate::services::session_timeout::record_activity(uid);
        }
        self.refresh_session_timeout_deadline();

        if !self.client_manager.is_unlocked(&uid) {
            self.show_login_for_active()
        } else {
            self.set_screen(Screen::Vault);
            self.switch_to_vault_task(uid)
        }
    }
}
