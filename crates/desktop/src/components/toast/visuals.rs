//! Animation timing + progress-bar widget shared by the manager + overlay.
//!
//! `ToastTimer` is persisted in widget-tree state; `ToastVisuals` is the
//! per-frame snapshot the overlay writes and the toast row's style closures
//! read at draw time.

use std::{cell::Cell, rc::Rc, time::Instant};

use iced::{
    Background, Border, Length, Rectangle, Size,
    advanced::{
        Layout, Widget,
        layout::{Limits, Node},
        mouse::Cursor,
        renderer::{self, Quad},
        widget::Tree,
    },
    border::Radius,
};

use crate::theme::AppTheme;

use super::{FADE_IN_MS, FADE_OUT_MS, MAX_ALPHA, TIMEOUT, ToastStatus};

/// Per-toast animation timestamps. Stored in the widget tree state so they
/// survive across `view()` rebuilds.
#[derive(Debug, Clone, Copy)]
pub(super) struct ToastTimer {
    pub(super) created: Instant,
    /// Set when the auto-dismiss timer fires; the toast then fades out over
    /// `FADE_OUT_MS` and is published once complete.
    pub(super) dismissing: Option<Instant>,
    /// True while the cursor is over the toast — pauses the countdown and
    /// pins the visible state to "fully visible".
    pub(super) hovered: bool,
}

/// Per-frame animation values shared between the overlay (which writes) and
/// the toast row's style closures (which read). One cell per toast row,
/// refreshed on every `RedrawRequested` tick from the persisted `ToastTimer`.
#[derive(Debug, Clone, Copy)]
pub(super) struct ToastVisuals {
    pub(super) alpha: f32,
    pub(super) progress: f32,
}

impl ToastVisuals {
    pub(super) const HIDDEN: Self = Self {
        alpha: 0.0,
        progress: 0.0,
    };
}

/// Compute the current display alpha for a toast in `[0.0, MAX_ALPHA]`.
fn compute_alpha(timer: &ToastTimer, now: Instant) -> f32 {
    if timer.hovered && timer.dismissing.is_none() {
        return MAX_ALPHA;
    }

    let appear_ms = now.saturating_duration_since(timer.created).as_millis() as f32;
    let fade_in = (appear_ms / FADE_IN_MS as f32).clamp(0.0, 1.0);

    let fade_out = match timer.dismissing {
        Some(start) => {
            let dismiss_ms = now.saturating_duration_since(start).as_millis() as f32;
            (1.0 - (dismiss_ms / FADE_OUT_MS as f32)).clamp(0.0, 1.0)
        }
        None => 1.0,
    };

    fade_in * fade_out * MAX_ALPHA
}

/// Fraction of the visible-time budget remaining: 1.0 once fully visible,
/// 0.0 once auto-dismiss fires. Pinned to 1.0 while hovered. The visible
/// budget excludes the fade-in window so the bar starts at exactly 1.0
/// when the toast is fully opaque, and pins to 0.0 the moment dismissal
/// begins (so the bar doesn't drift below 0 during the fade-out).
fn compute_progress(timer: &ToastTimer, now: Instant) -> f32 {
    if timer.dismissing.is_some() {
        return 0.0;
    }
    if timer.hovered {
        return 1.0;
    }
    let fade_in_secs = FADE_IN_MS as f32 / 1000.0;
    let visible_budget = TIMEOUT.as_secs_f32() - fade_in_secs;
    let elapsed = now.saturating_duration_since(timer.created).as_secs_f32();
    let elapsed_visible = (elapsed - fade_in_secs).max(0.0);
    (1.0 - elapsed_visible / visible_budget).clamp(0.0, 1.0)
}

pub(super) fn refresh_visuals(timer: &ToastTimer, now: Instant) -> ToastVisuals {
    ToastVisuals {
        alpha: compute_alpha(timer, now),
        progress: compute_progress(timer, now),
    }
}

/// Custom widget so the fill ratio updates per-frame WITHOUT triggering
/// re-layout — `draw` reads the cell at paint time.
/// `container.width(Length::Fixed(...))` would force a relayout per tick.
pub(super) struct ToastProgressBar {
    pub(super) cell: Rc<Cell<ToastVisuals>>,
    pub(super) height: f32,
    pub(super) status: ToastStatus,
}

impl<Message> Widget<Message, AppTheme, iced::Renderer> for ToastProgressBar {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(self.height))
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &iced::Renderer, limits: &Limits) -> Node {
        let max_width = limits.max().width;
        Node::new(Size::new(max_width, self.height))
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &AppTheme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;

        let visuals = self.cell.get();
        let bounds = layout.bounds();
        let progress = visuals.progress.clamp(0.0, 1.0);
        let alpha = visuals.alpha;
        let filled_width = (bounds.width - 2.0) * progress;
        if filled_width <= 0.0 || alpha <= 0.0 {
            return;
        }
        let color = self.status.progress_color(&theme.colors);
        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    x: bounds.x + 1.0,
                    y: bounds.y - 1.0,
                    width: filled_width,
                    height: bounds.height,
                },
                border: Border::default().rounded(Radius::default().bottom(bounds.height / 2.0)),
                shadow: iced::Shadow::default(),
                snap: false,
            },
            Background::Color(color.scale_alpha(alpha)),
        );
    }
}
