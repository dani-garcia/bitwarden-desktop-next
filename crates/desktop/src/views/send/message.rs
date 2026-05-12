//! `SendMessage` + `SendEvent` — parallel to the vault view's message model.

use std::sync::Arc;

use bitwarden_send::{SendId, SendView as SdkSendView};
use iced::widget::pane_grid;

use crate::{
    components::account_switcher::{AccountSwitcherEvent, AccountSwitcherMessage},
    debug_fmt::{NoDebug, Summary},
    domain::UserId,
    services::clipboard::Sensitivity,
};

use super::widgets::{
    send_edit::SendEditMessage,
    send_list::{SearchMessage, SendListMessage},
};

#[derive(Debug, Clone, derive_more::From)]
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
    #[from(skip)]
    PaneResized(pane_grid::ResizeEvent),
    /// The "+ New" button on the header. Creates a fresh form whose type
    /// depends on the currently-selected Send sub-filter (Text / File).
    NewItem,
    ConfirmDeleteSelected,
    CancelDeleteSelected,
    /// List-load result. Carries a uid but isn't stale-gated — background
    /// users' caches are still populated; only the filtered view recomputes
    /// when the result is for the active user.
    #[from(skip)]
    ListLoaded(UserId, Result<Summary<Vec<Arc<SdkSendView>>>, String>),
    /// SDK completion routed through a single stale-user guard. The dispatch
    /// site drops the message when the active user changed during the await,
    /// so [`ForUserMessage`] handlers don't need their own guards.
    #[from(skip)]
    ForUser(UserId, ForUserMessage),
    /// Result of `ClientManager::generate_password` requested by the
    /// Send form's regenerate button.
    #[from(skip)]
    PasswordGenerated(Result<NoDebug<String>, String>),
}

/// Strictly-stale-gated SDK completions. Wrapped in
/// [`SendMessage::ForUser`] so the dispatch site checks
/// [`crate::app::UpdateCtx::is_active_user`] once and drops the message
/// when the user switched during the await.
#[derive(Debug, Clone)]
pub enum ForUserMessage {
    DetailLoaded(SendId, Result<NoDebug<Box<SdkSendView>>, String>),
    SaveCompleted(Result<NoDebug<Box<SdkSendView>>, String>),
    DeleteCompleted(SendId, Result<(), String>),
}

#[derive(Debug, Clone, derive_more::From)]
pub enum SendEvent {
    AccountSwitcher(AccountSwitcherEvent),
    #[from(skip)]
    ItemSaved {
        uid: UserId,
    },
    #[from(skip)]
    ItemDeleted {
        uid: UserId,
    },
    #[from(skip)]
    ClipboardCopyRequested {
        value: String,
        sensitivity: Sensitivity,
        toast_label: String,
    },
    /// Send form's regenerate button. App dispatches the SDK call.
    RegeneratePasswordRequested,
}
