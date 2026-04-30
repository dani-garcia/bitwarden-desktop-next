//! Cross-platform stream of OS session events.
//!
//! Wraps the per-platform OS APIs for screen lock, unlock, suspend, and resume
//! into a single Rust async stream. Consumers get a [`SessionEvent`] each time
//! the OS reports one of those state changes.
//!
//! Platforms:
//! - **Linux**: logind D-Bus signals (`Lock`/`Unlock` and `PrepareForSleep`).
//!   Requires systemd-logind on the system bus; otherwise the stream stays
//!   silent.
//! - **macOS**: `NSDistributedNotificationCenter` (`com.apple.screenIsLocked`
//!   / `com.apple.screenIsUnlocked`) and `NSWorkspace.notificationCenter`
//!   (`NSWorkspaceWillSleep` / `NSWorkspaceDidWake`).
//! - **Windows**: a hidden message-only window receives
//!   `WM_WTSSESSION_CHANGE` and `WM_POWERBROADCAST`.
//! - **Other targets**: empty stream.
//!
//! The returned stream assumes a tokio runtime is available — the desktop app
//! provides one via iced. Errors during setup are logged at `debug` and end
//! the stream cleanly; the feature degrades silently rather than crashing.

use futures_core::Stream;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/// A change in the OS session lock or power state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    /// The screen has been locked.
    Locked,
    /// The screen has been unlocked.
    Unlocked,
    /// The system is about to suspend (sleep / hibernate).
    Suspended,
    /// The system has resumed from suspend.
    Resumed,
}

/// Returns a stream of session events for the current process's session.
///
/// On unsupported targets, or if the underlying OS service is unavailable,
/// yields no events.
pub fn event_stream() -> impl Stream<Item = SessionEvent> + Send + 'static {
    #[cfg(target_os = "linux")]
    {
        linux::platform_stream()
    }
    #[cfg(target_os = "macos")]
    {
        macos::platform_stream()
    }
    #[cfg(target_os = "windows")]
    {
        windows::platform_stream()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        /// A stream that never yields. Used on platforms with no native session-event
        /// source so the feature degrades to a silent no-op rather than failing.
        struct EmptyStream;

        impl Stream for EmptyStream {
            type Item = SessionEvent;
            fn poll_next(
                self: std::pin::Pin<&mut Self>,
                _: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Option<Self::Item>> {
                std::task::Poll::Pending
            }
        }
        EmptyStream
    }
}
