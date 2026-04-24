//! `VaultView` struct + view-local domain types + simple state methods.
//!
//! Behaviour that depends on `UpdateCtx` (SDK calls, event routing) lives in
//! `update.rs`; rendering lives in `view.rs`.

use std::{collections::HashMap, sync::Arc};

use bitwarden_vault::{CipherId, CipherListView, CipherView};
use iced::{Task, widget::pane_grid};

use crate::{app::ViewTypes, components::virtual_list, domain::UserId};

use super::{VaultEvent, VaultMessage, widgets::cipher_form::CipherForm};

// ── View-local domain ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::views::vault) enum SidebarFilter {
    AllItems,
    Favorites,
    Category(bitwarden_vault::CipherType),
    Archive,
    Trash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::views::vault) enum SidebarMode {
    Collapsed,
    Expanded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::views::vault) enum NavSection {
    Vault,
    Send,
    Generator,
    Import,
    Export,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum PaneKind {
    List,
    Detail,
}

// ── Sub-state ──────────────────────────────────────────────────────────────
//
// Three cohesive groups extracted from VaultView. Each group is a set of
// fields that are always read/written together. See docs/decisions.md →
// "View Architecture: Compositional MVU" for the rationale; this is the
// same grouping discipline applied one layer down.

/// Sidebar chrome + the filter it controls. Sidebar is the UI that owns
/// the active filter — the filter lives here, not on VaultView directly.
pub(in crate::views::vault) struct SidebarState {
    pub(in crate::views::vault) mode: SidebarMode,
    pub(in crate::views::vault) active_section: NavSection,
    pub(in crate::views::vault) active_filter: SidebarFilter,
    pub(in crate::views::vault) vault_tree_open: bool,
    pub(in crate::views::vault) send_tree_open: bool,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            mode: SidebarMode::Expanded,
            active_section: NavSection::Vault,
            active_filter: SidebarFilter::AllItems,
            vault_tree_open: true,
            send_tree_open: false,
        }
    }
}

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
    /// or when any non-delete detail-pane message arrives.
    pub(super) confirm_delete: bool,
}

impl Selection {
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Vault item storage. `all` is the full decrypted list from the SDK;
/// `cached` is `all` with the current filter + search applied. The two
/// are always kept in sync via `VaultView::recompute_filtered`.
#[derive(Default)]
pub(super) struct ItemCache {
    pub(super) all: Vec<Arc<CipherListView>>,
    pub(super) cached: Vec<Arc<CipherListView>>,
}

// ── VaultView ──────────────────────────────────────────────────────────────

pub struct VaultView {
    pub(super) search_query: String,
    pub(super) pane_state: pane_grid::State<PaneKind>,

    pub(super) sidebar: SidebarState,
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
        // `list_pane` and the returned `detail_pane` are needed only as local
        // handles during construction (one to split from, one to resize).
        // `PaneKind::List` / `PaneKind::Detail` discriminants in `pane_state`
        // are what `view()` matches on at render time — we don't need the
        // `Pane` handles themselves after this.
        let (mut pane_state, list_pane) = pane_grid::State::new(PaneKind::List);
        let (_detail_pane, split_id) = pane_state
            .split(pane_grid::Axis::Vertical, list_pane, PaneKind::Detail)
            .expect("splitting a fresh single-pane state always succeeds");
        pane_state.resize(split_id, 0.4);

        Self {
            search_query: String::new(),
            pane_state,
            sidebar: SidebarState::default(),
            selection: Selection::default(),
            items: HashMap::new(),
            list_scroll: virtual_list::ScrollState::default(),
        }
    }

    /// Reset transient view state when switching users. Item caches are
    /// preserved in the map — keyed by user so they can't mix.
    pub fn reset(&mut self, uid: &UserId) {
        self.search_query.clear();
        self.selection.clear();
        self.sidebar.active_filter = SidebarFilter::AllItems;
        self.list_scroll = virtual_list::ScrollState::default();
        self.recompute_filtered(uid);
    }

    /// Remove a signed-out user's cached vault data.
    pub fn remove_user_items(&mut self, uid: &UserId) {
        self.items.remove(uid);
    }

    /// Clear the search query and return a task that gives the search input
    /// focus. Called by the `File → Search vault` menu action; encapsulated
    /// here so `app/` doesn't need to reach into `widgets::search_bar`.
    pub(crate) fn focus_search_task(&mut self) -> Task<VaultMessage> {
        self.search_query.clear();
        iced::widget::operation::focus(super::widgets::search_bar::SEARCH_ID)
    }
}
