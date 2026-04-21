//! System-tray icon + context menu.
//!
//! Uses the `tray-icon` crate (sister crate to `muda`). The tray's menu is
//! built from `tray_icon::menu::*` (muda re-exported) so menu clicks land on
//! the same `MenuEvent::receiver()` the native app menu already polls — see
//! [`poll_menu_action`].
//!
//! ## Linux runtime requirement
//!
//! On Linux, `tray-icon` requires **`libayatana-appindicator3-1`** (apt/.deb)
//! or a `StatusNotifierItem` host in the desktop environment (KDE ships one;
//! GNOME needs the AppIndicator extension). If neither is present, [`build`]
//! returns `None` and the tray-adjacent settings become no-ops. Package the
//! dep in installer scripts before shipping a Linux build.
//!
//! ## macOS
//!
//! On macOS the icon is a **template image** (monochrome) so NSStatusBar can
//! recolour it for light and dark menubar modes. We flip the `icon_is_template`
//! builder flag accordingly.

use std::{collections::HashMap, sync::OnceLock};

use iced::futures::{SinkExt, Stream};
use tokio::sync::broadcast;
use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuId, MenuItem},
};

/// Actions the tray can emit — resolved from either a menu-item click
/// (via muda's `MenuEvent`) or a left-click on the icon itself (via
/// `tray_icon::TrayIconEvent`).
#[derive(Clone, Copy, Debug)]
pub enum TrayAction {
    ToggleShowHide,
    LockVault,
    Exit,
}

pub struct TrayHandle {
    // Retained: dropping would remove the OS tray entry.
    _icon: TrayIcon,
    actions: HashMap<MenuId, TrayAction>,
}

impl TrayHandle {
    /// Resolve a muda `MenuId` to a `TrayAction` if it belongs to the tray
    /// menu. Returns `None` for unrelated ids. The caller is responsible for
    /// draining `muda::MenuEvent::receiver()` — a global singleton shared
    /// between the tray menu and the app's native menu — and dispatching
    /// by id.
    pub fn resolve(&self, id: &MenuId) -> Option<TrayAction> {
        self.actions.get(id).copied()
    }
}

/// Build the tray icon + context menu. Returns `None` on failure (e.g. Linux
/// without an SNI host). The caller logs a warning and continues without a
/// tray — the tray-dependent settings are all checked against `Option::is_some`.
pub fn build() -> Option<TrayHandle> {
    let icon = match decode_icon() {
        Ok(i) => i,
        Err(e) => {
            tracing::warn!(error = %e, "failed to decode tray icon");
            return None;
        }
    };

    let show_hide = MenuItem::new(crate::fl!("tray-show-hide"), true, None);
    let lock_vault = MenuItem::new(crate::fl!("tray-lock-vault"), true, None);
    let exit = MenuItem::new(crate::fl!("tray-exit"), true, None);
    let sep = tray_icon::menu::PredefinedMenuItem::separator();

    let menu = match Menu::with_items(&[&show_hide, &sep, &lock_vault, &exit]) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, "failed to assemble tray menu");
            return None;
        }
    };

    let mut actions = HashMap::new();
    actions.insert(show_hide.id().clone(), TrayAction::ToggleShowHide);
    actions.insert(lock_vault.id().clone(), TrayAction::LockVault);
    actions.insert(exit.id().clone(), TrayAction::Exit);

    let builder = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("Bitwarden")
        .with_menu_on_left_click(false);

    // macOS template image — NSStatusBar recolours for light/dark menubar.
    #[cfg(target_os = "macos")]
    let builder = builder.with_icon_as_template(true);

    let tray = match builder.build() {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(error = %e, "tray icon initialisation failed; tray features disabled");
            return None;
        }
    };

    Some(TrayHandle {
        _icon: tray,
        actions,
    })
}

/// Broadcast fan-out for filtered tray-icon clicks. Installed once by
/// [`install_event_handler`]; a click that matches the "left button released"
/// filter is published as a [`TrayAction::ToggleShowHide`] for subscribers to
/// pick up. Non-matching events (right-click, enter/leave, button-down) are
/// dropped in the handler.
static TRAY_EVENTS: OnceLock<broadcast::Receiver<TrayAction>> = OnceLock::new();

/// Register `tray-icon`'s global event handler. Call once at startup, before
/// any tray icon is built. Subsequent calls are ignored (both `OnceLock::set`
/// and `tray-icon`'s own `OnceCell`-backed `set_event_handler` silently
/// no-op after the first).
pub fn install_event_handler() {
    use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};

    let (tx, rx) = broadcast::channel(16);
    let _ = TRAY_EVENTS.set(rx);
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            let _ = tx.send(TrayAction::ToggleShowHide);
        }
    }));
}

/// Iced-compatible stream of [`TrayAction`]s derived from tray-icon clicks.
/// Subscribes to the static broadcast channel populated by the handler
/// installed in [`install_event_handler`].
///
/// Use as a `fn` pointer with [`iced::Subscription::run`]; when iced drops
/// the subscription the receiver drops cleanly — no stranded threads.
pub fn click_stream() -> impl Stream<Item = TrayAction> {
    use iced::futures::channel::mpsc;
    iced::stream::channel(16, |mut out: mpsc::Sender<_>| async move {
        let mut rx = TRAY_EVENTS
            .get()
            .expect("tray::install_event_handler must run before subscribing")
            .resubscribe();
        loop {
            match rx.recv().await {
                Ok(action) => {
                    if out.send(action).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(dropped = n, "tray event subscriber lagged");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

fn decode_icon() -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
    let img = image::load_from_memory(crate::assets::TRAY_ICON)?.to_rgba8();
    let (w, h) = img.dimensions();
    Ok(tray_icon::Icon::from_rgba(img.into_raw(), w, h)?)
}
