//! Stacked toast notifications with fade in/out animations. Adapted from
//! iced's official `examples/toast` and extended with per-toast opacity
//! fades, a countdown progress bar, and hover-to-pause.
//!
//! Animation pipeline: persistent `ToastTimer`s in widget-tree state drive
//! per-frame `Rc<Cell<ToastVisuals>>` cells that style closures read at draw
//! time — no layout invalidation. Auto-dismiss is two-phase (timer →
//! fade-out → `on_close`); manual close publishes immediately. Hovering
//! pins to full visibility and cancels any in-flight dismissal.
//!
//! ## Layout
//!
//! - [`Toast`] / [`ToastStatus`] — public data + builders below.
//! - [`Manager`] — the public widget that wraps app content and overlays
//!   the toast stack ([`widget`]).
//! - [`overlay`] — `ToastOverlay` driving the per-frame state machine.
//! - [`visuals`] — `ToastTimer`, `ToastVisuals`, the progress-bar widget.

mod overlay;
mod visuals;
mod widget;

use std::time::Duration;

use iced::Color;

use crate::{components::icons, theme::AppColors};

pub use widget::Manager;

// ── Tunables ───────────────────────────────────────────────────────────────

/// How long each toast stays visible after fade-in completes.
pub(super) const TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const TOAST_MAX_WIDTH: f32 = 320.0;
pub(super) const FADE_IN_MS: u64 = 150;
pub(super) const FADE_OUT_MS: u64 = 150;
pub(super) const PROGRESS_BAR_HEIGHT: f32 = 3.0;
/// ~30fps tick for the toast's full visible lifetime so the progress bar
/// shrinks smoothly and fades stay continuous.
pub(super) const ANIMATION_TICK_MS: u64 = 33;
/// Peak opacity: toasts stay slightly translucent so content behind them
/// remains subtly visible.
pub(super) const MAX_ALPHA: f32 = 0.95;
/// Top inset so the toast stack never rides under the 32px custom title bar.
pub(super) const OVERLAY_TOP_PAD: f32 = 48.0;
pub(super) const OVERLAY_SIDE_PAD: f32 = 16.0;

// ── Public API ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Toast {
    pub title: String,
    pub body: String,
    pub status: ToastStatus,
}

#[derive(Debug, Clone, Copy)]
pub enum ToastStatus {
    Info,
    Success,
    Warning,
    Error,
}

impl Toast {
    fn new(status: ToastStatus, body: impl Into<String>, title: Option<&str>) -> Self {
        Self {
            title: title.unwrap_or(status.title()).to_string(),
            body: body.into(),
            status,
        }
    }

    pub fn info(body: impl Into<String>, title: Option<&str>) -> Self {
        Self::new(ToastStatus::Info, body, title)
    }

    pub fn success(body: impl Into<String>, title: Option<&str>) -> Self {
        Self::new(ToastStatus::Success, body, title)
    }

    pub fn warning(body: impl Into<String>, title: Option<&str>) -> Self {
        Self::new(ToastStatus::Warning, body, title)
    }

    pub fn error(body: impl Into<String>, title: Option<&str>) -> Self {
        Self::new(ToastStatus::Error, body, title)
    }
}

impl ToastStatus {
    pub(super) fn background(self, colors: &AppColors) -> Color {
        match self {
            ToastStatus::Info => colors.toast_info_bg,
            ToastStatus::Success => colors.toast_success_bg,
            ToastStatus::Warning => colors.toast_warning_bg,
            ToastStatus::Error => colors.toast_error_bg,
        }
    }

    /// Lighter tint of the severity background for the progress bar fill.
    pub(super) fn progress_color(self, colors: &AppColors) -> Color {
        let bg = self.background(colors);
        // Linear-RGB lerp toward white at 60% — noticeably lighter on the
        // same hue.
        const T: f32 = 0.6;
        Color {
            r: bg.r + (1.0 - bg.r) * T,
            g: bg.g + (1.0 - bg.g) * T,
            b: bg.b + (1.0 - bg.b) * T,
            a: 1.0,
        }
    }

    pub(super) fn icon(self) -> char {
        match self {
            ToastStatus::Info => icons::INFO_CIRCLE_FILL.char(),
            ToastStatus::Success => icons::CHECK_CIRCLE_FILL.char(),
            ToastStatus::Warning => icons::EXCLAMATION_TRIANGLE_FILL.char(),
            ToastStatus::Error => icons::X_CIRCLE_FILL.char(),
        }
    }

    /// Default title when the caller passes `None`.
    fn title(self) -> &'static str {
        match self {
            ToastStatus::Info => "Info",
            ToastStatus::Success => "Success",
            ToastStatus::Warning => "Warning",
            ToastStatus::Error => "Error",
        }
    }
}
