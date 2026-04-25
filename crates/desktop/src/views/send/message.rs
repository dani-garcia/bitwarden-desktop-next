//! `SendMessage` + `SendEvent` — parallel to the vault view's message model.

use std::sync::Arc;

use bitwarden_send::{SendId, SendView as SdkSendView};
use iced::widget::pane_grid;

use crate::{
    components::{
        account_switcher::{AccountSwitcherEvent, AccountSwitcherMessage},
        toast::Toast,
    },
    domain::UserId,
    services::clipboard::Sensitivity,
};

use super::widgets::{
    send_edit::SendEditMessage,
    send_list::{SearchMessage, SendListMessage},
};

#[derive(Clone)]
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
    /// User confirmed delete in the modal.
    ConfirmDeleteSelected,
    /// User dismissed the delete modal.
    CancelDeleteSelected,
    /// Async task completion: full send list loaded.
    ListLoaded(UserId, Result<Vec<Arc<SdkSendView>>, String>),
    /// Async task completion: single send fetched for the form.
    DetailLoaded(UserId, SendId, Result<Box<SdkSendView>, String>),
    /// Async task completion: save finished.
    SaveCompleted(UserId, Result<Box<SdkSendView>, String>),
    /// Async task completion: delete finished.
    DeleteCompleted(UserId, SendId, Result<(), String>),
}

impl std::fmt::Debug for SendMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ItemList(m) => f.debug_tuple("ItemList").field(m).finish(),
            Self::Search(m) => f.debug_tuple("Search").field(m).finish(),
            Self::AccountSwitcher(m) => f.debug_tuple("AccountSwitcher").field(m).finish(),
            Self::SendEdit(m) => f.debug_tuple("SendEdit").field(m).finish(),
            Self::CloseFormPane => f.write_str("CloseFormPane"),
            Self::FinalizeSheetClose => f.write_str("FinalizeSheetClose"),
            Self::PaneResized(e) => f.debug_tuple("PaneResized").field(e).finish(),
            Self::NewItem => f.write_str("NewItem"),
            Self::ConfirmDeleteSelected => f.write_str("ConfirmDeleteSelected"),
            Self::CancelDeleteSelected => f.write_str("CancelDeleteSelected"),
            Self::ListLoaded(uid, result) => {
                let mut t = f.debug_tuple("ListLoaded");
                t.field(uid);
                match result {
                    Ok(items) => t.field(&format_args!("Ok(<{} items>)", items.len())),
                    Err(e) => t.field(&format_args!("Err({e})")),
                };
                t.finish()
            }
            Self::DetailLoaded(uid, id, result) => {
                let mut t = f.debug_tuple("DetailLoaded");
                t.field(uid);
                t.field(id);
                match result {
                    Ok(_) => t.field(&"Ok(<SendView>)"),
                    Err(e) => t.field(&format_args!("Err({e})")),
                };
                t.finish()
            }
            Self::SaveCompleted(uid, result) => {
                let mut t = f.debug_tuple("SaveCompleted");
                t.field(uid);
                match result {
                    Ok(_) => t.field(&"Ok(<SendView>)"),
                    Err(e) => t.field(&format_args!("Err({e})")),
                };
                t.finish()
            }
            Self::DeleteCompleted(uid, id, result) => f
                .debug_tuple("DeleteCompleted")
                .field(uid)
                .field(id)
                .field(result)
                .finish(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SendEvent {
    /// Account-switcher action, routed to the shared App handler.
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
}
