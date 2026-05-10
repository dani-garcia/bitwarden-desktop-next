//! Iced-compatible adapters for `tokio::sync::broadcast` senders and receivers.
//!
//! Two flavors share the same recv loop:
//!
//! - [`from_once_lock`] — receiver lives in a `OnceLock` because the source is
//!   a process-singleton FFI bridge (muda menus, tray clicks, global hotkeys)
//!   whose `set_event_handler` callback is itself process-static. The receiver
//!   has nowhere else to live.
//! - [`subscription_from_sender`] — sender lives on an App-owned service. Each
//!   subscription run calls `sender.subscribe()` for a fresh receiver, so
//!   construction and teardown follow the App's lifetime instead of process
//!   lifetime. Use this for any new bridge.
//!
//! Sources that may legitimately fail to install (e.g. global-hotkey on
//! Wayland) leave the `OnceLock` empty; the stream then terminates immediately,
//! leaving the subscription idle.

use std::{hash::Hash, sync::OnceLock};

use iced::{
    Subscription,
    advanced::subscription::{EventStream, Hasher, Recipe, from_recipe},
    futures::{
        SinkExt, Stream,
        channel::mpsc,
        stream::{BoxStream, StreamExt},
    },
};
use tokio::sync::broadcast;

/// Drain a `broadcast::Receiver` into an `mpsc::Sender`, warning on lag and
/// terminating cleanly when the broadcaster closes or the sink drops.
async fn drain_to_sink<T>(
    mut rx: broadcast::Receiver<T>,
    mut out: mpsc::Sender<T>,
    lag_label: &'static str,
) where
    T: Clone + Send + 'static,
{
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
}

/// Iced stream backed by a process-static [`OnceLock`] receiver. Consumed via
/// `Subscription::run(fn_pointer)` — the `fn` pointer carries the subscription
/// identity. Empty `OnceLock` means the source failed to install; the stream
/// completes immediately and the subscription stays idle.
pub fn from_once_lock<T: Clone + Send + 'static>(
    source: &'static OnceLock<broadcast::Receiver<T>>,
    lag_label: &'static str,
) -> impl Stream<Item = T> {
    iced::stream::channel(16, move |out: mpsc::Sender<_>| async move {
        let Some(rx) = source.get() else {
            return;
        };
        drain_to_sink(rx.resubscribe(), out, lag_label).await;
    })
}

/// Build a [`Subscription`] from an App-owned `broadcast::Sender`. The
/// subscription identity is `(TypeId<T>, label)` — pass a stable label so the
/// recipe survives `subscription()` rebuilds. Each run calls
/// [`broadcast::Sender::subscribe`] for a fresh receiver, so multiple App
/// instances (e.g. across tests) don't fight over a shared receiver.
///
/// `label` doubles as the warning tag emitted on `Lagged`.
pub fn subscription_from_sender<T>(
    label: &'static str,
    sender: broadcast::Sender<T>,
) -> Subscription<T>
where
    T: Clone + Send + 'static,
{
    from_recipe(SenderRecipe {
        label,
        sender,
        _t: std::marker::PhantomData,
    })
}

struct SenderRecipe<T> {
    label: &'static str,
    sender: broadcast::Sender<T>,
    _t: std::marker::PhantomData<T>,
}

impl<T> Recipe for SenderRecipe<T>
where
    T: Clone + Send + 'static,
{
    type Output = T;

    fn hash(&self, state: &mut Hasher) {
        // `TypeId::of::<Self>()` encodes both the recipe type and its `T`
        // generic, so a different recipe (or a different message type)
        // sharing the same `label` can't collide with this one.
        std::any::TypeId::of::<Self>().hash(state);
        self.label.hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, T> {
        let label = self.label;
        let sender = self.sender;
        iced::stream::channel(16, move |out: mpsc::Sender<T>| async move {
            drain_to_sink(sender.subscribe(), out, label).await;
        })
        .boxed()
    }
}
