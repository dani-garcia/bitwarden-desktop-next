use iced::Task;

use crate::{
    app::{App, Message},
    domain::Screen,
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
                // Stamp `last_activity` so neither timer has a retroactive
                // head-start — the throttle path doesn't matter here since
                // there's no prior entry to suppress against.
                self.session_timeout.record_activity(uid);
                self.refresh_session_timeout_deadline();
                self.set_screen(Screen::Vault);
                tracing::info!(%uid, "unlock succeeded; loading vault + send lists");
                self.switch_to_vault_task(uid)
            }
            LoginEvent::AccountSwitcher(e) => self.handle_account_switcher_event(e),
        }
    }

    /// Lock the active user's keystore and route to its unlock screen.
    pub(crate) fn handle_lock_active(&mut self) -> Task<Message> {
        let Some(uid) = self.active_user else {
            return Task::none();
        };
        self.lock_user(&uid)
    }

    /// Sign the active user out: drop their decrypted vault cache, remove
    /// them from `ClientManager`, and either switch to another account or
    /// return to the login screen when no accounts remain.
    pub(crate) fn handle_log_out(&mut self) -> Task<Message> {
        if let Some(uid) = self.active_user {
            self.log_out_user(&uid);
            self.refresh_session_timeout_deadline();
        }
        // Drop any sticky Magnify search keyed to the user we just signed
        // out — otherwise the launcher still holds Arc clones of their
        // decrypted ciphers in `results`.
        self.magnify_reset_sticky();
        // The user we just logged out is already gone from `user_ids()`, so
        // any remaining id is a candidate for the next active user.
        let next_uid = self.client_manager.user_ids().into_iter().next();
        match next_uid {
            Some(uid) => self.handle_user_switch(uid),
            None => {
                self.active_user = None;
                self.views.login.reset_to_email_entry();
                self.set_screen(Screen::Login);
                self.views.login.auto_focus_task().map(Into::into)
            }
        }
    }
}
