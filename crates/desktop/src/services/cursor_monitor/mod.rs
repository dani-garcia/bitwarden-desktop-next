//! Cursor-position monitor lookup for multi-monitor window placement.
//!
//! Used by the Magnify launcher to summon on whichever screen the user is
//! currently focused on. Today **every platform** returns `None` and the
//! caller falls back to `Position::Centered`.
//!
//! TODO: implement per platform. Windows uses `GetCursorPos` +
//! `MonitorFromPoint` + `GetMonitorInfoW` + `GetDpiForMonitor` via
//! `windows-sys` (sketched below). macOS uses `NSEvent::mouseLocation` +
//! `NSScreen::screens()`. Linux X11 uses `XQueryPointer` + Xinerama; Wayland
//! needs the input-method protocol or a desktop-environment-specific shim.
//! See [docs/todo.md] → "Magnify launcher polish → Cursor-monitor centering
//! on summon".
//!
//! Logical pixels here are what iced expects in
//! [`iced::window::Position::Specific`] / [`iced::window::move_to`] —
//! physical pixels divided by the source monitor's DPI scale factor.

/// Center of the work area (taskbar-excluded) of whichever monitor the cursor
/// is currently over, in **logical** pixels. Returns `None` if the platform
/// query fails or the platform is unsupported — callers should fall back to
/// `Position::Centered`.
pub fn cursor_monitor_logical_center() -> Option<(f32, f32)> {
    None
}

// ── Reference implementation (Windows) ───────────────────────────────────
//
// Drop-in once the dependency is approved. Add to `crates/desktop/Cargo.toml`:
//
// ```toml
// [target.'cfg(target_os = "windows")'.dependencies]
// windows-sys = { version = "0.61", features = [
//     "Win32_Foundation",
//     "Win32_Graphics_Gdi",
//     "Win32_UI_HiDpi",
//     "Win32_UI_WindowsAndMessaging",
// ] }
// ```
//
// Then replace the `None`-only stub above with:
//
// ```rust
// #[cfg(target_os = "windows")]
// pub fn cursor_monitor_logical_center() -> Option<(f32, f32)> {
//     use windows_sys::Win32::Foundation::POINT;
//     use windows_sys::Win32::Graphics::Gdi::{
//         GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
//     };
//     use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
//     use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
//
//     // SAFETY: every Win32 call below is documented to be safe to invoke
//     // from any thread; pointers we hand in are stack-allocated and outlive
//     // the call. Failures are handled by returning `None`.
//     unsafe {
//         let mut pt = POINT { x: 0, y: 0 };
//         if GetCursorPos(&mut pt) == 0 {
//             return None;
//         }
//
//         let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
//         if monitor.is_null() {
//             return None;
//         }
//
//         let mut info: MONITORINFO = std::mem::zeroed();
//         info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
//         if GetMonitorInfoW(monitor, &mut info) == 0 {
//             return None;
//         }
//
//         // Per-monitor effective DPI — falls back to 96 (1.0× scale) if the
//         // call fails. iced expects logical pixels = physical / scale.
//         let mut dpi_x: u32 = 96;
//         let mut dpi_y: u32 = 96;
//         let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
//         let scale = (dpi_x.max(96)) as f32 / 96.0;
//
//         let cx_phys = (info.rcWork.left + info.rcWork.right) as f32 / 2.0;
//         let cy_phys = (info.rcWork.top + info.rcWork.bottom) as f32 / 2.0;
//         Some((cx_phys / scale, cy_phys / scale))
//     }
// }
//
// #[cfg(not(target_os = "windows"))]
// pub fn cursor_monitor_logical_center() -> Option<(f32, f32)> {
//     None
// }
// ```
