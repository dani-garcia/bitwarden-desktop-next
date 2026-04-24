//! `CipherDetailMessage` — messages emitted by the read-only cipher detail
//! pane. Parallel to [`super::super::cipher_edit::CipherEditMessage`].

#[derive(Debug, Clone)]
pub enum CipherDetailMessage {
    Close,
    CopyUsername,
    CopyPassword,
    /// Copy the URI at the given index in `login.uris`. A login may have
    /// multiple URIs; the autofill section renders one button row per URI.
    CopyUrl(usize),
    CopyTotp,
    /// Open the URI at the given index in `login.uris`. See `CopyUrl`.
    OpenUrl(usize),
    /// Copy the value of the custom field at the given index in
    /// `cipher.fields`. Only wired for hidden-type fields today; plain
    /// text fields can still be selected and copied manually.
    CopyCustomField(usize),
    CopySshPrivateKey,
    CopySshPublicKey,
    CopySshFingerprint,
    Edit,
    /// Trash icon pressed — opens the confirm modal (handled at the vault
    /// view layer). Does not actually delete by itself.
    Delete,
}
