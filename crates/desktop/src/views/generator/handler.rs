use iced::Task;

use crate::{
    app::{App, Message},
    fl,
    services::{clipboard::Sensitivity, sdk::ClientExt},
    views::generator::{GenerateKind, GeneratorEvent, GeneratorMessage, Mode},
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
                self.copy_and_toast(value, Sensitivity::Sensitive, fl!("generator-toast-copied"))
            }
        }
    }

    /// Spawn the SDK call for the given generation kind, piping the result
    /// back as [`GeneratorMessage::Generated`]. No-op if there's no active
    /// user — generator is gated by `EnabledWhen::Unlocked` so this is a
    /// defensive guard.
    pub(crate) fn run_generator(&self, kind: GenerateKind) -> Task<Message> {
        self.perform_with_active_client(
            move |client| async move {
                match kind {
                    GenerateKind::Password(req) => client.generate_password(req).await,
                    GenerateKind::Passphrase(req) => client.generate_passphrase(req).await,
                    GenerateKind::Username(req) => client.generate_username(req).await,
                }
            },
            |_uid, r| Message::generator(GeneratorMessage::Generated(r)),
        )
    }

    /// Open the Generator modal in its default tab and seed the first
    /// value. Called from the View → Generator menu action and the
    /// sidebar's Generator section. No-op when no user is active.
    pub(crate) fn open_generator_modal(&mut self) -> Task<Message> {
        let Some(uid) = self.require_active_user_and_close_overlay() else {
            return Task::none();
        };
        self.views.generator.open(Mode::Generator);
        let history = self.client_manager.password_history(&uid);
        self.views.generator.set_history(history);
        let kind = self.views.generator.current_request();
        self.run_generator(kind)
    }

    /// Open the Generator modal directly into history mode. Called from
    /// the View → Generator history menu action.
    pub(crate) fn open_generator_history(&mut self) -> Task<Message> {
        let Some(uid) = self.require_active_user_and_close_overlay() else {
            return Task::none();
        };
        self.views.generator.open(Mode::History);
        let history = self.client_manager.password_history(&uid);
        self.views.generator.set_history(history);
        Task::none()
    }
}
