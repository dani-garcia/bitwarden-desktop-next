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

#[cfg(test)]
mod tests {
    use super::*;
    use bitwarden_send::SendType;

    fn form() -> SendForm {
        SendForm::new(SendType::Text)
    }

    #[test]
    fn max_access_count_accepts_digit_strings() {
        let mut form = form();
        form.update(SendEditMessage::MaxAccessCountChanged("42".into()));
        assert_eq!(form.max_access_count_raw, "42");
    }

    #[test]
    fn max_access_count_accepts_empty_string() {
        // Empty is the "unlimited" state — the form must accept it so the
        // user can clear a previously-typed limit without going through
        // the decrement button.
        let mut form = form();
        form.max_access_count_raw = "9".to_string();
        form.update(SendEditMessage::MaxAccessCountChanged(String::new()));
        assert_eq!(form.max_access_count_raw, "");
    }

    #[test]
    fn max_access_count_rejects_non_digit_input() {
        // The text input is filtered: non-digit characters don't update
        // the field at all (the previous value persists). This is what
        // makes pasting "1a2" leave just the prior value rather than
        // partially-accepting it.
        let mut form = form();
        form.max_access_count_raw = "5".to_string();
        form.update(SendEditMessage::MaxAccessCountChanged("abc".into()));
        assert_eq!(form.max_access_count_raw, "5", "non-digits rejected");

        form.update(SendEditMessage::MaxAccessCountChanged("1.5".into()));
        assert_eq!(form.max_access_count_raw, "5", "decimal rejected");

        form.update(SendEditMessage::MaxAccessCountChanged("-1".into()));
        assert_eq!(form.max_access_count_raw, "5", "negative rejected");
    }

    #[test]
    fn max_access_count_bump_from_empty_starts_at_one() {
        // Empty represents "unlimited" / zero. +1 enters the numeric range.
        let mut form = form();
        form.update(SendEditMessage::MaxAccessCountIncrement);
        assert_eq!(form.max_access_count_raw, "1");
    }

    #[test]
    fn max_access_count_bump_floors_at_empty() {
        // Decrementing past zero returns to empty (the "unlimited"
        // sentinel) rather than producing "-1" or "0". Both 0 and empty
        // serialize the same to the SDK; empty is the canonical UI shape.
        let mut form = form();
        form.max_access_count_raw = "1".to_string();
        form.update(SendEditMessage::MaxAccessCountDecrement);
        assert_eq!(form.max_access_count_raw, "");

        // One more decrement stays at empty.
        form.update(SendEditMessage::MaxAccessCountDecrement);
        assert_eq!(form.max_access_count_raw, "");
    }
}
