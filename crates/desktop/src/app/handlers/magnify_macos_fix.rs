//! macOS magnify-window transparency. Two independent fixes — both are
//! needed because each addresses a different broken path on macOS:
//!
//! With `transparent: true` + `decorations: false`, the NSWindow IS made
//! non-opaque by winit, but the renderer's contents reach the screen through
//! a CALayer that doesn't honor that:
//!
//! - **wgpu** path: `CAMetalLayer.isOpaque` defaults to `true`. wgpu only
//!   calls `set_opaque(false)` when `CompositeAlphaMode::PostMultiplied` is
//!   requested, but iced's wgpu compositor selects `PreMultiplied` (or
//!   `Auto`), so the layer stays opaque and alpha-0 pixels render as black.
//!
//! - **tiny_skia** path (via softbuffer): the framebuffer is presented as a
//!   `CGImage` with `CGImageAlphaInfo::NoneSkipFirst` — alpha is discarded
//!   before reaching any layer. Setting `isOpaque = false` on the layer
//!   can't recover an alpha channel that was never sent.
//!
//! ## Fixes
//!
//! - [`apply_opaque_fix`]: walk the contentView's CALayer subtree and set
//!   `isOpaque = false` on each. Targets the wgpu `CAMetalLayer` sublayer so
//!   the OS composites alpha. Does not help the tiny_skia path.
//!
//! - [`apply_corner_fix`]: set `cornerRadius` + `masksToBounds = true` on the
//!   contentView's layer. The OS clips everything outside the rounded shape
//!   at the compositor level, so pixels never reach the screen regardless of
//!   what the renderer drew. Required for the tiny_skia path; harmless on
//!   wgpu where it just lines up with the iced-side rounded border.
//!
//! Both fixes rely on the NSWindow already being non-opaque (winit sets that
//! up via `transparent: true`).

#![cfg(target_os = "macos")]

use iced::window::raw_window_handle::RawWindowHandle;
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use objc2_quartz_core::CALayer;

/// Apply both fixes to the magnify window's contentView. Safe to call
/// multiple times — every setter touched is idempotent.
pub fn apply(window: &dyn iced::window::Window, corner_radius: f32) {
    let Ok(window_handle) = window.window_handle() else {
        tracing::warn!("magnify_macos_fix: failed to acquire window handle");
        return;
    };
    let RawWindowHandle::AppKit(handle) = window_handle.as_raw() else {
        tracing::warn!("magnify_macos_fix: not an AppKit window handle");
        return;
    };

    // SAFETY: `AppKitWindowHandle::ns_view` is documented as a valid pointer
    // to an NSView (winit's contentView). `iced::window::run` invokes us on
    // the main thread, which is required for AppKit access.
    let view: Retained<NSView> =
        unsafe { Retained::retain(handle.ns_view.as_ptr().cast()).expect("ns_view was null") };
    let Some(layer) = view.layer() else {
        tracing::warn!("magnify_macos_fix: NSView has no backing layer");
        return;
    };

    apply_opaque_fix(&layer);
    apply_corner_fix(&layer, corner_radius);
}

/// `isOpaque = false` on the contentView's layer and every sublayer
/// underneath. The recursion catches the wgpu `CAMetalLayer` regardless of
/// where it sits in the tree (wgpu inserts it as a sublayer of the root via
/// `new_observer_layer`, but that's an implementation detail).
fn apply_opaque_fix(layer: &CALayer) {
    layer.setOpaque(false);
    // SAFETY: `-[CALayer sublayers]` returns `nil` or an `NSArray<CALayer*>`.
    // The array is borrowed for the duration of iteration; `setOpaque` is a
    // simple property setter with no aliasing concerns.
    if let Some(sublayers) = unsafe { layer.sublayers() } {
        for sub in sublayers.iter() {
            apply_opaque_fix(&sub);
        }
    }
}

/// `cornerRadius` + `masksToBounds` on the contentView's layer. The OS
/// applies the rounded clip to all sublayers automatically.
fn apply_corner_fix(layer: &CALayer, corner_radius: f32) {
    layer.setCornerRadius(corner_radius as _);
    layer.setMasksToBounds(true);
}
