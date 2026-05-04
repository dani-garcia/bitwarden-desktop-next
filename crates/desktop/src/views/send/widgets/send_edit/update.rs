use super::{
    message::SendEditMessage,
    state::{FormEvent, SendForm},
};

impl SendForm {
    pub fn update(&mut self, msg: SendEditMessage) -> FormEvent {
        match msg {
            SendEditMessage::NameChanged(s) => self.set_name(s),
            SendEditMessage::TextAction(action) => self.text_content.perform(action),
            SendEditMessage::HideTextToggled(v) => self.set_text_hidden(v),
            SendEditMessage::ChooseFilePressed => {
                // TODO: wire up a real file picker. Tracked in docs/todo.md.
            }

            SendEditMessage::DeletionPresetChosen(preset) => self.set_deletion_preset(preset),
            SendEditMessage::AccessTypeChosen(t) => self.set_access_type(t),

            SendEditMessage::PasswordChanged(s) => self.set_password(s),
            SendEditMessage::PasswordRevealToggled => self.toggle_password_reveal(),
            SendEditMessage::PasswordRegenerate => return FormEvent::RegeneratePassword,
            SendEditMessage::PasswordCopy => {
                if !self.password.is_empty() {
                    return FormEvent::CopyPassword(self.password.clone());
                }
            }
            SendEditMessage::CopyLinkPressed => {
                if let Some(url) = self.send_link() {
                    return FormEvent::CopyLink(url);
                }
            }

            SendEditMessage::EmailsAction(action) => self.emails_content.perform(action),

            SendEditMessage::MaxAccessCountChanged(s) => self.set_max_access_count_raw(s),
            SendEditMessage::MaxAccessCountIncrement => self.bump_max_access_count(1),
            SendEditMessage::MaxAccessCountDecrement => self.bump_max_access_count(-1),

            SendEditMessage::HideEmailToggled(value) => self.hide_email = value,

            SendEditMessage::NotesAction(action) => self.notes_content.perform(action),

            SendEditMessage::SavePressed => return FormEvent::Save,
            SendEditMessage::CancelPressed => return FormEvent::Cancel,
            SendEditMessage::DeletePressed => return FormEvent::Delete,
        }
        FormEvent::None
    }
}
