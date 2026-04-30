//! Linux lock/sleep events via systemd-logind D-Bus signals.
//!
//! - `Lock` / `Unlock` on the current session (resolved via
//!   `GetSessionByPID(self)`).
//! - `PrepareForSleep(start: bool)` on the manager — `true` means the system
//!   is about to suspend, `false` means it has resumed.
//!
//! On systems without logind (rare in 2026) the connection setup fails and
//! the stream stays silent.

use std::pin::Pin;
use std::process;
use std::task::{Context, Poll};

use futures_core::Stream;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use zbus::proxy;
use zbus::zvariant::OwnedObjectPath;

use crate::SessionEvent;

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait LogindManager {
    #[zbus(name = "GetSessionByPID")]
    fn get_session_by_pid(&self, pid: u32) -> zbus::Result<OwnedObjectPath>;

    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1"
)]
trait LogindSession {
    #[zbus(signal)]
    fn lock(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn unlock(&self) -> zbus::Result<()>;
}

pub(crate) fn platform_stream() -> impl Stream<Item = SessionEvent> + Send + 'static {
    let (tx, rx) = mpsc::channel::<SessionEvent>(16);

    let handle = tokio::spawn(async move {
        if let Err(e) = run_listener(tx).await {
            tracing::debug!(error = %e, "session-events: logind listener exited");
        }
    });

    LinuxStream {
        rx,
        handle: Some(handle),
    }
}

struct LinuxStream {
    rx: mpsc::Receiver<SessionEvent>,
    handle: Option<JoinHandle<()>>,
}

impl Stream for LinuxStream {
    type Item = SessionEvent;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

impl Drop for LinuxStream {
    fn drop(&mut self) {
        if let Some(h) = self.handle.take() {
            h.abort();
        }
    }
}

async fn run_listener(tx: mpsc::Sender<SessionEvent>) -> zbus::Result<()> {
    let conn = zbus::Connection::system().await?;
    let manager = LogindManagerProxy::new(&conn).await?;
    let session_path = manager.get_session_by_pid(process::id()).await?;
    let session = LogindSessionProxy::builder(&conn)
        .path(session_path)?
        .build()
        .await?;

    let mut sleep = manager.receive_prepare_for_sleep().await?;
    let mut lock = session.receive_lock().await?;
    let mut unlock = session.receive_unlock().await?;

    loop {
        tokio::select! {
            biased;
            Some(signal) = sleep.next() => {
                match signal.args() {
                    Ok(args) => {
                        let ev = if args.start { SessionEvent::Suspended } else { SessionEvent::Resumed };
                        if tx.send(ev).await.is_err() { return Ok(()); }
                    }
                    Err(e) => tracing::debug!(error = %e, "PrepareForSleep args parse failed"),
                }
            }
            Some(_) = lock.next() => {
                if tx.send(SessionEvent::Locked).await.is_err() { return Ok(()); }
            }
            Some(_) = unlock.next() => {
                if tx.send(SessionEvent::Unlocked).await.is_err() { return Ok(()); }
            }
            else => return Ok(()), // all streams ended
        }
    }
}
