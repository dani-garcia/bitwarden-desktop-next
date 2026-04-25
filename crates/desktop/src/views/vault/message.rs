//! `VaultMessage` + `VaultEvent` + a hand-rolled `Debug` impl.
//!
//! iced debug-formats every update Message and warns if it takes >1 ms. The
//! default `#[derive(Debug)]` on `ListLoaded` walks through ~20k entries of
//! `Arc<CipherListView>` on the load-test account, which measured ~31 ms.
//! The only variants that carry a genuinely heavy payload are `ListLoaded`
//! and the option loads for the form; everything else is short. Hand-roll
//! `Debug` so bulk variants render as a terse summary and the cheap variants
//! keep their natural derive-like output.

use std::sync::Arc;

use bitwarden_vault::{CipherId, CipherListView, CipherView};
use iced::widget::pane_grid;

use crate::{
    components::{
        account_switcher::{AccountSwitcherEvent, AccountSwitcherMessage},
        toast::Toast,
    },
    domain::UserId,
    services::{
        clipboard::Sensitivity,
        sdk::{Collection, Organization},
    },
};

use super::widgets::{
    cipher_edit::{CipherEditMessage, FolderOption},
    cipher_detail::CipherDetailMessage,
    item_list::ItemListMessage,
    search_bar::SearchMessage,
};

/// Payload for [`VaultMessage::FormOptionsLoaded`]. Delivered as a single
/// value so the handler has one stale-check + one form-exists check.
#[derive(Debug, Clone)]
pub struct FormOptions {
    pub folders: Result<Vec<FolderOption>, String>,
    pub organizations: Vec<Organization>,
    pub collections: Vec<Collection>,
}

#[derive(Clone)]
pub enum VaultMessage {
    ItemList(ItemListMessage),
    Search(SearchMessage),
    AccountSwitcher(AccountSwitcherMessage),
    CipherDetail(CipherDetailMessage),
    CipherEdit(CipherEditMessage),
    CloseCipherDetail,
    /// Fires after the bottom-sheet outro animation completes — actually
    /// clears the selection. Split out so the sheet has content to render
    /// during its slide-down/fade-out.
    FinalizeSheetClose,
    PaneResized(pane_grid::ResizeEvent),
    NewItem,
    /// User confirmed the delete in the modal — fire the SDK soft-delete.
    ConfirmDeleteSelected,
    /// User dismissed the delete modal (Cancel, backdrop click, etc.).
    CancelDeleteSelected,
    /// Fires when the async `ClientManager::list_ciphers` task completes.
    ListLoaded(UserId, Result<Vec<Arc<CipherListView>>, String>),
    /// Fires when the async `ClientManager::full_cipher` task completes.
    DetailLoaded(UserId, CipherId, Result<Box<CipherView>, String>),
    /// Fires once the cipher form's option lists (folders, organizations,
    /// collections) are all ready. Bundled into a single message because the
    /// three loads are always triggered together from the Edit handler and
    /// the UI has no use for a partial population. `folders` is a `Result`
    /// because it's the one that goes through an async SDK repo call; orgs
    /// and collections are sync reads off the already-loaded `ClientManager`.
    /// `FolderOption` is used (not `FolderView`) because `FolderView` isn't
    /// `Clone` and `VaultMessage` must be.
    FormOptionsLoaded(UserId, FormOptions),
    /// Fires when `ClientManager::save_cipher` finishes.
    SaveCompleted(UserId, Result<Box<CipherView>, String>),
    /// Fires when `ClientManager::soft_delete_cipher` finishes.
    DeleteCompleted(UserId, CipherId, Result<(), String>),
}

impl std::fmt::Debug for VaultMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ItemList(m) => f.debug_tuple("ItemList").field(m).finish(),
            Self::Search(m) => f.debug_tuple("Search").field(m).finish(),
            Self::AccountSwitcher(m) => f.debug_tuple("AccountSwitcher").field(m).finish(),
            Self::CipherDetail(m) => f.debug_tuple("CipherDetail").field(m).finish(),
            Self::CipherEdit(m) => f.debug_tuple("CipherEdit").field(m).finish(),
            Self::CloseCipherDetail => f.write_str("CloseCipherDetail"),
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
                    Ok(_) => t.field(&"Ok(<CipherView>)"),
                    Err(e) => t.field(&format_args!("Err({e})")),
                };
                t.finish()
            }
            Self::FormOptionsLoaded(uid, opts) => {
                let folders = match &opts.folders {
                    Ok(fs) => format!("Ok(<{} folders>)", fs.len()),
                    Err(e) => format!("Err({e})"),
                };
                f.debug_tuple("FormOptionsLoaded")
                    .field(uid)
                    .field(&format_args!(
                        "folders={folders}, <{} orgs>, <{} collections>",
                        opts.organizations.len(),
                        opts.collections.len()
                    ))
                    .finish()
            }
            Self::SaveCompleted(uid, result) => {
                let mut t = f.debug_tuple("SaveCompleted");
                t.field(uid);
                match result {
                    Ok(_) => t.field(&"Ok(<CipherView>)"),
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

// ── Events ─────────────────────────────────────────────────────────────────
//
// Declarative domain facts the vault bubbles up for cross-cutting effects.
// Routed by App. Anything that only affects the vault's own state stays
// inside `VaultView::update` and never becomes an event.

#[derive(Debug, Clone)]
pub enum VaultEvent {
    /// Account-switcher action. Forwarded verbatim to
    /// `App::handle_account_switcher_event` so login, vault, and send share
    /// one dispatch site.
    AccountSwitcher(AccountSwitcherEvent),
    /// VaultView wants to show a cross-cutting toast notification.
    ToastRequested(Toast),
    /// A save completed successfully. App pushes a success toast and kicks
    /// off a list reload so the sidebar reflects renames / ownership moves.
    ItemSaved { uid: UserId },
    /// A soft-delete completed successfully. App pushes a success toast and
    /// reloads the list so the deleted row disappears.
    ItemDeleted { uid: UserId },
    /// User clicked a copy-to-clipboard button on the detail pane.
    /// Routed to `ClipboardManager::copy`; the handler also pushes a
    /// success toast with `toast_label` as the body.
    ClipboardCopyRequested {
        value: String,
        sensitivity: Sensitivity,
        toast_label: String,
    },
    /// User clicked the launch button on a login's URI.
    /// Routed to `clipboard::launch_url` which enforces scheme allowlist.
    LaunchUrlRequested { uri: String },
}
