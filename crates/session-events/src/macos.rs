//! macOS lock/sleep events via `NSDistributedNotificationCenter` and
//! `NSWorkspace.notificationCenter`.
//!
//! `com.apple.screenIsLocked` / `com.apple.screenIsUnlocked` are undocumented
//! distributed notifications, but they have been stable since at least 10.7
//! and are used by mainstream apps (1Password, Slack, etc.).
//!
//! Each observer is registered with a dedicated `NSOperationQueue`, so the
//! observer blocks fire on that queue's worker thread — no run loop is
//! required on the registering thread (modern macOS delivers via XPC).
//!
//! On drop, every observer is explicitly removed from its center; the queue
//! is then released along with the rest of the struct.

use std::pin::Pin;
use std::ptr::NonNull;
use std::task::{Context, Poll};

use block2::RcBlock;
use futures_core::Stream;
use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
use objc2_app_kit::NSWorkspace;
use objc2_foundation::{
    NSDistributedNotificationCenter, NSNotification, NSNotificationCenter,
    NSOperationQueue, NSString,
};
use tokio::sync::mpsc;

use crate::SessionEvent;

type ObsHandle = Retained<ProtocolObject<dyn NSObjectProtocol>>;

pub(crate) fn platform_stream() -> impl Stream<Item = SessionEvent> + Send + 'static {
    let (tx, rx) = mpsc::channel::<SessionEvent>(16);

    let queue = NSOperationQueue::new();
    let dist_center = NSDistributedNotificationCenter::defaultCenter();
    let workspace_center = NSWorkspace::sharedWorkspace().notificationCenter();

    let dist_observers = [
        register(
            &dist_center,
            &queue,
            "com.apple.screenIsLocked",
            tx.clone(),
            SessionEvent::Locked,
        ),
        register(
            &dist_center,
            &queue,
            "com.apple.screenIsUnlocked",
            tx.clone(),
            SessionEvent::Unlocked,
        ),
    ];
    let workspace_observers = [
        register(
            &workspace_center,
            &queue,
            "NSWorkspaceWillSleepNotification",
            tx.clone(),
            SessionEvent::Suspended,
        ),
        register(
            &workspace_center,
            &queue,
            "NSWorkspaceDidWakeNotification",
            tx,
            SessionEvent::Resumed,
        ),
    ];

    MacosStream {
        rx,
        dist_center,
        workspace_center,
        dist_observers,
        workspace_observers,
        _queue: queue,
    }
}

struct MacosStream {
    rx: mpsc::Receiver<SessionEvent>,
    dist_center: Retained<NSDistributedNotificationCenter>,
    workspace_center: Retained<NSNotificationCenter>,
    dist_observers: [ObsHandle; 2],
    workspace_observers: [ObsHandle; 2],
    _queue: Retained<NSOperationQueue>,
}

// SAFETY: `Retained<ProtocolObject<dyn NSObjectProtocol>>` is not auto-`Send`
// because `dyn NSObjectProtocol` lacks `Send + Sync` bounds, but the underlying
// ObjC objects (notification observer tokens) are documented thread-safe to
// release. The other fields are Send already.
unsafe impl Send for MacosStream {}

impl Stream for MacosStream {
    type Item = SessionEvent;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

impl Drop for MacosStream {
    fn drop(&mut self) {
        for obs in &self.dist_observers {
            // SAFETY: obs is a valid token previously returned by dist_center.
            unsafe {
                self.dist_center
                    .removeObserver_name_object(obs.as_ref(), None, None)
            };
        }
        for obs in &self.workspace_observers {
            // SAFETY: obs is a valid token previously returned by workspace_center.
            unsafe {
                self.workspace_center
                    .removeObserver_name_object(obs.as_ref(), None, None)
            };
        }
    }
}

fn register(
    center: &NSNotificationCenter,
    queue: &NSOperationQueue,
    name: &str,
    tx: mpsc::Sender<SessionEvent>,
    event: SessionEvent,
) -> ObsHandle {
    let name = NSString::from_str(name);
    let block = RcBlock::new(move |_: NonNull<NSNotification>| {
        // try_send drops events if the consumer is slow or already gone — these
        // are state changes, not commands, so dropping is the right behavior.
        let _ = tx.try_send(event);
    });
    // SAFETY: block has the right `Fn(NonNull<NSNotification>)` signature; the
    // queue keeps blocks off the registering thread so no run loop is needed.
    unsafe {
        center.addObserverForName_object_queue_usingBlock(Some(&name), None, Some(queue), &block)
    }
}
