//! Account → Fingerprint phrase modal handler. Routed at the top level
//! (not via `ViewMessage`) — the modal isn't a `View`, its state lives on
//! `App`, and the actions are one-liners that don't need `UpdateCtx`.

use iced::Task;

use crate::{
    app::{App, Message},
    components::toast::Toast,
    fl,
    views::fingerprint_phrase::{self, FingerprintMessage},
};

impl App {
    pub(crate) fn handle_fingerprint_message(&mut self, msg: FingerprintMessage) -> Task<Message> {
        match msg {
            FingerprintMessage::Close => {
                self.fingerprint.close();
            }
            FingerprintMessage::OpenLearnMore => {
                fingerprint_phrase::open_learn_more();
                self.fingerprint.close();
            }
            FingerprintMessage::Copy => {
                let phrase = self.fingerprint.phrase().to_string();
                if phrase.is_empty() {
                    return Task::none();
                }
                // Skip `copy_and_toast` here so `minimize_on_copy` doesn't
                // hide the still-open modal out from under the user.
                self.clipboard
                    .copy(phrase, crate::services::clipboard::Sensitivity::Normal);
                self.push_toast(Toast::success(fl!("vault-toast-copied-fingerprint"), None));
            }
        }
        Task::none()
    }
}
