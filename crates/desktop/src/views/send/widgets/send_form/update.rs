use super::{
    message::SendFormMessage,
    state::{FormAction, SendForm},
};

impl SendForm {
    pub fn update(&mut self, msg: SendFormMessage) -> FormAction {
        match msg {
            SendFormMessage::NameChanged(s) => self.set_name(s),
            SendFormMessage::TextAction(action) => self.text_content.perform(action),
            SendFormMessage::HideTextToggled(v) => self.set_text_hidden(v),
            SendFormMessage::ChooseFilePressed => {
                // TODO: wire up a real file picker. Tracked in docs/todo.md.
            }

            SendFormMessage::DeletionPresetChosen(preset) => self.set_deletion_preset(preset),
            SendFormMessage::AccessTypeChosen(t) => self.set_access_type(t),

            SendFormMessage::PasswordChanged(s) => self.set_password(s),
            SendFormMessage::PasswordRevealToggled => self.toggle_password_reveal(),
            SendFormMessage::PasswordRegenerate => self.regenerate_password(),
            SendFormMessage::PasswordCopy => {
                if !self.password.is_empty() {
                    return FormAction::CopyPassword(self.password.clone());
                }
            }
            SendFormMessage::CopyLinkPressed => {
                if let Some(url) = self.send_link() {
                    return FormAction::CopyLink(url);
                }
            }

            SendFormMessage::EmailsAction(action) => self.emails_content.perform(action),

            SendFormMessage::MaxAccessCountChanged(s) => self.set_max_access_count_raw(s),
            SendFormMessage::MaxAccessCountIncrement => self.bump_max_access_count(1),
            SendFormMessage::MaxAccessCountDecrement => self.bump_max_access_count(-1),

            SendFormMessage::HideEmailToggled(value) => self.hide_email = value,

            SendFormMessage::NotesAction(action) => self.notes_content.perform(action),

            SendFormMessage::SavePressed => return FormAction::Save,
            SendFormMessage::CancelPressed => return FormAction::Cancel,
            SendFormMessage::DeletePressed => return FormAction::Delete,
        }
        FormAction::None
    }
}
