use iced::Task;

use crate::{state::Screen, views::login::LoginEvent};

use super::super::{App, Message};

impl App {
    pub(in crate::app) fn handle_login_event(&mut self, event: LoginEvent) -> Task<Message> {
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
}
