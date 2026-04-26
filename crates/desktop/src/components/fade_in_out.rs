//! Open/close transition state for overlays (modals, sheets, dropdowns).
//!
//! Wraps a [`lilt::Animated<bool, Instant>`] so callers don't have to know
//! the lilt API surface. The wrapper handles the keep-alive-during-close
//! lifecycle: callers gate every `view()` on [`FadeInOut::progress_if_visible`]
//! and only render when it returns `Some(progress)`.
//!
//! Usage:
//! ```ignore
//! pub struct MyModal {
//!     fade: FadeInOut,
//! }
//!
//! // open / close
//! self.fade.open();
//! self.fade.close();
//!
//! // view: returns None while fully closed; the `?` short-circuits.
//! let progress = self.fade.progress_if_visible()?;
//! ```
//!
//! Every `open()` / `close()` calls [`crate::services::animation::extend`],
//! so the App-level frame subscription auto-lights without per-modal
//! wiring.
//!
//! When the underlying content needs to outlive the outro animation (e.g.
//! a bottom sheet that should still display its cipher while sliding away),
//! pair `close()` with a delayed cleanup: schedule a `Finalize*Close` message
//! via `tokio::time::sleep(animation_duration)` and clear the data only when
//! it arrives. Without that, the `view()` gate returns `Some(progress)` but
//! the content it would render is already gone.

use std::time::{Duration, Instant};

use lilt::{Animated, Easing};

use crate::services::animation;

const DEFAULT_DURATION_MS: f32 = 180.0;
const DEFAULT_DURATION: Duration = Duration::from_millis(DEFAULT_DURATION_MS as u64);

#[derive(Debug, Clone)]
pub struct FadeInOut {
    inner: Animated<bool, Instant>,
}

impl Default for FadeInOut {
    fn default() -> Self {
        Self {
            inner: Animated::new(false)
                .duration(DEFAULT_DURATION_MS)
                .easing(Easing::EaseOut),
        }
    }
}

impl FadeInOut {
    /// Start (or interrupt with) an opening transition.
    pub fn open(&mut self) {
        self.inner.transition(true, Instant::now());
        animation::extend(DEFAULT_DURATION);
    }

    /// Start (or interrupt with) a closing transition. The view keeps
    /// rendering until [`Self::progress_if_visible`] returns `None`.
    pub fn close(&mut self) {
        self.inner.transition(false, Instant::now());
        animation::extend(DEFAULT_DURATION);
    }

    /// The logical open/closed value — flips immediately on `open()` /
    /// `close()` (the animation runs against this target).
    pub fn is_open(&self) -> bool {
        self.inner.value
    }

    /// One-shot gate for the top of every `modal_view`: returns `None`
    /// when the overlay is fully closed (caller short-circuits via `?`)
    /// and `Some(progress)` (`0.0` → `1.0`) while it's still visible —
    /// either because the user has it open OR because an outro animation
    /// is still in flight after a close.
    pub fn progress_if_visible(&self) -> Option<f32> {
        let now = Instant::now();
        let visible = self.inner.value || self.inner.in_progress(now);
        visible.then(|| self.inner.animate_bool(0.0, 1.0, now))
    }
}
