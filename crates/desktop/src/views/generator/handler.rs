use std::sync::Arc;

use iced::Task;

use crate::{
    app::{App, Message},
    components::toast::Toast,
    fl,
    services::{clipboard::Sensitivity, sdk::ClientManager},
    views::generator::{GenerateKind, GeneratorEvent, GeneratorMessage},
};

impl App {
    pub(crate) fn handle_generator_event(&mut self, event: GeneratorEvent) -> Task<Message> {
        match event {
            GeneratorEvent::Generate(kind) => self.run_generator(kind),
            GeneratorEvent::ClearHistory => {
                if let Some(uid) = self.active_user {
                    self.client_manager.clear_password_history(&uid);
                }
                // View has already cleared its cached list; nothing more
                // needed.
                Task::none()
            }
            GeneratorEvent::Copy(value) => {
                // Generator output is secret-grade (passwords, passphrases,
                // generated usernames feeding future credentials).
                self.clipboard.copy(value, Sensitivity::Sensitive);
                self.push_toast(Toast::success(fl!("generator-toast-copied"), None));
                Task::none()
            }
            GeneratorEvent::Toast(t) => {
                self.push_toast(t);
                Task::none()
            }
        }
    }

    /// Spawn the SDK call for the given generation kind, piping the result
    /// back as [`GeneratorMessage::Generated`]. No-op if there's no active
    /// user — generator is gated by `EnabledWhen::Unlocked` so this is a
    /// defensive guard.
    pub(crate) fn run_generator(&self, kind: GenerateKind) -> Task<Message> {
        let Some(uid) = self.active_user else {
            return Task::none();
        };
        let mgr: Arc<ClientManager> = Arc::clone(&self.client_manager);
        match kind {
            GenerateKind::Password(req) => Task::perform(
                async move { mgr.generate_password(&uid, req).await },
                |r| Message::generator(GeneratorMessage::Generated(r)),
            ),
            GenerateKind::Passphrase(req) => Task::perform(
                async move { mgr.generate_passphrase(&uid, req).await },
                |r| Message::generator(GeneratorMessage::Generated(r)),
            ),
            GenerateKind::Username(req) => Task::perform(
                async move { mgr.generate_username(&uid, req).await },
                |r| Message::generator(GeneratorMessage::Generated(r)),
            ),
        }
    }

    /// Open the Generator modal in its default tab and seed the first
    /// value. Called from the View → Generator menu action and the
    /// sidebar's Generator section. No-op when no user is active.
    pub(crate) fn open_generator_modal(&mut self) -> Task<Message> {
        let Some(uid) = self.active_user else {
            return Task::none();
        };
        self.open_overlay = None;
        self.views.generator.open_as_generator();
        let history = self.client_manager.password_history(&uid);
        self.views.generator.set_history(history);
        let kind = self.views.generator.current_request();
        self.run_generator(kind)
    }

    /// Open the Generator modal directly into history mode. Called from
    /// the View → Generator history menu action.
    pub(crate) fn open_generator_history(&mut self) -> Task<Message> {
        let Some(uid) = self.active_user else {
            return Task::none();
        };
        self.open_overlay = None;
        self.views.generator.open_as_history();
        let history = self.client_manager.password_history(&uid);
        self.views.generator.set_history(history);
        Task::none()
    }
}
