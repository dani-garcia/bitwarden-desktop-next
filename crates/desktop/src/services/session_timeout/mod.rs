//! Per-user session timeout enforcement.
//!
//! Tracks `last_activity[uid]: Instant` for every signed-in user and fires a
//! single broadcast tick when the soonest deadline elapses. The driver is a
//! tokio task that sleeps until the next deadline (no fixed-interval polling)
//! and wakes early when activity / settings / login state push the deadline
//! forward via a [`tokio::sync::watch`] channel.
//!
//! ## Why `Instant`
//!
//! [`std::time::Instant`] is the project's monotonic-time pattern (see
//! `services/animation`). Backed by `CLOCK_MONOTONIC` / `mach_absolute_time` /
//! `QueryPerformanceCounter` — immune to wall-clock manipulation. All elapsed
//! math goes through [`Instant::saturating_duration_since`] so any platform
//! corner-case non-monotonicity clamps to zero rather than wrapping.
//!
//! `SystemTime` is read in exactly one place: at [`SessionTimeout::note_resume`],
//! to compute how long the system was suspended on platforms (Linux, macOS)
//! where `Instant` pauses during sleep. The resulting delta only ever pushes
//! `last_activity` *backward* (never forward), so a clock-advance attack
//! during suspend can only cause the vault to lock more aggressively, never
//! less.
//!
//! ## Ownership
//!
//! The state, watch channel, and driver task all live on
//! [`SessionTimeout`]. `App` owns one instance; tests construct fresh
//! instances per case (no shared `TEST_LOCK` needed). The driver task's
//! `JoinHandle` is aborted on drop so a dropped instance cleans up
//! immediately.
//!
//! ## API surface
//!
//! - [`SessionTimeout::new`] — spawn the driver and return the handle.
//! - [`SessionTimeout::record_activity`] — bumps `last_activity[uid]` to now.
//!   Doubles as the enroll path; first call for a uid creates the entry.
//!   250 ms throttle on watch-channel pushes.
//! - [`SessionTimeout::unenroll`] — drop tracking on full logout (lock keeps
//!   the user enrolled because `logout_after` keeps ticking).
//! - [`SessionTimeout::note_suspend`] / [`SessionTimeout::note_resume`] —
//!   wall-clock + monotonic snapshot pair. `note_resume` reconciles
//!   `last_activity` with the suspend gap.
//! - [`SessionTimeout::recompute_and_push_deadline`] — recompute the soonest
//!   deadline and push it to the driver task.
//! - [`SessionTimeout::expired`] — pure check: what action (if any) should
//!   fire for one user right now.
//! - [`SessionTimeout::tick_sender`] — clone for `App::subscription` to feed
//!   into [`broadcast_stream::subscription_from_sender`].

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant, SystemTime},
};

use bitwarden_core::UserId;
use tokio::{
    sync::{broadcast, watch},
    task::JoinHandle,
};

use crate::services::preferences::UserPreferences;

/// Bitwarden Electron's input throttle ([`app.component.ts:753`]). Coalesces
/// rapid keystrokes / clicks so the watch channel only fires when the
/// extension to the deadline is meaningful.
const ACTIVITY_THROTTLE: Duration = Duration::from_millis(250);

/// What the timeout dictates for a given user once it expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Lock,
    Logout,
}

/// Per-user state. The map is the source of truth for "when did this user
/// last interact"; `last_throttle_push` is a separate watermark so the
/// throttle doesn't suppress activity recording, only watch-channel pushes.
#[derive(Default)]
struct State {
    last_activity: HashMap<UserId, Instant>,
    /// Last time we pushed a watch-channel update on behalf of `uid`; gates
    /// the 250 ms throttle on `record_activity`.
    last_throttle_push: HashMap<UserId, Instant>,
    /// Set on `note_suspend`, consumed on `note_resume`. `None` outside a
    /// suspend window.
    suspend_snapshot: Option<(Instant, SystemTime)>,
}

/// Owned by `App`. Drop the value to abort the driver task; new instances
/// (e.g. across tests) are independent and don't share state.
pub struct SessionTimeout {
    state: Mutex<State>,
    /// Driver task reads this for the next deadline; the App pushes new
    /// values whenever something shifts the soonest expiry.
    deadline_tx: watch::Sender<Option<Instant>>,
    /// Driver task pushes `()` here when the current deadline elapses.
    /// `App::subscription` clones this and consumes via
    /// [`crate::services::broadcast_stream::subscription_from_sender`] —
    /// each call uses [`broadcast::Sender::subscribe`] for a fresh
    /// receiver.
    tick_tx: broadcast::Sender<()>,
    /// Aborted on drop so the task doesn't outlive its sender/receiver.
    driver: JoinHandle<()>,
}

impl SessionTimeout {
    /// Spawn the driver in iced's tokio runtime. Idle (parked on
    /// `pending()`) until the first user enrolls.
    pub fn new() -> Self {
        let (deadline_tx, deadline_rx) = watch::channel::<Option<Instant>>(None);
        let (tick_tx, _) = broadcast::channel::<()>(1);
        let driver = tokio::spawn(driver(deadline_rx, tick_tx.clone()));
        Self {
            state: Mutex::default(),
            deadline_tx,
            tick_tx,
            driver,
        }
    }

    /// Sender clone for `App::subscription` to feed into
    /// [`crate::services::broadcast_stream::subscription_from_sender`].
    pub fn tick_sender(&self) -> broadcast::Sender<()> {
        self.tick_tx.clone()
    }

    /// Record an input event. Bumps `last_activity[uid]` to now, subject
    /// to a 250 ms throttle on subsequent watch-channel pushes. Returns
    /// `true` when the deadline was actually pushed (i.e. the throttle
    /// didn't suppress it), so the caller can decide whether to recompute
    /// the soonest deadline.
    pub fn record_activity(&self, uid: UserId) -> bool {
        let now = Instant::now();
        let mut s = self.state.lock().expect("session_timeout state poisoned");
        s.last_activity.insert(uid, now);
        let last_push = s.last_throttle_push.get(&uid).copied();
        match last_push {
            Some(prev) if now.saturating_duration_since(prev) < ACTIVITY_THROTTLE => false,
            _ => {
                s.last_throttle_push.insert(uid, now);
                true
            }
        }
    }

    /// Drop the user's tracking entries. Called when the user fully logs
    /// out (a lock event keeps the user enrolled because `logout_after`
    /// may still fire on the locked vault).
    pub fn unenroll(&self, uid: &UserId) {
        let mut s = self.state.lock().expect("session_timeout state poisoned");
        s.last_activity.remove(uid);
        s.last_throttle_push.remove(uid);
    }

    /// Snapshot wall-clock + monotonic at the moment the OS reports a
    /// suspend. Replaces any prior snapshot.
    pub fn note_suspend(&self) {
        let mut s = self.state.lock().expect("session_timeout state poisoned");
        s.suspend_snapshot = Some((Instant::now(), SystemTime::now()));
    }

    /// Reconcile `last_activity` with the suspend duration so the next
    /// deadline check sees the correct elapsed time.
    ///
    /// On Windows, `Instant` keeps ticking during sleep, so the gap between
    /// wall-clock and monotonic is zero — no work to do. On Linux/macOS,
    /// `Instant` pauses; the gap captures exactly the time `CLOCK_MONOTONIC`
    /// missed. We subtract that gap from every enrolled user's
    /// `last_activity`, so `Instant::now() - last_activity` post-resume
    /// equals what it would have been if the monotonic clock had ticked
    /// through the suspend.
    ///
    /// Wall-clock is consulted only for this delta, never for an absolute
    /// deadline — a clock-rewind attack during suspend produces
    /// `wall_delta = 0` (`SystemTime::duration_since` returns `Err`), which
    /// collapses the gap to zero and leaves the timer behaving as if there
    /// were no suspend. That's the same as the pre-resume state, never
    /// weaker.
    ///
    /// `Instant::checked_sub` underflow leaves the entry untouched — at
    /// that point the entry is already older than any realistic timeout, so
    /// the immediately-following timeout check will lock anyway.
    pub fn note_resume(&self) {
        let mut s = self.state.lock().expect("session_timeout state poisoned");
        let Some((suspend_instant, suspend_wall)) = s.suspend_snapshot.take() else {
            return;
        };
        let instant_delta = Instant::now().saturating_duration_since(suspend_instant);
        let wall_delta = SystemTime::now()
            .duration_since(suspend_wall)
            .unwrap_or(Duration::ZERO);
        let gap = wall_delta.saturating_sub(instant_delta);
        if gap.is_zero() {
            return;
        }
        for at in s.last_activity.values_mut() {
            if let Some(new) = at.checked_sub(gap) {
                *at = new;
            }
        }
    }

    /// Compute the soonest deadline across every signed-in user and push
    /// it to the driver task. The active+focused user's `lock_after` is
    /// excluded from the deadline (defensive grace — the user is looking
    /// at the screen); their `logout_after` still counts.
    ///
    /// Without that exclusion, an active+focused user whose `lock_after`
    /// had just elapsed would hot-spin: the driver fires, `expired`
    /// returns `None` (grace), the App recomputes the same deadline
    /// (already past), `sleep_until` returns immediately, repeat.
    pub fn recompute_and_push_deadline(
        &self,
        snapshots: &[UserSnapshot],
        active_uid: Option<&UserId>,
        focused: bool,
    ) {
        let next = self.compute_next_deadline(snapshots, active_uid, focused);
        let _ = self.deadline_tx.send(next);
    }

    fn compute_next_deadline(
        &self,
        snapshots: &[UserSnapshot],
        active_uid: Option<&UserId>,
        focused: bool,
    ) -> Option<Instant> {
        let s = self.state.lock().expect("session_timeout state poisoned");
        let mut next: Option<Instant> = None;
        for snap in snapshots {
            let Some(&last) = s.last_activity.get(&snap.uid) else {
                continue;
            };
            let active_focused = active_uid == Some(&snap.uid) && focused;
            for d in deadlines_for(snap, last, active_focused) {
                next = Some(match next {
                    Some(prev) => prev.min(d),
                    None => d,
                });
            }
        }
        next
    }

    /// Partition expired users into the three buckets the App applies
    /// after a tick: non-active logouts (processed inline so the
    /// "next active user" hand-off only runs once), an active-user
    /// logout flag (drives `handle_log_out`), and locks.
    ///
    /// Splitting the active-user logout into its own boolean — rather
    /// than letting it land in `inline_logouts` — prevents the bug
    /// the comment in `App::run_session_timeout_check` used to guard
    /// against: if `handle_log_out` runs first, it switches to the
    /// "next" user, and that user might also be in the logout list,
    /// causing the loop to inline-clean a now-active user.
    pub fn plan_timeout_actions(
        &self,
        snapshots: &[UserSnapshot],
        active: Option<&UserId>,
        focused: bool,
    ) -> TimeoutPlan {
        let mut plan = TimeoutPlan::default();
        for snap in snapshots {
            match self.expired(snap, active, focused) {
                Some(Action::Logout) => {
                    if active == Some(&snap.uid) {
                        plan.handle_active_logout = true;
                    } else {
                        plan.inline_logouts.push(snap.uid);
                    }
                }
                Some(Action::Lock) => plan.locks.push(snap.uid),
                None => {}
            }
        }
        plan
    }

    /// Decide what (if anything) should fire for `snap` right now.
    ///
    /// - `Logout` wins over `Lock` (it's strictly more aggressive).
    /// - The active+focused user is exempt from `Lock` only — `Logout`
    ///   still fires (parity with Bitwarden, where the focus grace
    ///   short-circuits `shouldLock` but the action is the same channel).
    pub fn expired(
        &self,
        snap: &UserSnapshot,
        active_uid: Option<&UserId>,
        focused: bool,
    ) -> Option<Action> {
        let last = {
            let s = self.state.lock().expect("session_timeout state poisoned");
            *s.last_activity.get(&snap.uid)?
        };

        let elapsed = Instant::now().saturating_duration_since(last);
        let active_focused = active_uid == Some(&snap.uid) && focused;

        if let Some(d) = snap.prefs.logout_after.as_duration()
            && elapsed >= d
        {
            return Some(Action::Logout);
        }
        if snap.is_unlocked
            && !active_focused
            && let Some(d) = snap.prefs.lock_after.as_duration()
            && elapsed >= d
        {
            return Some(Action::Lock);
        }
        None
    }
}

impl Drop for SessionTimeout {
    fn drop(&mut self) {
        self.driver.abort();
    }
}

async fn driver(mut rx: watch::Receiver<Option<Instant>>, tick: broadcast::Sender<()>) {
    loop {
        let deadline = *rx.borrow_and_update();
        tokio::select! {
            _ = sleep_until_or_pending(deadline) => {
                let _ = tick.send(());
                // Block until the App processes the tick and pushes a new
                // deadline; otherwise we'd hot-loop on a deadline already
                // in the past.
                let _ = rx.changed().await;
            }
            _ = rx.changed() => {}
        }
    }
}

async fn sleep_until_or_pending(deadline: Option<Instant>) {
    match deadline {
        Some(at) => tokio::time::sleep_until(at.into()).await,
        // No enrolled deadline; park until `changed()` fires on the other arm.
        None => std::future::pending::<()>().await,
    }
}

/// Per-user input to deadline computation and expiry checks. The
/// `is_unlocked` flag affects which timer is relevant: a locked user can't
/// be locked again (so `lock_after` is moot for them) but their
/// `logout_after` keeps ticking.
#[derive(Debug, Clone, Copy)]
pub struct UserSnapshot {
    pub uid: UserId,
    pub prefs: UserPreferences,
    pub is_unlocked: bool,
}

/// Output of [`SessionTimeout::plan_timeout_actions`]. App processes the
/// plan in-order: inline logouts → active-user logout (if any) → locks.
#[derive(Default, Debug, PartialEq, Eq)]
pub struct TimeoutPlan {
    /// Non-active users to log out before any user-switch happens.
    pub inline_logouts: Vec<UserId>,
    /// Whether the active user expired-for-Logout. App should call
    /// `handle_log_out` (which transitions to the next user) when set.
    pub handle_active_logout: bool,
    /// Users (active or not) that should be locked.
    pub locks: Vec<UserId>,
}

/// Yields candidate deadlines (lock + logout, in order, skipping NEVER).
/// Skips the `lock_after` candidate when `active_focused` holds, matching
/// the same exemption applied in `expired`.
fn deadlines_for(
    snap: &UserSnapshot,
    last: Instant,
    active_focused: bool,
) -> impl IntoIterator<Item = Instant> {
    let lock = (snap.is_unlocked && !active_focused)
        .then(|| snap.prefs.lock_after.as_duration())
        .flatten()
        .and_then(|d| last.checked_add(d));
    let logout = snap
        .prefs
        .logout_after
        .as_duration()
        .and_then(|d| last.checked_add(d));
    [lock, logout].into_iter().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitwarden_core::UserId;

    use crate::services::preferences::DurationSecs;

    fn uid(s: &str) -> UserId {
        // UserId is a uuid_newtype; pad/format to a real uuid string.
        let padded = format!("{:0>32}", s);
        let formatted = format!(
            "{}-{}-{}-{}-{}",
            &padded[0..8],
            &padded[8..12],
            &padded[12..16],
            &padded[16..20],
            &padded[20..32]
        );
        formatted.parse().expect("valid uuid")
    }

    fn prefs(lock: u32, logout: u32) -> UserPreferences {
        UserPreferences {
            lock_after: DurationSecs(lock),
            logout_after: DurationSecs(logout),
            ..UserPreferences::default()
        }
    }

    /// Each test gets a fresh `SessionTimeout`. Driver task is spawned
    /// inside a tokio runtime; tests that don't drive it just exercise the
    /// pure logic and let `Drop` abort the spawn.
    fn fresh() -> SessionTimeout {
        SessionTimeout::new()
    }

    #[tokio::test]
    async fn compute_next_deadline_picks_min() {
        let st = fresh();
        let a = uid("a");
        let b = uid("b");
        st.record_activity(a);
        st.record_activity(b);
        let snaps = [
            UserSnapshot {
                uid: a,
                prefs: prefs(60, 3600),
                is_unlocked: true,
            },
            UserSnapshot {
                uid: b,
                prefs: prefs(900, 0),
                is_unlocked: true,
            },
        ];
        let now = Instant::now();
        let next = st
            .compute_next_deadline(&snaps, None, false)
            .expect("at least one deadline");
        // a's lock_after = 60s is the soonest.
        let delta = next.saturating_duration_since(now);
        assert!(delta <= Duration::from_secs(60));
        assert!(delta >= Duration::from_secs(59));
    }

    #[tokio::test]
    async fn compute_next_deadline_skips_never_pair() {
        let st = fresh();
        let a = uid("a");
        st.record_activity(a);
        let snaps = [UserSnapshot {
            uid: a,
            prefs: prefs(0, 0),
            is_unlocked: true,
        }];
        assert!(st.compute_next_deadline(&snaps, None, false).is_none());
    }

    #[tokio::test]
    async fn compute_next_deadline_locked_user_skips_lock_timer() {
        let st = fresh();
        let a = uid("a");
        st.record_activity(a);
        let snaps = [UserSnapshot {
            uid: a,
            prefs: prefs(60, 0), // lock_after = 60s, logout = NEVER
            is_unlocked: false,  // locked
        }];
        // No logout deadline + locked user means no deadline at all.
        assert!(st.compute_next_deadline(&snaps, None, false).is_none());
    }

    #[tokio::test]
    async fn compute_next_deadline_active_focused_skips_lock_after() {
        let st = fresh();
        let a = uid("a");
        st.record_activity(a);
        let snaps = [UserSnapshot {
            uid: a,
            prefs: prefs(60, 0),
            is_unlocked: true,
        }];
        // Logout NEVER + lock_after exempted via grace ⇒ no deadline.
        assert!(st.compute_next_deadline(&snaps, Some(&a), true).is_none());
        // Same user, unfocused: lock_after re-enters the deadline.
        assert!(st.compute_next_deadline(&snaps, Some(&a), false).is_some());
    }

    #[tokio::test]
    async fn record_activity_throttles_within_250ms() {
        let st = fresh();
        let a = uid("a");
        // First call enrols (no prior throttle entry); we don't assert on
        // its return — its job here is to seed the throttle watermark.
        st.record_activity(a);
        tokio::time::sleep(Duration::from_millis(50)).await;
        let pushed = st.record_activity(a);
        assert!(!pushed, "throttled at 50 ms");
        // Generous post-throttle margin so a slow scheduler can't flake the
        // assertion in the other direction.
        tokio::time::sleep(Duration::from_millis(500)).await;
        let pushed = st.record_activity(a);
        assert!(pushed, "throttle clears past 250 ms");
    }

    #[tokio::test]
    async fn note_resume_back_dates_by_gap() {
        let st = fresh();
        let a = uid("a");
        st.record_activity(a);
        let before = st.state.lock().unwrap().last_activity[&a];

        // Forge a suspend snapshot 60 s in the past on the wall clock and
        // ~now on the monotonic clock. `note_resume` should compute a 60 s
        // gap and back-date every entry by that.
        {
            let mut s = st.state.lock().unwrap();
            s.suspend_snapshot =
                Some((Instant::now(), SystemTime::now() - Duration::from_secs(60)));
        }
        st.note_resume();

        let after = st.state.lock().unwrap().last_activity[&a];
        let backdated = before.saturating_duration_since(after);
        // Allow a small tolerance for the time spent in the test itself.
        assert!(
            (Duration::from_secs(58)..=Duration::from_secs(62)).contains(&backdated),
            "expected ~60 s back-date, got {backdated:?}",
        );
    }

    #[tokio::test]
    async fn note_resume_no_op_without_snapshot() {
        let st = fresh();
        let a = uid("a");
        st.record_activity(a);
        let before = st.state.lock().unwrap().last_activity[&a];
        // No `note_suspend` call → no snapshot → no-op.
        st.note_resume();
        let after = st.state.lock().unwrap().last_activity[&a];
        assert_eq!(before, after);
    }

    // ── plan_timeout_actions ──────────────────────────────────────────────

    /// Force a user's `last_activity` to the past so `expired` fires now.
    fn backdate(st: &SessionTimeout, user: UserId, ago: Duration) {
        let mut s = st.state.lock().unwrap();
        s.last_activity
            .insert(user, Instant::now().checked_sub(ago).unwrap());
    }

    #[tokio::test]
    async fn plan_empty_when_no_users() {
        let st = fresh();
        assert_eq!(
            st.plan_timeout_actions(&[], None, false),
            TimeoutPlan::default()
        );
    }

    #[tokio::test]
    async fn plan_routes_active_logout_to_handle_flag_not_inline() {
        // The whole reason this method exists: when the active user
        // expires for Logout, it must NOT land in `inline_logouts` —
        // otherwise the App's iteration would log out a user whose
        // hand-off to the next active hasn't run yet.
        let st = fresh();
        let a = uid("a");
        st.record_activity(a);
        backdate(&st, a, Duration::from_secs(120));

        let snaps = [UserSnapshot {
            uid: a,
            prefs: prefs(0, 60), // logout = 60s, lock = NEVER
            is_unlocked: true,
        }];

        let plan = st.plan_timeout_actions(&snaps, Some(&a), false);
        assert!(plan.handle_active_logout);
        assert!(plan.inline_logouts.is_empty());
        assert!(plan.locks.is_empty());
    }

    #[tokio::test]
    async fn plan_inlines_non_active_logouts() {
        let st = fresh();
        let a = uid("a");
        let b = uid("b");
        let c = uid("c");
        st.record_activity(a);
        st.record_activity(b);
        st.record_activity(c);
        // a is active and fresh; b and c expired-for-Logout.
        backdate(&st, b, Duration::from_secs(120));
        backdate(&st, c, Duration::from_secs(120));

        let snaps = [
            UserSnapshot {
                uid: a,
                prefs: prefs(0, 0),
                is_unlocked: true,
            },
            UserSnapshot {
                uid: b,
                prefs: prefs(0, 60),
                is_unlocked: true,
            },
            UserSnapshot {
                uid: c,
                prefs: prefs(0, 60),
                is_unlocked: true,
            },
        ];

        let plan = st.plan_timeout_actions(&snaps, Some(&a), false);
        assert!(!plan.handle_active_logout);
        assert_eq!(plan.inline_logouts, vec![b, c]);
        assert!(plan.locks.is_empty());
    }

    #[tokio::test]
    async fn plan_collects_locks_separately_from_logouts() {
        let st = fresh();
        let a = uid("a");
        let b = uid("b");
        st.record_activity(a);
        st.record_activity(b);
        backdate(&st, a, Duration::from_secs(120));
        backdate(&st, b, Duration::from_secs(120));

        let snaps = [
            // a: only `lock_after` set, expired.
            UserSnapshot {
                uid: a,
                prefs: prefs(60, 0),
                is_unlocked: true,
            },
            // b: `logout_after` set, expired.
            UserSnapshot {
                uid: b,
                prefs: prefs(0, 60),
                is_unlocked: true,
            },
        ];

        let plan = st.plan_timeout_actions(&snaps, None, false);
        assert!(!plan.handle_active_logout);
        assert_eq!(plan.locks, vec![a]);
        assert_eq!(plan.inline_logouts, vec![b]);
    }

    #[tokio::test]
    async fn plan_respects_active_focused_lock_grace() {
        let st = fresh();
        let a = uid("a");
        st.record_activity(a);
        backdate(&st, a, Duration::from_secs(120));
        let snaps = [UserSnapshot {
            uid: a,
            prefs: prefs(60, 0), // lock_after expired, no logout
            is_unlocked: true,
        }];

        // Active+focused → lock is suppressed.
        let plan = st.plan_timeout_actions(&snaps, Some(&a), true);
        assert!(plan.locks.is_empty());
        assert!(!plan.handle_active_logout);
        assert!(plan.inline_logouts.is_empty());

        // Same user, unfocused → lock fires.
        let plan = st.plan_timeout_actions(&snaps, Some(&a), false);
        assert_eq!(plan.locks, vec![a]);
    }

    #[tokio::test]
    async fn plan_logout_wins_over_lock_for_same_user() {
        // `expired` returns Logout when both timers have fired; the
        // planner must surface that — never duplicate the user across
        // both buckets.
        let st = fresh();
        let a = uid("a");
        st.record_activity(a);
        backdate(&st, a, Duration::from_secs(120));
        let snaps = [UserSnapshot {
            uid: a,
            prefs: prefs(30, 60), // both fired (120s elapsed > each)
            is_unlocked: true,
        }];

        let plan = st.plan_timeout_actions(&snaps, None, false);
        assert!(plan.locks.is_empty());
        assert_eq!(plan.inline_logouts, vec![a]);
    }
}
