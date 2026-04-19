//! Single-instance lock with IPC wake-up.
//!
//! Prevents a second launch from starting a duplicate process — and instead
//! tells the already-running primary to show its window. The transport is a
//! per-platform local IPC primitive driven by tokio (already a dep via iced):
//!
//! - **Unix**: Unix domain socket at `/tmp/bitwarden-desktop-next.sock`.
//! - **Windows**: named pipe at `\\.\pipe\bitwarden-desktop-next`.
//!
//! The sync probe in [`notify_primary_if_running`] runs before iced starts
//! (no tokio runtime yet) and uses stdlib sockets to send `b"show\n"` to a
//! running primary. On the primary side, [`wake_stream`] is an iced
//! [`Subscription::run`]-compatible stream that binds the listener and yields
//! `()` for each `"show"` signal received.

use iced::futures::{SinkExt, Stream, channel::mpsc};

#[cfg(unix)]
const SOCKET_PATH: &str = "/tmp/bitwarden-desktop-next.sock";

#[cfg(windows)]
const PIPE_NAME: &str = r"\\.\pipe\bitwarden-desktop-next";

/// Probe for an already-running primary. If one is listening, send the wake
/// signal and return `true`; the caller should then exit cleanly.
///
/// Returns `false` when no primary is listening — in which case this process
/// should go on to become the primary.
pub fn notify_primary_if_running() -> bool {
    #[cfg(unix)]
    {
        use std::{io::Write, os::unix::net::UnixStream};
        if let Ok(mut s) = UnixStream::connect(SOCKET_PATH) {
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
        let _ = std::fs::remove_file(SOCKET_PATH);
    }
}

/// Iced-compatible stream that yields `()` every time another instance sends
/// a wake signal. Feed to [`iced::Subscription::run`] with a `fn` pointer so
/// iced can hash the subscription identity and keep the listener alive
/// across `update()` cycles.
pub fn wake_stream() -> impl Stream<Item = ()> {
    iced::stream::channel(4, |mut out: mpsc::Sender<()>| async move {
        #[cfg(unix)]
        {
            use tokio::{
                io::{AsyncBufReadExt, BufReader},
                net::UnixListener,
            };
            let listener = match UnixListener::bind(SOCKET_PATH) {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!(error = %e, path = SOCKET_PATH, "failed to bind instance socket");
                    return;
                }
            };
            loop {
                let (stream, _) = match listener.accept().await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!(error = %e, "instance socket accept failed");
                        continue;
                    }
                };
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                if reader.read_line(&mut line).await.is_ok() && line.trim() == "show" {
                    let _ = out.send(()).await;
                }
            }
        }
        #[cfg(windows)]
        {
            use tokio::{
                io::{AsyncBufReadExt, BufReader},
                net::windows::named_pipe::ServerOptions,
            };
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
                let mut reader = BufReader::new(server);
                let mut line = String::new();
                if reader.read_line(&mut line).await.is_ok() && line.trim() == "show" {
                    let _ = out.send(()).await;
                }
                // reader drops → pipe instance closes → loop creates the next.
            }
        }
    })
}
