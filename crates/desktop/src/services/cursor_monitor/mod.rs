//! Cursor-position monitor lookup for multi-monitor window placement.
//!
//! Used by the Magnify launcher to summon on whichever screen the user is
//! currently focused on. Windows and macOS query the OS for cursor + monitor
//! geometry; Linux returns `None` (winit's `Position::Centered` falls back to
//! the primary monitor — Wayland has no global-cursor protocol, X11 support
//! is a follow-up tracked in `docs/todo.md`).
//!
//! Logical pixels here are what iced expects in
//! [`iced::window::Position::Specific`] / [`iced::window::move_to`] —
//! physical pixels divided by the source monitor's DPI scale factor.

/// Center of the work area (taskbar / dock / menu-bar excluded) of whichever
/// monitor the cursor is currently over, in **logical** pixels. Returns
/// `None` when the platform query fails or the platform is unsupported —
/// callers should fall back to `Position::Centered`.
#[cfg(target_os = "windows")]
pub fn cursor_monitor_logical_center() -> Option<(f32, f32)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
    };
    use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    // SAFETY: every Win32 call below is documented to be safe to invoke from
    // any thread; pointers we hand in are stack-allocated and outlive the
    // call. Failures are handled by returning `None`.
    //
    // Assumes the process is Per-Monitor V2 DPI aware (winit/iced sets that
    // during init). Under PMv2, GetCursorPos / MonitorFromPoint / rcWork all
    // come back in physical pixels and dividing by per-monitor DPI yields the
    // logical coordinates iced expects. In other awareness modes rcWork is
    // pre-scaled and this math would over-correct on high-DPI displays.
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) == 0 {
            return None;
        }

        let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return None;
        }

        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return None;
        }

        // Per-monitor effective DPI — falls back to 96 (1.0× scale) if the
        // call fails. iced expects logical pixels = physical / scale.
        let mut dpi_x: u32 = 96;
        let mut dpi_y: u32 = 96;
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        let scale_x = dpi_x.max(96) as f32 / 96.0;
        let scale_y = dpi_y.max(96) as f32 / 96.0;

        let cx_phys = (info.rcWork.left + info.rcWork.right) as f32 / 2.0;
        let cy_phys = (info.rcWork.top + info.rcWork.bottom) as f32 / 2.0;
        Some((cx_phys / scale_x, cy_phys / scale_y))
    }
}

/// macOS implementation — `NSEvent::mouseLocation` returns logical points in
/// screen coordinates with the **bottom-left** origin anchored on the primary
/// (menu-bar) screen. We pick the screen whose `frame` contains that point,
/// take its `visibleFrame` (excludes menu bar + dock), and flip y to the
/// top-left origin that iced/winit expect. The flip pivots on
/// `screens()[0].frame.size.height` — winit uses the same anchor in
/// `flip_window_screen_coordinates`, so positions round-trip correctly.
#[cfg(target_os = "macos")]
pub fn cursor_monitor_logical_center() -> Option<(f32, f32)> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSEvent, NSScreen};

    let mtm = MainThreadMarker::new()?;
    let cursor = NSEvent::mouseLocation();
    let screens = NSScreen::screens(mtm);

    let containing = screens.iter().find(|screen| {
        let f = screen.frame();
        cursor.x >= f.origin.x
            && cursor.x < f.origin.x + f.size.width
            && cursor.y >= f.origin.y
            && cursor.y < f.origin.y + f.size.height
    })?;

    let visible = containing.visibleFrame();
    let primary_height = screens.iter().next()?.frame().size.height;

    let cx = visible.origin.x + visible.size.width / 2.0;
    let cy_macos = visible.origin.y + visible.size.height / 2.0;
    let cy_iced = primary_height - cy_macos;

    Some((cx as f32, cy_iced as f32))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn cursor_monitor_logical_center() -> Option<(f32, f32)> {
    None
}
