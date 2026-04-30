//! Windows lock/sleep events via a hidden message-only window.
//!
//! The window class is registered and a `HWND_MESSAGE` window created on a
//! dedicated OS thread that runs the standard Win32 message pump. The window
//! procedure translates `WM_WTSSESSION_CHANGE` and `WM_POWERBROADCAST`
//! messages into [`SessionEvent`]s and forwards them through a
//! [`tokio::sync::mpsc`] channel.
//!
//! When the consumer drops the stream, the [`Drop`] impl posts `WM_QUIT` to
//! the worker thread; `GetMessageW` returns 0, the thread destroys the window
//! (which triggers the `WM_DESTROY` cleanup path) and exits. The drop is
//! non-blocking — we don't `join()` the thread, since `Drop` may run on a
//! tokio worker.

use std::pin::Pin;
use std::sync::mpsc as std_mpsc;
use std::task::{Context, Poll};
use std::thread;
use std::time::Duration;

use futures_core::Stream;
use tokio::sync::mpsc;
use windows::core::{PCWSTR, w};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Power::{
    PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMESUSPEND, PBT_APMSUSPEND,
};
use windows::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetMessageW,
    GetWindowLongPtrW, HWND_MESSAGE, MSG, PostThreadMessageW, RegisterClassW, SetWindowLongPtrW,
    TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_DESTROY, WM_POWERBROADCAST, WM_QUIT,
    WM_WTSSESSION_CHANGE, WNDCLASSW, WTS_SESSION_LOCK, WTS_SESSION_UNLOCK,
};

use crate::SessionEvent;

const CLASS_NAME: PCWSTR = w!("BitwardenSessionEventsWindow");

pub(crate) fn platform_stream() -> impl Stream<Item = SessionEvent> + Send + 'static {
    let (tx, rx) = mpsc::channel::<SessionEvent>(16);
    let (tid_tx, tid_rx) = std_mpsc::sync_channel::<u32>(1);

    let spawn_result = thread::Builder::new()
        .name("session-events-windows".into())
        .spawn(move || run_message_loop(tx, tid_tx));

    let thread_id = match spawn_result {
        // The worker sends its TID as the very first thing it does; 100 ms is
        // generous for a local thread spawn.
        Ok(_) => tid_rx.recv_timeout(Duration::from_millis(100)).ok(),
        Err(e) => {
            tracing::debug!(error = %e, "failed to spawn Windows pump thread");
            None
        }
    };

    WindowsStream { rx, thread_id }
}

struct WindowsStream {
    rx: mpsc::Receiver<SessionEvent>,
    thread_id: Option<u32>,
}

impl Stream for WindowsStream {
    type Item = SessionEvent;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

impl Drop for WindowsStream {
    fn drop(&mut self) {
        if let Some(tid) = self.thread_id.take() {
            // SAFETY: tid was obtained from GetCurrentThreadId() on a thread
            // we spawned; posting WM_QUIT to it is always safe.
            let _ = unsafe { PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0)) };
        }
    }
}

fn run_message_loop(tx: mpsc::Sender<SessionEvent>, tid_tx: std_mpsc::SyncSender<u32>) {
    // SAFETY: GetCurrentThreadId is documented as always-succeed; the rest of
    // the body is documented Win32 with checked error paths.
    unsafe {
        let tid = GetCurrentThreadId();
        if tid_tx.send(tid).is_err() {
            return;
        }

        let hinstance = match GetModuleHandleW(None) {
            Ok(h) => HINSTANCE(h.0),
            Err(e) => {
                tracing::debug!(error = %e, "GetModuleHandleW failed");
                return;
            }
        };

        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: hinstance,
            lpszClassName: CLASS_NAME,
            ..Default::default()
        };
        // RegisterClassW returns 0 on failure; a class already registered in
        // the same process returns a non-zero atom too. Either way is fine.
        let _ = RegisterClassW(&class);

        let tx_box: Box<mpsc::Sender<SessionEvent>> = Box::new(tx);
        let tx_ptr = Box::into_raw(tx_box);

        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE(0),
            CLASS_NAME,
            w!("session-events"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(hinstance),
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                tracing::debug!(error = %e, "CreateWindowExW failed");
                drop(Box::from_raw(tx_ptr));
                return;
            }
        };

        SetWindowLongPtrW(hwnd, GWLP_USERDATA, tx_ptr as isize);

        if let Err(e) = WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) {
            tracing::debug!(error = %e, "WTSRegisterSessionNotification failed");
            // Continue anyway — power events still flow via WM_POWERBROADCAST.
        }

        // Pump messages. None for hwnd lets us also receive WM_QUIT posted to
        // the thread queue (it has no associated window). WM_QUIT is special-
        // cased by GetMessageW regardless of the filter, but using None keeps
        // any future thread-message use working correctly too.
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // WM_QUIT received: tear down. DestroyWindow fires WM_DESTROY
        // synchronously, which our window proc uses to unregister the WTS
        // notification and free the leaked Sender box.
        let _ = DestroyWindow(hwnd);
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_WTSSESSION_CHANGE => {
                let event = match wparam.0 as u32 {
                    WTS_SESSION_LOCK => Some(SessionEvent::Locked),
                    WTS_SESSION_UNLOCK => Some(SessionEvent::Unlocked),
                    _ => None,
                };
                if let Some(ev) = event {
                    forward(hwnd, ev);
                }
                LRESULT(0)
            }
            WM_POWERBROADCAST => {
                let event = match wparam.0 as u32 {
                    PBT_APMSUSPEND => Some(SessionEvent::Suspended),
                    PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND => Some(SessionEvent::Resumed),
                    _ => None,
                };
                if let Some(ev) = event {
                    forward(hwnd, ev);
                }
                // 1 = TRUE acknowledgement.
                LRESULT(1)
            }
            WM_DESTROY => {
                let _ = WTSUnRegisterSessionNotification(hwnd);
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut mpsc::Sender<SessionEvent>;
                if !ptr.is_null() {
                    drop(Box::from_raw(ptr));
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                }
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

unsafe fn forward(hwnd: HWND, ev: SessionEvent) {
    unsafe {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const mpsc::Sender<SessionEvent>;
        if let Some(tx) = ptr.as_ref() {
            let _ = tx.try_send(ev);
        }
    }
}
