//! Magnify launcher state.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use bitwarden_vault::{CipherId, CipherListView};
use iced::widget;

use crate::domain::UserId;

/// Sticky-search retention: re-summoning the launcher within this window of
/// the last interaction restores the previous query and selection. After
/// the window expires, the next summon starts fresh.
pub const STICKY_SEARCH_TTL: Duration = Duration::from_secs(300);

/// Widget id for the search input. Lives here (rather than in
/// `widgets::search`) because the handler-side focus / select-all
/// operations need it too.
pub const MAGNIFY_SEARCH_ID: widget::Id = widget::Id::new("magnify-search");

/// Widget id for the results scrollable. The handler emits
/// `widget::operation::scroll_to` against this id when arrow navigation
/// would push the selected row off-screen.
pub const MAGNIFY_RESULTS_SCROLL_ID: widget::Id = widget::Id::new("magnify-results");

/// Whether the launcher renders in locked-state pill mode or in normal
/// search mode. Computed on each summon from
/// `ClientManager::is_unlocked(active_user)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Locked,
    Unlocked,
}

#[derive(Default)]
pub struct MagnifyView {
    /// Iced window id for the launcher. Created hidden in `App::new` so
    /// every hotkey press is a cheap show / hide; remains `Some` for the
    /// lifetime of the app. Stays `Option` only because `Default::default()`
    /// has nothing meaningful to put here.
    pub(crate) window: Option<iced::window::Id>,

    /// Search query. Persists across summons subject to the sticky-search
    /// rules — see `App::handle_magnify_hotkey`.
    pub(crate) query: String,

    /// Filtered ciphers matching the current query, in display order.
    /// Recomputed on every `QueryChanged` and on summon. Stored as `Arc`
    /// snapshots so the view can borrow them without retouching the
    /// source `ItemCache`.
    pub(crate) results: Vec<Arc<CipherListView>>,

    /// Index into `results` — the highlighted row.
    pub(crate) selected: usize,

    /// Render mode resolved at summon time. Default `Locked` so the first
    /// `view()` call before the hotkey ever fires is a safe no-op.
    pub(crate) mode: Mode,

    /// Last interaction timestamp — drives the 5-minute sticky-search reset.
    pub(crate) last_used: Option<Instant>,

    /// User the sticky state belongs to. Cleared / replaced on user switch
    /// or lock so we never restore one user's query into another's view.
    pub(crate) anchored_user: Option<UserId>,

    /// In-flight password decrypt id. Set when `Ctrl+C` fires the async
    /// `full_cipher` task and cleared when the completion message arrives.
    /// Used to ignore stale completions if the user navigated away or the
    /// active user changed mid-decrypt.
    pub(crate) pending_password: Option<CipherId>,

    /// Top edge of the results scrollable's viewport, in pixels. Updated
    /// by the `Scrolled` message; used by the arrow-navigation handler to
    /// decide whether to emit a `scroll_to` Task that brings the selected
    /// row back on-screen.
    pub(crate) scroll_offset_y: f32,
}

impl MagnifyView {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether `last_used` is within `STICKY_SEARCH_TTL` of `now`.
    pub(crate) fn sticky_state_fresh(&self, now: Instant) -> bool {
        self.last_used
            .is_some_and(|t| now.saturating_duration_since(t) < STICKY_SEARCH_TTL)
    }

    /// Wipe transient search state. Called on user switch, lock, or
    /// sticky-search expiry. Window id, `mode`, and the anchored user are
    /// intentionally preserved.
    pub(crate) fn reset_search(&mut self) {
        self.query.clear();
        self.results.clear();
        self.selected = 0;
        self.pending_password = None;
        self.scroll_offset_y = 0.0;
    }

    /// Stamp the last-used timestamp. Called on hide so the next summon
    /// can decide whether to honour the sticky window.
    pub(crate) fn touch(&mut self) {
        self.last_used = Some(Instant::now());
    }

    /// Currently-selected cipher snapshot, if any.
    pub(crate) fn selected_item(&self) -> Option<&Arc<CipherListView>> {
        self.results.get(self.selected)
    }

    /// Recompute the filtered results against a fresh items slice. Only
    /// clamps `selected` if it falls outside the new result set — the
    /// caller is responsible for resetting selection (e.g. on a fresh
    /// `QueryChanged`). Sticky restore on summon depends on selection
    /// surviving the recompute pass. Archived and deleted ciphers are
    /// excluded — they live in dedicated vault views and shouldn't
    /// surface in a quick-launcher search.
    pub(crate) fn recompute(&mut self, all_items: &[Arc<CipherListView>]) {
        if self.query.is_empty() {
            self.results.clear();
            self.selected = 0;
            self.scroll_offset_y = 0.0;
            return;
        }
        let query = self.query.to_lowercase();
        self.results = all_items
            .iter()
            .filter(|item| {
                item.deleted_date.is_none()
                    && item.archived_date.is_none()
                    && crate::services::search::matches_query(item, &query)
            })
            .cloned()
            .collect();
        if self.selected >= self.results.len() {
            self.selected = self.results.len().saturating_sub(1);
        }
    }
}
