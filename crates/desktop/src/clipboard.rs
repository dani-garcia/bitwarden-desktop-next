//! Clipboard + URL-launch helpers for the detail pane's copy/launch buttons.
//!
//! A single long-lived worker thread owns the `arboard::Clipboard` and
//! serves copy commands over an `mpsc` channel. The worker's main loop
//! does `recv_timeout(timeout)` — arriving commands reset the timer,
//! a timeout fires the auto-clear. On channel disconnect (e.g. app
//! shutdown, via `ClipboardManager`'s `Drop`) the worker runs a final
//! clear before returning.
//!
//! Sensitive values are marked via platform-specific arboard extensions
//! (Windows: exclude from history + cloud + monitoring; macOS / Linux:
//! exclude from history) — matches the official Bitwarden client.
//!
//! `timeout = None` disables both the auto-clear and the clear-on-close.

use std::{
    sync::mpsc::{self, RecvTimeoutError},
    thread::{self, JoinHandle},
    time::Duration,
};

/// Hardcoded for now. A future per-user setting will flow into
/// `ClipboardManager::set_timeout()` when user switches happen.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy)]
pub enum Sensitivity {
    /// Allow the clipboard manager / cloud clipboard to retain the value.
    /// Use for usernames and URLs — matches the official Bitwarden client.
    Normal,
    /// Exclude from Windows clipboard history + cloud sync, macOS
    /// concealed-type pasteboard, Linux exclude-from-history. Use for
    /// passwords and TOTP.
    Sensitive,
}

enum Command {
    Copy {
        value: String,
        sensitivity: Sensitivity,
    },
    /// Update the auto-clear timeout. `None` disables auto-clear AND
    /// clear-on-close — the worker drops any outstanding tracked value
    /// (without clearing it) and stops starting new timers.
    SetTimeout(Option<Duration>),
}

pub struct ClipboardManager {
    /// `Option` so `Drop` can drop the sender (signals worker to
    /// terminate) before joining.
    tx: Option<mpsc::Sender<Command>>,
    /// Kept so `Drop` can poll `is_finished` and then `join` the thread.
    worker: Option<JoinHandle<()>>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();

        // Default until per-user config is loaded; callers can override
        // at any time via `set_timeout`.
        let timeout = Some(DEFAULT_TIMEOUT);
        let worker = thread::Builder::new()
            .name("clipboard".into())
            .spawn(move || worker_loop(cmd_rx, timeout))
            .expect("spawn clipboard worker thread");

        Self {
            tx: Some(cmd_tx),
            worker: Some(worker),
        }
    }

    pub fn copy(&self, value: String, sensitivity: Sensitivity) {
        self.send(Command::Copy { value, sensitivity });
    }

    /// Change the auto-clear timeout. `None` disables auto-clear AND
    /// clear-on-close: the worker drops any outstanding tracked value
    /// (without clearing it) and future copies won't schedule a timer.
    /// Passing `Some` re-enables tracking for future copies; a copy that
    /// was already in the clipboard before this call is not retroactively
    /// tracked.
    pub fn set_timeout(&self, timeout: Option<Duration>) {
        self.send(Command::SetTimeout(timeout));
    }

    fn send(&self, cmd: Command) {
        let Some(tx) = self.tx.as_ref() else {
            return;
        };
        if let Err(e) = tx.send(cmd) {
            // The worker thread panicked or exited. Not much we can do.
            tracing::error!(%e, "clipboard worker unreachable");
        }
    }
}

impl Drop for ClipboardManager {
    fn drop(&mut self) {
        // Drop the sender → worker sees `Disconnected`, runs its final
        // clear, and exits. This is also our clear-on-close path: `App`
        // drop runs during iced's shutdown, before the process exits.
        self.tx = None;

        let Some(worker) = self.worker.take() else {
            return;
        };
        let _ = worker.join();
    }
}

fn worker_loop(cmd_rx: mpsc::Receiver<Command>, initial_timeout: Option<Duration>) {
    let mut clipboard = match arboard::Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(%e, "arboard init failed; clipboard manager disabled");
            return;
        }
    };

    // The value currently held in the clipboard by us.
    let mut last: Option<String> = None;
    // The timeout for auto-clearing the clipboard. `None` disables auto-clear
    let mut timeout = initial_timeout;

    loop {
        // Only wait with a timeout when we're tracking an outstanding
        // clear. Otherwise block indefinitely on recv.
        let recv = match (last.as_ref(), timeout) {
            (Some(_), Some(t)) => cmd_rx.recv_timeout(t),
            _ => cmd_rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };

        match recv {
            Ok(Command::Copy { value, sensitivity }) => {
                if let Err(e) = arboard_write(&mut clipboard, &value, sensitivity) {
                    tracing::warn!(%e, "clipboard write failed");
                    continue;
                }

                // Track the copied value when an auto-clear timeout is active.
                if timeout.is_some() {
                    last = Some(value);
                } else {
                    last = None;
                }
            }
            Ok(Command::SetTimeout(new_timeout)) => {
                timeout = new_timeout;
                // If we're disabling the timeout, we don't care about tracking the value anymore.
                if timeout.is_none() {
                    last = None;
                }
            }
            Err(e) => {
                // Clear the clipboard on timeout/exit and the value is still there.
                if let Some(expected) = last.take()
                    && clipboard.get_text().unwrap_or_default() == expected
                    && let Err(e) = clipboard.clear()
                {
                    tracing::warn!(%e, "clipboard clear failed");
                }

                if matches!(e, RecvTimeoutError::Disconnected) {
                    break;
                }
            }
        }
    }
    tracing::info!("clipboard worker exiting");
}

// ── arboard + platform extensions ─────────────────────────────────────────

fn arboard_write(
    clipboard: &mut arboard::Clipboard,
    value: &str,
    sensitivity: Sensitivity,
) -> Result<(), arboard::Error> {
    let mut set = clipboard.set();

    if matches!(sensitivity, Sensitivity::Sensitive) {
        #[cfg(target_os = "windows")]
        {
            use arboard::SetExtWindows;
            set = set
                .exclude_from_history()
                .exclude_from_cloud()
                .exclude_from_monitoring();
        }

        #[cfg(target_os = "macos")]
        {
            use arboard::SetExtApple;
            set = set.exclude_from_history();
        }

        #[cfg(target_os = "linux")]
        {
            use arboard::SetExtLinux;
            set = set.exclude_from_history();
        }
    }

    set.text(value)
}

// ── URL launching ─────────────────────────────────────────────────────────

/// Open `uri` in the default browser, but only if the scheme is `http`
/// or `https` and the host is non-empty. Anything else is silently
/// ignored — `file://`, `javascript:`, and `data:` must never be handed
/// to `open::that_detached`.
///
/// Bitwarden frequently stores URIs without a scheme (e.g.
/// `acmecorp.atlassian.net`). If the first parse fails, retry with
/// `https://` prepended.
pub fn launch_url(uri: &str) {
    let Ok(parsed) = url::Url::parse(uri).or_else(|_| url::Url::parse(&format!("https://{uri}")))
    else {
        tracing::debug!(uri, "launch_url: unparseable URI");
        return;
    };
    if !matches!(parsed.scheme(), "http" | "https") {
        tracing::debug!(scheme = parsed.scheme(), "launch_url: scheme rejected");
        return;
    }
    if parsed.host_str().is_none_or(str::is_empty) {
        tracing::debug!(uri, "launch_url: empty host");
        return;
    }

    if let Err(e) = open::that_detached(parsed.as_str()) {
        tracing::warn!(%e, "open::that_detached failed");
    }
}
