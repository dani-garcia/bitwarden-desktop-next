//! Account → Fingerprint phrase modal event handler.
//!
//! The view handles dismissal inline. Two side effects need App-level
//! state and bubble out as [`FingerprintEvent`]s: launching the help
//! URL in the OS browser, and pushing the phrase onto the clipboard
//! (with the matching success toast).

use iced::Task;

use crate::{
    app::{App, Message},
    components::toast::Toast,
    fl,
    views::fingerprint_phrase::{self, FingerprintEvent},
};

impl App {
    pub(crate) fn handle_fingerprint_event(&mut self, event: FingerprintEvent) -> Task<Message> {
        match event {
            FingerprintEvent::OpenLearnMore => {
                fingerprint_phrase::open_learn_more();
            }
            FingerprintEvent::Copy(phrase) => {
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
