//! Generic viewport-windowed scrollable list.
//!
//! `Column::layout()` walks every child every frame even when only a window is
//! drawn, so 1k+ rows make scrolling and window-drag jank. This module builds
//! widgets only for rows inside the scroll viewport (plus an overscan buffer)
//! and pads top/bottom with `Space` so the scrollbar still reports total
//! height. Layout cost drops from O(N) to O(window).
//!
//! Every row is forced to `row_height` pixels via a fixed-height container, so
//! the caller's row closure can return any `Element`. The parent owns the
//! `ScrollState` because iced's `view()` is pure — the offset has to be set
//! before the next frame reads it.

use iced::{
    Element, Fill, Length,
    widget::{Column, Scrollable, Space, container, scrollable},
};

use crate::theme::AppTheme;

/// Scroll state a parent view owns and updates from the `on_scroll`
/// callback. Stored on the view struct so iced's view function stays pure.
#[derive(Debug, Clone, Copy)]
pub struct ScrollState {
    pub offset_y: f32,
    pub viewport_height: f32,
}

impl Default for ScrollState {
    fn default() -> Self {
        // Large seed so the first paint covers typical monitors; on_scroll
        // overwrites with the real bounds.
        Self {
            offset_y: 0.0,
            viewport_height: 1500.0,
        }
    }
}

impl ScrollState {
    /// Update the scroll state from an iced `on_scroll` viewport.
    pub fn track(&mut self, viewport: scrollable::Viewport) {
        self.offset_y = viewport.absolute_offset().y;
        self.viewport_height = viewport.bounds().height;
    }
}

/// Minimum overscan in rows so quick scrolls on small viewports don't pop.
const MIN_OVERSCAN: usize = 5;

/// Overscan as a fraction of the visible window size.
const OVERSCAN_RATIO: f32 = 0.3;

/// Build a virtualized scrollable list of uniform-height items.
///
/// Only items in the visible scroll window (plus an overscan buffer) are
/// constructed. Top/bottom `Space` fillers preserve the scroll thumb ratio.
/// Each row is wrapped in a fixed-height container so callers can return
/// any Element without enforcing the height invariant themselves.
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

    // Ignore negative offsets — iced briefly reports one during overscroll
    // bounce. Casting negative f32 to usize would saturate by accident.
    let offset_y = scroll.offset_y.max(0.0);

    let visible_rows = ((scroll.viewport_height / row_height).ceil() as usize).max(1);
    let overscan = ((visible_rows as f32 * OVERSCAN_RATIO).ceil() as usize).max(MIN_OVERSCAN);
    let first_fully_visible = (offset_y / row_height) as usize;
    // Clamp `first` to `total` so the window stays valid when `offset_y`
    // is stale (e.g. dataset shrank between frames). Without the clamp,
    // `first > last` would produce a garbage spacer for one frame.
    let first = first_fully_visible.saturating_sub(overscan).min(total);
    let last = (first_fully_visible + visible_rows + overscan).min(total);

    let window_len = last - first;
    let mut children: Vec<Element<'a, Msg, AppTheme>> = Vec::with_capacity(window_len + 2);

    let top_h = (first as f32) * row_height;
    if top_h > 0.0 {
        children.push(Space::new().height(Length::Fixed(top_h)).into());
    }

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
