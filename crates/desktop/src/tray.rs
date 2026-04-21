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

use std::collections::HashMap;

use iced::futures::{SinkExt, Stream};
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

/// Iced-compatible stream that yields one [`TrayAction`] per qualifying
/// left-click on the tray icon. Non-matching tray events (right-click,
/// button-down, mouse-enter, …) are swallowed in the pump thread so they
/// never reach the subscription.
///
/// Bridges `tray-icon`'s global blocking receiver into iced via a dedicated
/// std thread + tokio channel, same shape as [`crate::menu::muda_event_stream`].
/// Use with [`iced::Subscription::run`] as a `fn` pointer.
pub fn click_stream() -> impl Stream<Item = TrayAction> {
    use iced::futures::channel::mpsc;
    use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};

    iced::stream::channel(16, |mut out: mpsc::Sender<_>| async move {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        std::thread::Builder::new()
            .name("tray-pump".into())
            .spawn(move || {
                while let Ok(event) = TrayIconEvent::receiver().recv() {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                        && tx.send(TrayAction::ToggleShowHide).is_err()
                    {
                        break;
                    }
                }
            })
            .expect("spawn tray pump thread");

        while let Some(action) = rx.recv().await {
            if out.send(action).await.is_err() {
                break;
            }
        }
    })
}

fn decode_icon() -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
    let img = image::load_from_memory(crate::assets::TRAY_ICON)?.to_rgba8();
    let (w, h) = img.dimensions();
    Ok(tray_icon::Icon::from_rgba(img.into_raw(), w, h)?)
}
