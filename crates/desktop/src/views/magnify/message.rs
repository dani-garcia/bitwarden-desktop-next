//! Top-level message variant for the Magnify launcher.
//!
//! Routed at the `App` level via `Message::Magnify(...)`; there's no
//! `UpdateCtx` plumbing because Magnify lives in its own window and reads
//! cipher data directly off `App` (active user + vault item cache).

use bitwarden_vault::CipherId;

use crate::domain::UserId;

use super::state::CopyField;

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
    /// One of the Ctrl-key copy shortcuts that needs an async cipher decrypt
    /// (Password = Ctrl+C, Totp = Ctrl+T, Notes = Ctrl+Shift+N). The handler
    /// stamps `pending_decrypt` and runs `full_cipher`.
    CopyFieldRequested(CopyField),
    /// Continuation of [`CopyFieldRequested`] — full cipher decrypted.
    /// `uid` and `field` are captured at request time so a slow decrypt
    /// finishing after the active user changed (or after a different field
    /// shortcut superseded this one) gets dropped instead of leaking the
    /// secret across users / fields.
    FieldDecryptCompleted(UserId, CipherId, CopyField, Result<Option<String>, String>),
    /// `Ctrl+Shift+C` — copy the selected item's username (already in the
    /// `CipherListView` summary, no decrypt needed).
    CopyUsernameRequested,
    /// `Ctrl+U` — copy the selected item's first URI (already in the
    /// `CipherListView` summary, no decrypt needed).
    CopyUriRequested,
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
