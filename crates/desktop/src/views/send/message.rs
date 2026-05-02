//! `SendMessage` + `SendEvent` — parallel to the vault view's message model.

use std::sync::Arc;

use bitwarden_send::{SendId, SendView as SdkSendView};
use iced::widget::pane_grid;

use crate::{
    components::{
        account_switcher::{AccountSwitcherEvent, AccountSwitcherMessage},
        toast::Toast,
    },
    debug_fmt::{NoDebug, Summary},
    domain::UserId,
    services::clipboard::Sensitivity,
};

use super::widgets::{
    send_edit::SendEditMessage,
    send_list::{SearchMessage, SendListMessage},
};

#[derive(Debug, Clone)]
pub enum SendMessage {
    ItemList(SendListMessage),
    Search(SearchMessage),
    AccountSwitcher(AccountSwitcherMessage),
    SendEdit(SendEditMessage),
    CloseFormPane,
    /// Fires after the bottom-sheet outro animation completes — actually
    /// clears the selection. Split out so the sheet has content to render
    /// during its slide-down/fade-out.
    FinalizeSheetClose,
    PaneResized(pane_grid::ResizeEvent),
    /// The "+ New" button on the header. Creates a fresh form whose type
    /// depends on the currently-selected Send sub-filter (Text / File).
    NewItem,
    ConfirmDeleteSelected,
    CancelDeleteSelected,
    ListLoaded(UserId, Result<Summary<Vec<Arc<SdkSendView>>>, String>),
    DetailLoaded(UserId, SendId, Result<NoDebug<Box<SdkSendView>>, String>),
    SaveCompleted(UserId, Result<NoDebug<Box<SdkSendView>>, String>),
    DeleteCompleted(UserId, SendId, Result<(), String>),
    /// Result of `ClientManager::generate_password` requested by the
    /// Send form's regenerate button.
    PasswordGenerated(Result<NoDebug<String>, String>),
}

#[derive(Debug, Clone)]
pub enum SendEvent {
    AccountSwitcher(AccountSwitcherEvent),
    ToastRequested(Toast),
    ItemSaved {
        uid: UserId,
    },
    ItemDeleted {
        uid: UserId,
    },
    ClipboardCopyRequested {
        value: String,
        sensitivity: Sensitivity,
        toast_label: String,
    },
    /// Send form's regenerate button. App dispatches the SDK call.
    RegeneratePasswordRequested,
}
