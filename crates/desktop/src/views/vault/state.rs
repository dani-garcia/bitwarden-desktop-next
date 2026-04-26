//! `VaultView` struct + view-local domain types + simple state methods.
//!
//! Behaviour that depends on `UpdateCtx` (SDK calls, event routing) lives in
//! `update.rs`; rendering lives in `view.rs`.

use std::{collections::HashMap, sync::Arc};

use bitwarden_vault::{CipherId, CipherListView, CipherView};
use iced::Task;

use crate::{
    app::ViewTypes,
    components::{FadeInOut, collapsible_pane::CollapsiblePane, virtual_list},
    domain::UserId,
    services::sdk::Organization,
};

use super::{VaultEvent, VaultMessage, widgets::cipher_edit::CipherForm};

// ── View-local domain ──────────────────────────────────────────────────────

/// Ratio the detail/form pane opens to the first time. `0.6` matches the
/// old hardcoded split where the list took 60% and the detail 40%.
const INITIAL_DETAIL_PANE_RATIO: f32 = 0.4;

/// The currently-selected vault item. An item click sets index + id and
/// triggers an async decrypt that populates `detail`. When `form` is `Some`,
/// the right-hand pane renders the editable form instead of the read-only
/// detail view; `detail` stays populated throughout so cancel returns
/// instantly without a reload.
#[derive(Default)]
pub(super) struct Selection {
    pub(super) item: Option<usize>,
    pub(super) id: Option<CipherId>,
    pub(super) detail: Option<CipherView>,
    pub(super) form: Option<CipherForm>,
    /// Armed-delete state for the detail pane's inline confirm row.
    /// Reset when selection changes (via `clear()`), when the user cancels,
    /// or when any non-delete detail-pane message arrives. Wrapped in a
    /// `FadeInOut` so the modal animates in/out — call `.open()` to arm,
    /// `.close()` to disarm.
    pub(super) confirm_delete: FadeInOut,
    /// Animates the bottom sheet (narrow-mode only) in and out. Driven by
    /// the same setters that set `detail`: every code path that loads a
    /// detail calls `.open()`, and `CloseCipherDetail` calls `.close()`
    /// alongside scheduling a delayed clear so the outro has content to
    /// render.
    pub(super) sheet_fade: FadeInOut,
}

impl Selection {
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Vault item storage. `all` is the full decrypted list from the SDK;
/// `cached` is `all` with the current filter + search applied. The two
/// are always kept in sync via `VaultView::recompute_filtered`.
/// `organizations` is a snapshot of the user's orgs, refreshed alongside
/// the cipher list so the sidebar can render per-org rows without the SDK.
#[derive(Default)]
pub(super) struct ItemCache {
    pub(super) all: Vec<Arc<CipherListView>>,
    pub(super) cached: Vec<Arc<CipherListView>>,
    pub(super) organizations: Vec<Organization>,
}

// ── VaultView ──────────────────────────────────────────────────────────────

pub struct VaultView {
    pub(super) search_query: String,
    /// Persistent list/detail split. Mounted from construction so iced's
    /// `pane_grid::diff` never drops the right pane's child tree state
    /// when a cipher is selected — see `components::collapsible_pane`.
    pub(super) pane: CollapsiblePane,

    pub(super) selection: Selection,
    // Item storage keyed by user. Decrypted vault data is structurally
    // isolated per-user so it's impossible to accidentally show one user's
    // ciphers while another is active. `Arc` wrap because `CipherListView`
    // isn't `Clone` and both Message dispatch and filter recomputation
    // need cheap clones.
    pub(super) items: HashMap<UserId, ItemCache>,
    /// Scroll state for the windowed item list. Updated by the
    /// `ItemListMessage::Scrolled` handler on every scroll event so
    /// `view()` can build only the visible row widgets.
    pub(super) list_scroll: virtual_list::ScrollState,
}

impl ViewTypes for VaultView {
    type Message = VaultMessage;
    type Event = VaultEvent;
}

impl VaultView {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            pane: CollapsiblePane::new(INITIAL_DETAIL_PANE_RATIO),
            selection: Selection::default(),
            items: HashMap::new(),
            list_scroll: virtual_list::ScrollState::default(),
        }
    }

    /// Reset transient view state when switching users. Item caches are
    /// preserved in the map — keyed by user so they can't mix. Callers are
    /// expected to reset the sidebar filter separately (it lives on App).
    pub fn reset(&mut self, uid: &UserId, filter: crate::components::sidebar::VaultFilter) {
        self.search_query.clear();
        self.selection.clear();
        self.pane.close();
        self.list_scroll = virtual_list::ScrollState::default();
        self.recompute_filtered(uid, filter);
    }

    /// Remove a signed-out user's cached vault data.
    pub fn remove_user_items(&mut self, uid: &UserId) {
        self.items.remove(uid);
    }

    /// Cached organizations for a user, snapshotted whenever the cipher
    /// list reloads. App reads this to render per-org rows in the shared
    /// sidebar without needing a separate org-load task.
    pub fn organizations_for(&self, uid: &UserId) -> Option<&[Organization]> {
        self.items.get(uid).map(|ic| ic.organizations.as_slice())
    }

    /// Raw decrypted item list for a user — the unfiltered backing store.
    /// Includes archived and deleted ciphers; consumers (e.g. the Magnify
    /// launcher) are responsible for excluding `archived_date` / `deleted_date`
    /// rows where appropriate.
    pub fn all_items_for(&self, uid: &UserId) -> Option<&[Arc<CipherListView>]> {
        self.items.get(uid).map(|ic| ic.all.as_slice())
    }

    /// Clear the search query and return a task that gives the search input
    /// focus. Called by the `File → Search vault` menu action; encapsulated
    /// here so `app/` doesn't need to reach into `widgets::search_bar`.
    pub(crate) fn focus_search_task(&mut self) -> Task<VaultMessage> {
        self.search_query.clear();
        iced::widget::operation::focus(super::widgets::search_bar::SEARCH_ID)
    }

    /// Focus the search input *without* clearing the query. Fired on every
    /// transition into the Vault screen (sidebar tab, post-unlock route,
    /// user switch) so the user can type immediately. Differs from
    /// `focus_search_task` which is a deliberate "fresh search" action.
    pub(crate) fn auto_focus_task(&self) -> Task<VaultMessage> {
        iced::widget::operation::focus(super::widgets::search_bar::SEARCH_ID)
    }

    /// Variant of [`auto_focus_task`] for screen-mount transitions where
    /// the vault widget tree wasn't on screen the moment the focus task
    /// was queued — the operation would otherwise walk the outgoing
    /// screen's tree and miss the search input. The brief sleep yields to
    /// iced's runtime so view() rebuilds the tree (and pane_grid mounts
    /// its panes) before the focus operation runs.
    pub(crate) fn delayed_auto_focus_task() -> Task<VaultMessage> {
        Task::perform(
            tokio::time::sleep(std::time::Duration::from_millis(50)),
            |_| VaultMessage::AutoFocusSearchDelayed,
        )
    }
}
