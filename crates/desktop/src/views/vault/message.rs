//! `VaultMessage` + `VaultEvent`.
//!
//! Variants whose payloads are large or sensitive use the [`NoDebug`] /
//! [`Summary`] wrappers from [`crate::debug_fmt`] so the enum just derives
//! `Debug` instead of hand-rolling per-variant formatters. iced warns when
//! a `Message`'s `Debug` takes >1 ms — the load-test account's 20k-cipher
//! `ListLoaded` measured ~31 ms with the derive default, hence `Summary`
//! on the bulk variant; `DetailLoaded` / `SaveCompleted` carry decrypted
//! ciphers, hence `NoDebug`.

use std::sync::Arc;

use bitwarden_vault::{CipherId, CipherListView, CipherType, CipherView};
use iced::widget::pane_grid;

use crate::{
    components::account_switcher::{AccountSwitcherEvent, AccountSwitcherMessage},
    debug_fmt::{NoDebug, Summary},
    domain::UserId,
    services::{
        clipboard::Sensitivity,
        sdk::{Collection, Organization},
    },
};

use super::widgets::{
    cipher_detail::CipherDetailMessage,
    cipher_edit::{CipherEditMessage, FolderOption},
    item_list::ItemListMessage,
    search_bar::SearchMessage,
};

/// Payload for [`VaultMessage::FormOptionsLoaded`]. Delivered as a single
/// value so the handler has one stale-check + one form-exists check.
#[derive(Clone)]
pub struct FormOptions {
    pub folders: Result<Vec<FolderOption>, String>,
    pub organizations: Vec<Organization>,
    pub collections: Vec<Collection>,
}

#[derive(Debug, Clone)]
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
    /// User clicked the +New button — toggle the cipher-type picker dropdown.
    ToggleNewItemMenu,
    /// User picked a type (from the +New dropdown, a File-menu item, or a
    /// keyboard accelerator). Mounts an empty form of that type.
    NewItem(CipherType),
    /// User confirmed the delete in the modal — fire the SDK soft-delete.
    ConfirmDeleteSelected,
    /// User dismissed the delete modal (Cancel, backdrop click, etc.).
    CancelDeleteSelected,
    ListLoaded(UserId, Result<Summary<Vec<Arc<CipherListView>>>, String>),
    DetailLoaded(UserId, CipherId, Result<NoDebug<Box<CipherView>>, String>),
    /// Fires once the cipher form's option lists (folders, organizations,
    /// collections) are all ready. Bundled into a single message because the
    /// three loads are always triggered together from the Edit handler and
    /// the UI has no use for a partial population. `folders` is a `Result`
    /// because it's the one that goes through an async SDK repo call; orgs
    /// and collections are sync reads off the already-loaded `ClientManager`.
    /// `FolderOption` is used (not `FolderView`) because `FolderView` isn't
    /// `Clone` and `VaultMessage` must be.
    FormOptionsLoaded(UserId, NoDebug<FormOptions>),
    SaveCompleted(UserId, Result<NoDebug<Box<CipherView>>, String>),
    DeleteCompleted(UserId, CipherId, Result<(), String>),
    /// Internal: dispatched on a one-frame delay from screen-mount handlers
    /// (post-unlock, switch-user-already-unlocked) so the focus operation
    /// runs against the freshly-mounted vault widget tree rather than the
    /// outgoing screen's tree. See CLAUDE.md "Iced Gotchas" → pane_grid
    /// first-mount behavior. Outside a fresh-mount context, the synchronous
    /// `auto_focus_task` / `focus_search_task` are sufficient.
    AutoFocusSearchDelayed,
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
    /// A save completed successfully. App pushes a success toast and kicks
    /// off a list reload so the sidebar reflects renames / ownership moves.
    ItemSaved {
        uid: UserId,
    },
    /// A soft-delete completed successfully. App pushes a success toast and
    /// reloads the list so the deleted row disappears.
    ItemDeleted {
        uid: UserId,
    },
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
    LaunchUrlRequested {
        uri: String,
    },
}
