//! Global animation watermark — coordinates the App-level frame
//! subscription with whatever per-widget animation primitives exist.
//!
//! Every animation primitive ([`crate::components::FadeInOut`], future
//! slide / toast / spring helpers, ...) calls [`extend`] when it starts a
//! transition, declaring how long the transition will run. The App's
//! per-frame subscription gates on [`any_in_progress`] — a single global
//! read — so adding new animation types doesn't require any plumbing on
//! the App side.
//!
//! The contract is intentionally tiny:
//! - `extend(d)` at the start of every transition.
//! - `any_in_progress()` from the App subscription.
//!
//! Mutex is fine here — iced's update loop is single-threaded, and a few
//! nanoseconds of uncontended lock/unlock per `extend()` is below noise.
//! No coordination is needed between primitives because the watermark
//! always advances to the latest known end time (`max` semantics).

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

/// Latest `Instant` at which every running transition is known to have
/// settled. `None` means no animation has ever been registered (or the
/// last one is far in the past — we don't bother resetting).
static EARLIEST_QUIET: Mutex<Option<Instant>> = Mutex::new(None);

/// Extend the global "quiet at" watermark forward by `duration`. Called
/// at the start of every transition. Multiple concurrent transitions
/// from different primitives just take the max, no coordination needed.
pub fn extend(duration: Duration) {
    let target = Instant::now() + duration;
    let mut guard = EARLIEST_QUIET.lock().expect("EARLIEST_QUIET poisoned");
    *guard = Some(match *guard {
        Some(prev) if prev > target => prev,
        _ => target,
    });
}

/// True if at least one transition somewhere in the app might still be
/// running. Drives App's per-frame subscription so the app only wakes at
/// 60 Hz while something needs redraws.
pub fn any_in_progress() -> bool {
    let guard = EARLIEST_QUIET.lock().expect("EARLIEST_QUIET poisoned");
    match *guard {
        Some(quiet) => Instant::now() < quiet,
        None => false,
    }
}
