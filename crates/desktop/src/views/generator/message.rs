//! `GeneratorMessage` (view-internal) and `GeneratorEvent` (cross-cutting,
//! routed by App) plus the [`GenerateKind`] payload App needs to dispatch
//! the right SDK call.

use bitwarden_generators::{
    PassphraseGeneratorRequest, PasswordGeneratorRequest, UsernameGeneratorRequest,
};

use super::state::{TabKind, UsernameKind};

#[derive(Debug, Clone)]
pub enum GeneratorMessage {
    Close,
    SelectTab(TabKind),
    ShowHistory,
    BackToGenerator,
    ClearHistory,
    CopyCurrent,
    CopyHistoryEntry(usize),
    Regenerate,
    // Password
    SetLength(String),
    BumpLength(i32),
    ToggleLowercase(bool),
    ToggleUppercase(bool),
    ToggleNumbers(bool),
    ToggleSpecial(bool),
    SetMinNumber(String),
    BumpMinNumber(i32),
    SetMinSpecial(String),
    BumpMinSpecial(i32),
    ToggleAvoidAmbiguous(bool),
    // Passphrase
    SetNumWords(String),
    BumpNumWords(i32),
    SetWordSeparator(String),
    TogglePassphraseCapitalize(bool),
    TogglePassphraseIncludeNumber(bool),
    // Username
    SelectUsernameKind(UsernameKind),
    ToggleUsernameCapitalize(bool),
    ToggleUsernameIncludeNumber(bool),
    SetEmail(String),
    SetDomain(String),
    // Async round-trip from the SDK. Carries just the value; the view's
    // update commits it into `ClientManager::password_history` via the
    // `&mut ClientManager` on `UpdateCtx` and stashes the snapshot.
    Generated(Result<String, String>),
}

/// Events bubbled up to App. App is responsible for actually running the
/// SDK call (needs the per-user handle from `ClientManager::client_for`) and
/// for the clipboard/toast side effects.
pub enum GeneratorEvent {
    /// Regenerate the output for the active tab. App spawns a Task that
    /// calls `ClientManager::generate_*` and pipes the result back through
    /// [`GeneratorMessage::Generated`].
    Generate(GenerateKind),
    ClearHistory,
    Copy(String),
}

/// The concrete request payload for a regeneration. Built by the view from
/// its form state so App doesn't need to peek at private fields.
pub enum GenerateKind {
    Password(PasswordGeneratorRequest),
    Passphrase(PassphraseGeneratorRequest),
    Username(UsernameGeneratorRequest),
}
