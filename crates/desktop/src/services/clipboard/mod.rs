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
    /// `None` disables auto-clear AND clear-on-close — the worker drops any
    /// outstanding tracked value (without clearing it) and stops starting timers.
    SetTimeout(Option<Duration>),
}

pub struct ClipboardManager {
    /// `Option` so `Drop` can drop the sender (signals worker to terminate)
    /// before joining.
    tx: Option<mpsc::Sender<Command>>,
    worker: Option<JoinHandle<()>>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();

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

    /// `None` disables auto-clear AND clear-on-close. Passing `Some` re-enables
    /// tracking for future copies; a copy already in the clipboard before this
    /// call is not retroactively tracked.
    pub fn set_timeout(&self, timeout: Option<Duration>) {
        self.send(Command::SetTimeout(timeout));
    }

    fn send(&self, cmd: Command) {
        let Some(tx) = self.tx.as_ref() else {
            return;
        };
        if let Err(e) = tx.send(cmd) {
            tracing::error!(%e, "clipboard worker unreachable");
        }
    }
}

impl Drop for ClipboardManager {
    fn drop(&mut self) {
        // Drop the sender → worker sees `Disconnected`, runs its final clear,
        // and exits. This is also our clear-on-close path: `App` drop runs
        // during iced's shutdown, before the process exits.
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

    let mut last: Option<String> = None;
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

                if timeout.is_some() {
                    last = Some(value);
                } else {
                    last = None;
                }
            }
            Ok(Command::SetTimeout(new_timeout)) => {
                timeout = new_timeout;
                if timeout.is_none() {
                    last = None;
                }
            }
            Err(e) => {
                // Clear on timeout/exit if our last-written value is still there.
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

/// Validate that `uri` is safe to hand to `open::that_detached`. Returns
/// the parsed URL when the scheme is `http`/`https` and the host is
/// non-empty; rejects everything else. `file://`, `javascript:`, and
/// `data:` must never reach the launcher.
///
/// Bitwarden frequently stores URIs without a scheme (e.g.
/// `acmecorp.atlassian.net`); when the input has no `://` the function
/// prepends `https://` so bare hosts launch correctly. Inputs that
/// already contain `://` are parsed strictly — without that gate, an
/// input like `https://` would fall back to `https://https://` and
/// accept it with host = `"https"`.
fn validate_launchable_url(uri: &str) -> Option<url::Url> {
    let parsed = if uri.contains("://") {
        url::Url::parse(uri).ok()?
    } else {
        url::Url::parse(&format!("https://{uri}")).ok()?
    };
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    if parsed.host_str().is_none_or(str::is_empty) {
        return None;
    }
    Some(parsed)
}

/// Open `uri` in the default browser, but only if it passes
/// [`validate_launchable_url`].
pub fn launch_url(uri: &str) {
    let Some(parsed) = validate_launchable_url(uri) else {
        tracing::debug!(uri, "launch_url: rejected");
        return;
    };
    if let Err(e) = open::that_detached(parsed.as_str()) {
        tracing::warn!(%e, "open::that_detached failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_https_url_is_accepted() {
        let url = validate_launchable_url("https://example.com/path").expect("accepted");
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("example.com"));
    }

    #[test]
    fn launch_http_url_is_accepted() {
        let url = validate_launchable_url("http://example.com").expect("accepted");
        assert_eq!(url.scheme(), "http");
    }

    #[test]
    fn launch_bare_host_gets_https_prepended() {
        // Mirrors how Bitwarden stores URIs (no scheme).
        let url = validate_launchable_url("acmecorp.atlassian.net").expect("accepted");
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("acmecorp.atlassian.net"));
    }

    #[test]
    fn launch_bare_host_with_path_gets_https_prepended() {
        let url = validate_launchable_url("example.com/login").expect("accepted");
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.path(), "/login");
    }

    #[test]
    fn launch_rejects_javascript_scheme() {
        // The headline attack: stored URI with `javascript:` would otherwise
        // execute in whatever browser handles the protocol.
        assert!(validate_launchable_url("javascript:alert(1)").is_none());
    }

    #[test]
    fn launch_rejects_file_scheme() {
        assert!(validate_launchable_url("file:///etc/passwd").is_none());
    }

    #[test]
    fn launch_rejects_data_scheme() {
        assert!(validate_launchable_url("data:text/html,<script>alert(1)</script>").is_none());
    }

    #[test]
    fn launch_rejects_other_schemes() {
        // Schemes that use `://` go down the strict path and reject via
        // the scheme allowlist.
        assert!(validate_launchable_url("ftp://example.com/").is_none());
        assert!(validate_launchable_url("ssh://example.com").is_none());
        // Schemes that use opaque payloads (`scheme:payload`) hit the
        // fallback path; their payload mangles into an invalid host or
        // port, so the parse fails outright.
        assert!(validate_launchable_url("vbscript:msgbox").is_none());
        assert!(validate_launchable_url("tel:+15551234").is_none());
    }

    #[test]
    fn launch_rejects_empty_input() {
        assert!(validate_launchable_url("").is_none());
        assert!(validate_launchable_url("   ").is_none());
    }

    #[test]
    fn launch_rejects_empty_host_after_scheme() {
        // The fallback `https://`-prepend used to corrupt this into
        // `https://https://`, accepting it as host = "https". The strict
        // path now rejects it cleanly.
        assert!(validate_launchable_url("https://").is_none());
        assert!(validate_launchable_url("http://").is_none());
    }
}
