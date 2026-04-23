use iced::Task;

use crate::{
    app::{App, Message},
    domain::Screen,
    services::menu::MenuAction,
    views::login::LoginEvent,
};

impl App {
    pub(crate) fn handle_login_event(&mut self, event: LoginEvent) -> Task<Message> {
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
            LoginEvent::SignOutRequested => self.handle_sign_out(),
            LoginEvent::UserSelected { uid } => self.handle_user_switch(uid),
            LoginEvent::LockAllRequested => self.handle_menu_action(MenuAction::LockAllVaults),
            LoginEvent::SettingsRequested => self.handle_menu_action(MenuAction::Settings),
            LoginEvent::LockActiveRequested => self.handle_lock_active(),
            LoginEvent::ToastRequested(t) => {
                self.push_toast(t);
                Task::none()
            }
        }
    }

    /// Lock the active user's keystore. If it was the active user (it
    /// always is when this is triggered from the active-account card),
    /// route to the unlock screen for that account.
    pub(crate) fn handle_lock_active(&mut self) -> Task<Message> {
        let Some(uid) = self.active_user else {
            return Task::none();
        };
        self.client_manager.lock(&uid);
        self.screen = Screen::Login;
        self.login_view
            .show_unlock_for(Some(&uid), &self.client_manager);
        Task::none()
    }

    /// Sign the active user out: drop their decrypted vault cache, remove
    /// them from `ClientManager`, and either switch to another account or
    /// return to the login screen when no accounts remain.
    pub(crate) fn handle_sign_out(&mut self) -> Task<Message> {
        if let Some(uid) = self.active_user {
            self.vault_view.remove_user_items(&uid);
            self.favicon.evict_user(&uid);
            self.client_manager.log_out(&uid);
        }
        let next_uid = self
            .client_manager
            .user_ids()
            .into_iter()
            .find(|id| self.active_user.as_ref() != Some(id));
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
}
