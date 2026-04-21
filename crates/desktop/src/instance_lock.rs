//! Single-instance lock with IPC wake-up.
//!
//! Prevents a second launch from starting a duplicate process — and instead
//! tells the already-running primary to show its window. The transport is a
//! per-platform local IPC primitive driven by tokio (already a dep via iced):
//!
//! - **Unix**: Unix domain socket in a per-user directory (see [`socket_path`]).
//! - **Windows**: named pipe at `\\.\pipe\bitwarden-desktop-next`.
//!
//! The sync probe in [`notify_primary_if_running`] runs before iced starts
//! (no tokio runtime yet) and uses stdlib sockets to send `b"show\n"` to a
//! running primary. On the primary side, [`wake_stream`] is an iced
//! [`Subscription::run`]-compatible stream that binds the listener and yields
//! `()` for each `"show"` signal received.

use iced::futures::{SinkExt, Stream, channel::mpsc};

#[cfg(windows)]
const PIPE_NAME: &str = r"\\.\pipe\bitwarden-desktop-next";

/// Per-user Unix socket path. Uses `$XDG_RUNTIME_DIR` when set (Linux, mode 700
/// and owned by the current user) and falls back to the process temp dir —
/// `$TMPDIR` on macOS is already per-user, and we suffix with the uid on Linux
/// so `/tmp` is not shared across users.
#[cfg(unix)]
fn socket_path() -> std::path::PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return std::path::PathBuf::from(dir).join("bitwarden-desktop-next.sock");
    }
    std::env::temp_dir().join("bitwarden-desktop-next.sock")
}

/// Timeout for a primary → second-launch handshake. The probe writes a single
/// 5-byte line then closes; if anything takes longer than this, we assume a
/// misbehaving or malicious peer is holding the connection open and drop it.
const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

/// Probe for an already-running primary. If one is listening, send the wake
/// signal and return `true`; the caller should then exit cleanly.
///
/// Returns `false` when no primary is listening — in which case this process
/// should go on to become the primary.
pub fn notify_primary_if_running() -> bool {
    #[cfg(unix)]
    {
        use std::{io::Write, os::unix::net::UnixStream};
        if let Ok(mut s) = UnixStream::connect(socket_path()) {
            let _ = s.write_all(b"show\n");
            return true;
        }
    }
    #[cfg(windows)]
    {
        use std::{fs::OpenOptions, io::Write};
        if let Ok(mut f) = OpenOptions::new().read(true).write(true).open(PIPE_NAME) {
            let _ = f.write_all(b"show\n");
            return true;
        }
    }
    false
}

/// Remove a stale Unix socket file left over from a hard-killed primary.
///
/// Safe to call **only after** [`notify_primary_if_running`] returned
/// `false` — that confirms nobody is listening, so any file on the path
/// is definitionally stale. No-op on Windows (named pipes are kernel
/// objects that vanish with the process).
pub fn cleanup_stale_socket() {
    #[cfg(unix)]
    {
        let _ = std::fs::remove_file(socket_path());
    }
}

/// Iced-compatible stream that yields `()` every time another instance sends
/// a wake signal. Feed to [`iced::Subscription::run`] with a `fn` pointer so
/// iced can hash the subscription identity and keep the listener alive
/// across `update()` cycles.
pub fn wake_stream() -> impl Stream<Item = ()> {
    iced::stream::channel(16, |out: mpsc::Sender<()>| async move {
        #[cfg(unix)]
        {
            use tokio::net::UnixListener;
            let path = socket_path();
            let listener = match UnixListener::bind(&path) {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!(error = %e, path = %path.display(), "failed to bind instance socket");
                    return;
                }
            };
            loop {
                let stream = match listener.accept().await {
                    Ok((s, _)) => s,
                    Err(e) => {
                        tracing::warn!(error = %e, "instance socket accept failed");
                        continue;
                    }
                };
                let out = out.clone();
                tokio::spawn(handle_wake_client(stream, out));
            }
        }
        #[cfg(windows)]
        {
            use tokio::net::windows::named_pipe::ServerOptions;
            loop {
                // Windows requires a fresh pipe instance per accepted client.
                let server = match ServerOptions::new().create(PIPE_NAME) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!(error = %e, "CreateNamedPipe failed; wake listener stopping");
                        return;
                    }
                };
                if let Err(e) = server.connect().await {
                    tracing::warn!(error = %e, "named pipe connect failed");
                    continue;
                }
                let out = out.clone();
                tokio::spawn(handle_wake_client(server, out));
            }
        }
    })
}

/// Read one line from a newly-accepted client, forward `"show"` to the app,
/// and drop the connection. Bounded by [`READ_TIMEOUT`] so a client that
/// connects-and-stalls can't pin this task forever.
async fn handle_wake_client<S>(stream: S, mut out: mpsc::Sender<()>)
where
    S: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::{AsyncBufReadExt, BufReader};
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    match tokio::time::timeout(READ_TIMEOUT, reader.read_line(&mut line)).await {
        Ok(Ok(n)) if n > 0 && line.trim() == "show" => {
            let _ = out.send(()).await;
        }
        Ok(Ok(_)) => {} // EOF or non-matching payload; ignore
        Ok(Err(e)) => tracing::debug!(error = %e, "instance wake read error"),
        Err(_) => tracing::debug!("instance wake read timed out"),
    }
    // `reader` drops here → stream closes immediately.
}
