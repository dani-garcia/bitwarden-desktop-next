//! Generic viewport-windowed scrollable list.
//!
//! Large lists (>~1k items) blow up `view()` performance even though iced's
//! `Column::draw()` culls offscreen children: `Column::layout()` still iterates
//! **every** child on every frame. That's O(N) layout work per redraw, which
//! makes scrolling AND window drag feel sluggish long before we hit the render
//! budget.
//!
//! This module windows the list by building widgets **only for rows inside
//! the visible scroll window** (plus a small overscan buffer), with a single
//! `Space` filler above and below to keep the scroll thumb reporting the
//! correct total height. Layout cost drops from O(N) to O(window).
//!
//! ## Upstream tracking
//!
//! iced has an open issue for first-class virtualized list support:
//! <https://github.com/iced-rs/iced/issues/160>. Once an upstream solution
//! lands (targeted at iced 1.0 per the issue), this module can be replaced
//! by a thin adapter around the native widget — or deleted entirely if the
//! upstream API fits our usage directly.
//!
//! ## Invariant
//!
//! Every row occupies exactly `row_height` pixels — `virtual_list` enforces
//! this by wrapping each row the caller produces in a fixed-height container.
//! The caller's row closure is free to return any `Element`; it will be
//! forced to the correct slot height with top-left alignment.
//!
//! ## Usage
//!
//! ```ignore
//! // Parent view struct:
//! pub struct MyView {
//!     scroll: virtual_list::ScrollState,
//!     items: Vec<Item>,
//! }
//!
//! // Message handler — one-liner via the `track` helper:
//! MyMessage::Scrolled(viewport) => self.scroll.track(viewport),
//!
//! // view() — return whatever Element you want for a row, no need to
//! // worry about enforcing its height:
//! virtual_list::view(
//!     &self.items,
//!     self.scroll,
//!     ROW_HEIGHT,
//!     |i, item| row_element(i, item),
//!     MyMessage::Scrolled,
//! )
//! .height(Fill)
//! .style(my_scrollable_style)
//! ```
//!
//! ## Why the parent owns the scroll state
//!
//! iced's `view()` function is pure — the parent passes `&self` in and gets
//! an `Element` back. For the visible window to change in response to scroll,
//! `view()` must read a scroll offset that was updated *before* it ran. That
//! offset therefore has to live on the parent's state, and scroll events
//! must be routed through the parent's message handler.
//!
//! Hiding this entirely would require a stateful custom iced widget that
//! manages its own `tree::State` and re-implements scroll event handling
//! from raw wheel/touch events — a lot of machinery to eliminate one message
//! variant and one `track()` call. Not worth it.

use iced::{
    Element, Fill, Length,
    widget::{Column, Scrollable, Space, container, scrollable},
};

use crate::theme::AppTheme;

/// Scroll state a parent view owns and updates from the `on_scroll` callback.
///
/// Stored on the view struct — not inside `virtual_list` — so iced's view
/// function stays pure and re-runs cheaply. The defaults seed a non-zero
/// viewport so the first paint renders a reasonable number of rows before
/// the first real scroll event arrives.
#[derive(Debug, Clone, Copy)]
pub struct ScrollState {
    pub offset_y: f32,
    pub viewport_height: f32,
}

impl Default for ScrollState {
    fn default() -> Self {
        // Seed with a viewport large enough to cover most monitors on first
        // paint. The scrollable's first real `on_scroll` event overwrites
        // this with the actual bounds, usually within the first frame the
        // user interacts with.
        Self {
            offset_y: 0.0,
            viewport_height: 1500.0,
        }
    }
}

impl ScrollState {
    /// Update the scroll state from an iced `on_scroll` viewport. Intended
    /// to be the full body of the caller's `Scrolled` message handler —
    /// everything the parent needs to do per scroll event is in here.
    pub fn track(&mut self, viewport: scrollable::Viewport) {
        self.offset_y = viewport.absolute_offset().y;
        self.viewport_height = viewport.bounds().height;
    }
}

/// Minimum overscan in rows. A very small viewport (e.g. only 2-3 rows
/// visible) still gets at least this many rows of buffer so quick scrolls
/// don't pop.
const MIN_OVERSCAN: usize = 3;

/// Overscan as a fraction of the visible window size. 30% on each side
/// gives a comfortable buffer for flick scrolls without materially
/// increasing the number of built widgets.
const OVERSCAN_RATIO: f32 = 0.3;

/// Build a virtualized scrollable list of uniform-height items.
///
/// Only items inside the visible scroll window (plus a small auto-computed
/// overscan buffer) are constructed as widgets. Top and bottom `Space`
/// fillers preserve the scroll thumb ratio so scrolling feels like a real
/// N-item list.
///
/// Each row returned by `render_row` is wrapped in a
/// `container(...).width(Fill).height(Length::Fixed(row_height))` so the
/// caller never has to enforce the height invariant themselves — return
/// any Element and it is forced to `row_height`. If your content is
/// naturally smaller it will be top-left aligned inside the slot; wrap
/// it in your own centering container if you need something else.
///
/// Returns an `iced::widget::Scrollable` so the caller can chain `.height()`
/// and `.style()` fluently without extra plumbing.
pub fn view<'a, T, Msg, F, G>(
    items: &'a [T],
    scroll: ScrollState,
    row_height: f32,
    render_row: F,
    on_scroll: G,
) -> Scrollable<'a, Msg, AppTheme>
where
    Msg: 'a,
    F: Fn(usize, &'a T) -> Element<'a, Msg, AppTheme>,
    G: Fn(scrollable::Viewport) -> Msg + 'a,
{
    debug_assert!(
        row_height > 0.0,
        "virtual_list::view row_height must be positive, got {row_height}"
    );

    let total = items.len();

    // Ignore negative offsets — iced can briefly report one during overscroll
    // bounce. Casting negative f32 to usize is saturating but only by accident;
    // clamp explicitly so the intent is clear.
    let offset_y = scroll.offset_y.max(0.0);

    // Compute the visible window with an auto-computed overscan buffer
    // on both sides. 30% of the visible window, floored at MIN_OVERSCAN.
    let visible_rows = ((scroll.viewport_height / row_height).ceil() as usize).max(1);
    let overscan = ((visible_rows as f32 * OVERSCAN_RATIO).ceil() as usize).max(MIN_OVERSCAN);
    let first_fully_visible = (offset_y / row_height) as usize;
    // Clamp `first` to `total` so the window stays valid even if `offset_y`
    // is stale (e.g. dataset shrank between frames and the scrollable hasn't
    // corrected its internal offset yet). Without the clamp, `first > last`
    // would produce a garbage spacer for one frame.
    let first = first_fully_visible.saturating_sub(overscan).min(total);
    let last = (first_fully_visible + visible_rows + overscan).min(total);

    // Assemble: [top spacer] + [visible rows] + [bottom spacer]
    let window_len = last - first;
    let mut children: Vec<Element<'a, Msg, AppTheme>> = Vec::with_capacity(window_len + 2);

    let top_h = (first as f32) * row_height;
    if top_h > 0.0 {
        children.push(Space::new().height(Length::Fixed(top_h)).into());
    }

    // Each row is wrapped in a fixed-height container so the height
    // invariant is enforced in one place: here. Consumers return whatever
    // Element they want and don't have to remember to set a height on it.
    for (offset, item) in items[first..last].iter().enumerate() {
        let i = first + offset;
        let row = render_row(i, item);
        children.push(
            container(row)
                .width(Fill)
                .height(Length::Fixed(row_height))
                .into(),
        );
    }

    let bottom_h = ((total - last) as f32) * row_height;
    if bottom_h > 0.0 {
        children.push(Space::new().height(Length::Fixed(bottom_h)).into());
    }

    scrollable(Column::with_children(children)).on_scroll(on_scroll)
}
