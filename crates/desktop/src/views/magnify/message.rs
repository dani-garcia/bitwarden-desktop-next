//! Top-level message variant for the Magnify launcher.
//!
//! Routed at the `App` level via `Message::Magnify(...)`; there's no
//! `UpdateCtx` plumbing because Magnify lives in its own window and reads
//! cipher data directly off `App` (active user + vault item cache).

use bitwarden_vault::CipherId;

use crate::domain::UserId;

#[derive(Debug, Clone)]
pub enum MagnifyMessage {
    /// Global hotkey fired (`Ctrl+Shift+Space` / `Cmd+Shift+Space`).
    /// Toggles the launcher window between hidden and visible, opening it
    /// the first time.
    HotkeyPressed,
    /// Launcher window finished opening (first summon only).
    WindowOpened(iced::window::Id),
    QueryChanged(String),
    NavigateUp,
    NavigateDown,
    RowClicked(usize),
    /// `Ctrl+C` — kick off async decrypt of the selected cipher's password.
    CopyPasswordRequested,
    /// Continuation of `CopyPasswordRequested` — full cipher decrypted.
    /// `uid` is captured at request time so a slow decrypt finishing after
    /// the active user changed gets dropped instead of leaking across users.
    PasswordDecryptCompleted(UserId, CipherId, Result<Option<String>, String>),
    /// `Ctrl+Shift+C` — copy the selected item's username (already in the
    /// `CipherListView` summary, no decrypt needed).
    CopyUsernameRequested,
    /// `Esc` or click-outside — hide the launcher.
    Hide,
    /// Locked-state pill button — bring the main window forward and route
    /// to the unlock screen for the active user.
    OpenMainWindow,
    /// Results-list scrolled (mouse wheel / drag). Updates the cached
    /// scroll offset so arrow navigation can decide whether the selected
    /// row is still on-screen.
    Scrolled(iced::widget::scrollable::Viewport),
}
