//! Per-window screen-capture exclusion.
//!
//! Mirrors Bitwarden's Electron client (`BrowserWindow.setContentProtection`):
//! when `protect = true` the window is hidden from screenshots and screen-
//! recording APIs at the OS level. Toggle is runtime-settable on Windows
//! and macOS — no restart needed despite what the previous TODO entry
//! claimed.
//!
//! Linux is genuinely unsupported: winit's `set_content_protected` is a
//! no-op on both X11 and Wayland (no upstream protocol), matching
//! Electron's behavior on Linux. The function still returns a `Task` so
//! the caller doesn't need to gate on `cfg(target_os = ...)` at every
//! call site.
//!
//! ## Caller responsibility
//!
//! Apply at startup (post-`window::open`) for the persisted state, and on
//! any `SettingChange::AllowScreenshots` toggle. The settings flow uses
//! the "confirm window still visible" pattern (5 s timeout, auto-revert)
//! so a user accidentally enabling protection in a remote-desktop session
//! doesn't lose access to the window.

use iced::{Task, window};

/// Set whether `window_id` is excluded from screen capture.
///
/// - **Windows**: `SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE | WDA_NONE)`.
///   Win10 build 2004+ truly excludes the window; earlier builds fall
///   back to `WDA_MONITOR` (window goes black during capture).
/// - **macOS**: `NSWindow.sharingType = NSWindowSharingNone | NSWindowSharingReadOnly`.
/// - **Linux / other**: no-op.
pub fn apply(window_id: window::Id, protect: bool) -> Task<()> {
    iced::window::run(window_id, move |w| {
        #[cfg(target_os = "windows")]
        apply_windows(w, protect);
        #[cfg(target_os = "macos")]
        apply_macos(w, protect);
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let _ = (w, protect);
    })
}

#[cfg(target_os = "windows")]
fn apply_windows(w: &dyn window::Window, protect: bool) {
    use iced::window::raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE},
    };

    let Ok(handle) = w.window_handle() else {
        tracing::warn!("screenshot_protection: window_handle unavailable");
        return;
    };
    let RawWindowHandle::Win32(h) = handle.as_raw() else {
        tracing::warn!("screenshot_protection: non-Win32 handle on Windows");
        return;
    };
    let hwnd: HWND = h.hwnd.get() as *mut _;
    let affinity = if protect {
        WDA_EXCLUDEFROMCAPTURE
    } else {
        WDA_NONE
    };
    // SAFETY: hwnd came from winit via raw-window-handle; valid for the
    // lifetime of the window. SetWindowDisplayAffinity is documented as
    // safe to call from any thread for any window the process owns.
    unsafe {
        SetWindowDisplayAffinity(hwnd, affinity);
    }
}

#[cfg(target_os = "macos")]
fn apply_macos(w: &dyn window::Window, protect: bool) {
    use iced::window::raw_window_handle::RawWindowHandle;
    use objc2_app_kit::{NSView, NSWindowSharingType};

    let Ok(handle) = w.window_handle() else {
        tracing::warn!("screenshot_protection: window_handle unavailable");
        return;
    };
    let RawWindowHandle::AppKit(h) = handle.as_raw() else {
        tracing::warn!("screenshot_protection: non-AppKit handle on macOS");
        return;
    };

    // SAFETY: AppKitWindowHandle.ns_view is a non-null pointer to a live
    // NSView for as long as the iced window exists. iced runs window
    // operations on the main thread, where AppKit access is required.
    let ns_view: &NSView = unsafe { h.ns_view.cast::<NSView>().as_ref() };
    let Some(ns_window) = ns_view.window() else {
        tracing::warn!("screenshot_protection: NSView has no window");
        return;
    };
    let sharing = if protect {
        NSWindowSharingType::None
    } else {
        NSWindowSharingType::ReadOnly
    };
    ns_window.setSharingType(sharing);
}
