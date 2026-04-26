//! Global animation watermark — coordinates the App-level frame
//! subscription with per-widget animation primitives.
//!
//! Every animation primitive calls [`extend`] when it starts a transition;
//! the App's per-frame subscription gates on [`any_in_progress`] so adding
//! new animation types needs no App-side plumbing.
//!
//! Mutex is fine here — iced's update loop is single-threaded, and the
//! watermark always advances to the latest known end time (`max` semantics)
//! so primitives need no coordination.

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

/// Latest `Instant` at which every running transition is known to have
/// settled. `None` means no animation has ever been registered.
static EARLIEST_QUIET: Mutex<Option<Instant>> = Mutex::new(None);

/// Extend the global "quiet at" watermark forward by `duration`. Called
/// at the start of every transition.
pub fn extend(duration: Duration) {
    let target = Instant::now() + duration;
    let mut guard = EARLIEST_QUIET.lock().expect("EARLIEST_QUIET poisoned");
    *guard = Some(match *guard {
        Some(prev) if prev > target => prev,
        _ => target,
    });
}

/// True if at least one transition somewhere in the app might still be
/// running. Drives App's per-frame subscription.
pub fn any_in_progress() -> bool {
    let guard = EARLIEST_QUIET.lock().expect("EARLIEST_QUIET poisoned");
    match *guard {
        Some(quiet) => Instant::now() < quiet,
        None => false,
    }
}
