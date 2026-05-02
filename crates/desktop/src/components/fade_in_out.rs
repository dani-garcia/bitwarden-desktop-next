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

use iced::{Task, widget};
use lilt::{Animated, Easing};

use crate::services::animation;

const DEFAULT_DURATION_MS: f32 = 180.0;
const DEFAULT_DURATION: Duration = Duration::from_millis(DEFAULT_DURATION_MS as u64);

/// Buffer past the animation duration before applying focus. Covers jitter
/// from frame timing — the slide-in animation can drag a few frames longer
/// than the nominal duration on a busy main thread.
const POST_OPEN_FOCUS_BUFFER: Duration = Duration::from_millis(40);

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

/// Returns a [`Task`] that focuses the widget with the given `id` after the
/// `FadeInOut` intro animation has settled. Pair with `fade.open()` to
/// auto-focus an input inside a freshly-opened overlay.
///
/// Why this exists: applying `widget::operation::focus(id)` while the fade
/// is still animating *appears* to focus the input — the cursor briefly
/// shows — but the focus state gets clobbered before any keystrokes can
/// land. The per-frame tree rebuild during the slide-in drops
/// `text_input::is_focused` partway through. Deferring past the animation
/// duration + a small jitter buffer lets focus stick.
///
/// # Example
///
/// ```ignore
/// LoginMessage::OpenSelfHosted => {
///     self.modal.fade.open();
///     return Outcome::task(fade_in_out::focus_after_open(URL_FIELD_ID));
/// }
/// ```
///
/// No intermediate message round-trip is needed — the returned task chains
/// the sleep with the focus operation directly.
pub fn focus_after_open<M: Send + 'static>(id: widget::Id) -> Task<M> {
    let delay = DEFAULT_DURATION + POST_OPEN_FOCUS_BUFFER;
    Task::perform(tokio::time::sleep(delay), |_| ())
        .then(move |_| iced::widget::operation::focus(id.clone()))
}
