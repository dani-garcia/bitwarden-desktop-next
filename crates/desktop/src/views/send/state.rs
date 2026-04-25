//! `SendView` struct + view-local domain types.

use std::{collections::HashMap, sync::Arc};

use bitwarden_send::{SendId, SendView as SdkSendView};
use iced::Task;

use crate::{
    app::ViewTypes,
    components::{FadeInOut, collapsible_pane::CollapsiblePane, sidebar::SendFilter, virtual_list},
    domain::UserId,
};

use super::{SendEvent, SendMessage, widgets::send_edit::SendForm};

/// The currently-selected send. Sends open directly into the edit form
/// when a row is clicked, so there's no read-only "detail" step — `form`
/// is populated either on "New" or on row click (once `DetailLoaded`
/// fires).
#[derive(Default)]
pub(super) struct Selection {
    pub(super) item: Option<usize>,
    pub(super) id: Option<SendId>,
    pub(super) form: Option<SendForm>,
    /// Armed-delete state for the inline confirmation modal. Wrapped in a
    /// `FadeInOut` so the modal animates in/out — call `.open()` to arm,
    /// `.close()` to disarm.
    pub(super) confirm_delete: FadeInOut,
}

impl Selection {
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Send item storage, per user. `all` is the full list from the (stub)
/// SDK; `cached` is `all` with the current filter + search applied.
#[derive(Default)]
pub(super) struct ItemCache {
    pub(super) all: Vec<Arc<SdkSendView>>,
    pub(super) cached: Vec<Arc<SdkSendView>>,
}

/// Ratio given to the form (right) pane the first time it's opened.
/// `0.5` splits the screen evenly.
const INITIAL_FORM_PANE_RATIO: f32 = 0.4;

pub struct SendView {
    pub(super) search_query: String,
    /// Persistent list/form split. Mounted from construction and only
    /// resized to toggle form visibility — see `components::collapsible_pane`.
    pub(super) pane: CollapsiblePane,
    pub(super) selection: Selection,
    pub(super) items: HashMap<UserId, ItemCache>,
    pub(super) list_scroll: virtual_list::ScrollState,
}

impl ViewTypes for SendView {
    type Message = SendMessage;
    type Event = SendEvent;
}

impl SendView {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            pane: CollapsiblePane::new(INITIAL_FORM_PANE_RATIO),
            selection: Selection::default(),
            items: HashMap::new(),
            list_scroll: virtual_list::ScrollState::default(),
        }
    }

    /// Called by App when the active send filter changes. Mirrors
    /// `VaultView::apply_filter`.
    pub fn apply_filter(&mut self, uid: &UserId, filter: SendFilter) {
        self.selection.clear();
        self.pane.close();
        self.recompute_filtered(uid, filter);
    }

    /// Reset transient state when switching users. The per-user `items`
    /// cache is preserved in the map.
    pub fn reset(&mut self, uid: &UserId, filter: SendFilter) {
        self.search_query.clear();
        self.selection.clear();
        self.pane.close();
        self.list_scroll = virtual_list::ScrollState::default();
        self.recompute_filtered(uid, filter);
    }

    /// Drop a user's cached send data on sign-out.
    pub fn remove_user_items(&mut self, uid: &UserId) {
        self.items.remove(uid);
    }

    /// Clear the search and focus the input. Intended for a future
    /// `File → Search Sends` menu item; the vault side has the same helper.
    #[expect(
        dead_code,
        reason = "wired up when the File menu gains a Search Sends item"
    )]
    pub(crate) fn focus_search_task(&mut self) -> Task<SendMessage> {
        self.search_query.clear();
        iced::widget::operation::focus(super::widgets::send_list::SEND_SEARCH_ID)
    }
}
