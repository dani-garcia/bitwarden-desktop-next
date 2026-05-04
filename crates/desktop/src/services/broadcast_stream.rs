//! Iced-compatible adapter for `tokio::sync::broadcast` receivers.
//!
//! Push-callback services (muda menus, tray clicks, global hotkeys) park their
//! events in a `broadcast::Receiver` stashed in a `OnceLock` at startup; iced
//! consumes them via `Subscription::run(fn_pointer)`. The wiring is the same
//! every time — `resubscribe` so each subscriber gets a fresh queue, recv loop,
//! warn on `Lagged`, break on `Closed` — so this module owns it.
//!
//! Sources that may legitimately fail to install (e.g. global-hotkey on
//! Wayland) leave the `OnceLock` empty; the stream then terminates immediately,
//! leaving the subscription idle.

use std::sync::OnceLock;

use iced::futures::{SinkExt, Stream};
use tokio::sync::broadcast;

pub fn from_once_lock<T: Clone + Send + 'static>(
    source: &'static OnceLock<broadcast::Receiver<T>>,
    lag_label: &'static str,
) -> impl Stream<Item = T> {
    use iced::futures::channel::mpsc;
    iced::stream::channel(16, move |mut out: mpsc::Sender<_>| async move {
        let Some(rx) = source.get() else {
            return;
        };
        let mut rx = rx.resubscribe();
        loop {
            match rx.recv().await {
                Ok(item) => {
                    if out.send(item).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(dropped = n, %lag_label, "broadcast subscriber lagged");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}
