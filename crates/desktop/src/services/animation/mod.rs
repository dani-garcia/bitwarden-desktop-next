//! Global animation watermark — coordinates the App-level frame
//! subscription with per-widget animation primitives.
//!
//! Every animation primitive calls [`extend`] when it starts a transition;
//! the App's per-frame subscription gates on [`any_in_progress`] so adding
//! new animation types needs no App-side plumbing.
//!
//! ## Ownership
//!
//! State lives on [`AnimationWatermark`], which `App` owns as
//! `Arc<AnimationWatermark>`. A module-level `Weak<AnimationWatermark>`
//! registry lets the free [`extend`] / [`any_in_progress`] functions reach
//! the live watermark without each animation primitive needing the App's
//! reference threaded through it. When the App drops, the `Weak` upgrade
//! fails and the free functions become no-ops — so a second App
//! constructed in tests doesn't fight over leftover state from the first.
//!
//! Mutex on the inner `Option<Instant>` is fine: iced's update loop is
//! single-threaded, and the watermark always advances to the latest known
//! end time (`max` semantics) so primitives need no coordination.

use std::{
    sync::{Arc, Mutex, Weak},
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct AnimationWatermark {
    /// Latest `Instant` at which every running transition is known to have
    /// settled. `None` means no animation has ever been registered against
    /// this watermark.
    quiet: Mutex<Option<Instant>>,
}

impl AnimationWatermark {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Extend the "quiet at" watermark forward by `duration`.
    pub fn extend(&self, duration: Duration) {
        let target = Instant::now() + duration;
        let mut guard = self.quiet.lock().expect("AnimationWatermark poisoned");
        *guard = Some(match *guard {
            Some(prev) if prev > target => prev,
            _ => target,
        });
    }

    /// True if at least one transition might still be running.
    pub fn any_in_progress(&self) -> bool {
        let guard = self.quiet.lock().expect("AnimationWatermark poisoned");
        match *guard {
            Some(quiet) => Instant::now() < quiet,
            None => false,
        }
    }
}

/// Registry slot. `Weak::new()` has been const since Rust 1.73, so the
/// static can be initialised without `OnceLock` ceremony. Replacing the
/// Weak (e.g. across test re-constructions of `App`) is fine because the
/// previous strong Arc has been dropped by then.
static REGISTERED: Mutex<Weak<AnimationWatermark>> = Mutex::new(Weak::new());

/// Install `watermark` as the active target for the free functions below.
/// Call once from `App::new`. Subsequent calls (e.g. test scenarios that
/// build a fresh App) replace the Weak; the previous Arc has already been
/// dropped with the previous App.
pub fn register(watermark: &Arc<AnimationWatermark>) {
    *REGISTERED.lock().expect("animation registry poisoned") = Arc::downgrade(watermark);
}

/// Extend the registered watermark by `duration`. No-op when no App is
/// alive (e.g. unit tests of an animation primitive that doesn't care
/// about the App-level frame subscription).
///
/// This free function is the single ergonomic hook for animation primitives
/// — they can call `animation::extend(...)` from anywhere (widget `diff`,
/// `update` handlers, etc.) without threading a watermark reference. The
/// dual `any_in_progress` query is intentionally not exposed here because
/// the only consumer (`App::subscription`) already has direct access to its
/// owned `Arc<AnimationWatermark>`.
pub fn extend(duration: Duration) {
    if let Some(w) = REGISTERED
        .lock()
        .expect("animation registry poisoned")
        .upgrade()
    {
        w.extend(duration);
    }
}
