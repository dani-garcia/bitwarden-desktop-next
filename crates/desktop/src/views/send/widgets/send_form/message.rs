use iced::widget::text_editor;

use super::state::{AccessType, DeletionPreset};

#[derive(Debug, Clone)]
pub enum SendFormMessage {
    NameChanged(String),
    TextAction(text_editor::Action),
    HideTextToggled(bool),
    /// Placeholder: the file picker integration is tracked in
    /// `docs/todo.md`. Clicking the button does nothing for now.
    ChooseFilePressed,

    DeletionPresetChosen(DeletionPreset),
    AccessTypeChosen(AccessType),

    PasswordChanged(String),
    PasswordRevealToggled,
    PasswordRegenerate,
    PasswordCopy,
    CopyLinkPressed,

    EmailsAction(text_editor::Action),

    MaxAccessCountChanged(String),
    MaxAccessCountIncrement,
    MaxAccessCountDecrement,

    HideEmailToggled(bool),

    NotesAction(text_editor::Action),

    SavePressed,
    CancelPressed,
    DeletePressed,
}
