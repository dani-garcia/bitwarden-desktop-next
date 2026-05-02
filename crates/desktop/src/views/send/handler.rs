use std::sync::Arc;

use bitwarden_generators::PasswordGeneratorRequest;
use iced::Task;

use crate::{
    app::{App, Message},
    components::toast::Toast,
    debug_fmt::NoDebug,
    fl,
    services::sdk::ClientManager,
    views::send::{SendEvent, SendMessage},
};

impl App {
    pub(crate) fn handle_send_event(&mut self, event: SendEvent) -> Task<Message> {
        match event {
            SendEvent::AccountSwitcher(e) => self.handle_account_switcher_event(e),
            SendEvent::ToastRequested(t) => {
                self.push_toast(t);
                Task::none()
            }
            SendEvent::ItemSaved { uid } => {
                self.push_toast(Toast::success(fl!("send-toast-item-saved"), None));
                self.load_send_list_task(uid)
            }
            SendEvent::ItemDeleted { uid } => {
                self.push_toast(Toast::success(fl!("send-toast-item-deleted"), None));
                self.load_send_list_task(uid)
            }
            SendEvent::ClipboardCopyRequested {
                value,
                sensitivity,
                toast_label,
            } => self.copy_and_toast(value, sensitivity, toast_label),
            SendEvent::RegeneratePasswordRequested => self.regenerate_send_password(),
        }
    }

    /// Drive the Send form's regenerate button through the SDK with a
    /// fixed sensible default (14-char, all charsets on, min one digit + one
    /// symbol). The value (not the history snapshot) is piped back into
    /// the form via `SendMessage::PasswordGenerated`; the SDK still records
    /// the entry in `ClientManager::password_history` as a side effect.
    fn regenerate_send_password(&self) -> Task<Message> {
        let Some(uid) = self.active_user else {
            return Task::none();
        };
        let mgr: Arc<ClientManager> = Arc::clone(&self.client_manager);
        let req = PasswordGeneratorRequest {
            lowercase: true,
            uppercase: true,
            numbers: true,
            special: true,
            length: 14,
            avoid_ambiguous: false,
            min_lowercase: None,
            min_uppercase: None,
            min_number: Some(1),
            min_special: Some(1),
        };
        Task::perform(
            async move { mgr.generate_password(&uid, req).await },
            |res| {
                Message::send(SendMessage::PasswordGenerated(
                    res.map(|(value, _)| NoDebug(value)),
                ))
            },
        )
    }
}
