//! Two-pane split where the right side can be collapsed to zero width
//! while staying mounted in the widget tree.
//!
//! ## Why this exists
//!
//! `iced::widget::pane_grid::PaneGrid` has a custom `diff` that retains
//! child tree slots by matching the new `panes` list against a stashed
//! `Memory::order` — when it fails to find a previous order (first mount
//! OR the grid toggling in/out of the tree), every child tree is
//! allocated fresh. `text_input`'s `is_focused` resets to `None` and
//! `text_editor`'s focus/cursor state is dropped, so the first click is
//! ignored and the first keystroke sends a character then loses focus.
//!
//! The iced `pane_grid` example never removes-then-readds panes; diff
//! assumes a stable `panes` list across view calls. This wrapper makes
//! that invariant easy to uphold: construct the pane once, drive
//! visibility via the split ratio. Caller never toggles anything in or
//! out of the tree.
//!
//! See `CLAUDE.md` → "Iced Gotchas" for the longer write-up.

use std::cell::RefCell;

use iced::{
    Element, Fill,
    widget::{
        Space,
        pane_grid::{self, Axis, PaneGrid, ResizeEvent, Split},
        row,
    },
};

use crate::{components::separator_v, theme::AppTheme};

/// Which side of the split each pane occupies. Used internally as the
/// pane_grid `T` so the view function can route left/right content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

/// Split ratio that fully collapses the right pane — list takes 100 %,
/// form 0 %. `1.0` is iced's own "all-left" end of the 0..1 range.
const COLLAPSED: f32 = 1.0;

/// Above this ratio, we treat a user drag as a "close by dragging" and
/// don't persist it as the next open width. Below: remember as the
/// user's preferred width.
const REMEMBER_UP_TO: f32 = 0.98;

pub struct CollapsiblePane {
    state: pane_grid::State<Side>,
    split: Split,
    /// Last non-collapsed ratio the user settled on via the drag handle,
    /// or the initial open width if they've never dragged. Restored on
    /// every `open()` so a wider/narrower preference sticks across
    /// open/close cycles.
    open_ratio: f32,
    is_open: bool,
}

impl CollapsiblePane {
    /// Construct with the given initial open width (0..1). `0.5` is an
    /// even split. Starts collapsed so the left pane renders full-width
    /// until someone calls `open()`.
    pub fn new(initial_open_ratio: f32) -> Self {
        let (mut state, left) = pane_grid::State::new(Side::Left);
        let (_right, split) = state
            .split(Axis::Vertical, left, Side::Right)
            .expect("fresh single-pane state always splits");
        state.resize(split, COLLAPSED);
        Self {
            state,
            split,
            open_ratio: initial_open_ratio,
            is_open: false,
        }
    }

    /// Open the right pane to the user's last chosen width. Idempotent.
    pub fn open(&mut self) {
        self.state.resize(self.split, self.open_ratio);
        self.is_open = true;
    }

    /// Collapse the right pane to zero width while keeping it mounted.
    /// Idempotent.
    pub fn close(&mut self) {
        self.state.resize(self.split, COLLAPSED);
        self.is_open = false;
    }

    /// Persist the ratio from a user-initiated drag. Call from the
    /// `on_resize` message handler. Drags close to the collapsed end are
    /// applied but not remembered — otherwise a user who dragged the
    /// handle all the way closed would get a zero-width pane on next
    /// open.
    pub fn set_ratio(&mut self, ratio: f32) {
        self.state.resize(self.split, ratio);
        if ratio < REMEMBER_UP_TO {
            self.open_ratio = ratio;
        }
    }
}

/// Render the collapsible pane.
///
/// - `left` is always shown.
/// - `right` is shown when the pane is open; when `None`, the right pane
///   renders a zero-sized filler so iced's widget tree has something to
///   diff against on the next open (rather than tearing down state).
/// - `on_resize` fires when the user drags the handle; feed it back into
///   `CollapsiblePane::set_ratio`.
///
/// A vertical separator is drawn at the boundary between panes.
pub fn view<'a, M: 'a + Clone>(
    pane: &'a CollapsiblePane,
    left: Element<'a, M, AppTheme>,
    right: Option<Element<'a, M, AppTheme>>,
    on_resize: impl Fn(ResizeEvent) -> M + 'a,
) -> Element<'a, M, AppTheme> {
    // `PaneGrid::new` takes `impl Fn` and calls it once per pane at
    // construction (see `widget/src/pane_grid.rs` in the pinned iced
    // rev, `pub fn new`). We have the Elements already built, so stash
    // them in RefCell slots and `take()` each one on the corresponding
    // pane. The slots make the closure compatible with the required
    // `Fn` bound without needing to rebuild the content inside.
    let left_slot = RefCell::new(Some(left));
    let right_slot = RefCell::new(right);

    PaneGrid::new(&pane.state, move |_pane, side, _maximized| match side {
        Side::Left => {
            let element = left_slot
                .borrow_mut()
                .take()
                .expect("left pane view built twice");
            pane_grid::Content::new(element)
        }
        Side::Right => {
            let element = right_slot
                .borrow_mut()
                .take()
                .unwrap_or_else(|| Space::new().into());
            let with_separator = row![separator_v(), element].height(Fill);
            pane_grid::Content::new(with_separator)
        }
    })
    .on_resize(6, on_resize)
    .spacing(1)
    // min_size must be 0 so the right pane can genuinely collapse to
    // zero width. iced's default 50 would clamp us.
    .min_size(0)
    .into()
}
