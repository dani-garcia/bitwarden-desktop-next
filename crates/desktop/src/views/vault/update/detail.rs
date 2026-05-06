//! Read-only cipher detail handlers: clipboard ops, detail-loaded.

use bitwarden_vault::{CipherId, CipherView};

use crate::{
    app::{Outcome, UpdateCtx},
    debug_fmt::NoDebug,
    domain::UserId,
    fl,
    services::clipboard::Sensitivity,
};

use crate::views::vault::{
    VaultEvent, state::VaultView, widgets::cipher_detail::CipherDetailMessage,
};

use super::list::clipboard_outcome;

impl VaultView {
    pub(super) fn handle_cipher_detail(
        &mut self,
        ctx: &UpdateCtx<'_>,
        msg: CipherDetailMessage,
    ) -> Outcome<Self> {
        match msg {
            CipherDetailMessage::Edit => self.handle_detail_edit(ctx),
            CipherDetailMessage::CopyUsername => clipboard_outcome(
                self.selected_login()
                    .and_then(|l| l.username.as_deref())
                    .map(str::to_owned),
                Sensitivity::Normal,
                fl!("vault-toast-copied-username"),
            ),
            CipherDetailMessage::CopyPassword => clipboard_outcome(
                self.selected_login()
                    .and_then(|l| l.password.as_deref())
                    .map(str::to_owned),
                Sensitivity::Sensitive,
                fl!("vault-toast-copied-password"),
            ),
            CipherDetailMessage::CopyUrl(idx) => clipboard_outcome(
                self.selected_login_uri(idx).map(str::to_owned),
                Sensitivity::Normal,
                fl!("vault-toast-copied-website"),
            ),
            CipherDetailMessage::OpenUrl(idx) => {
                Outcome::from_option(self.selected_login_uri(idx).map(|uri| {
                    VaultEvent::LaunchUrlRequested {
                        uri: uri.to_owned(),
                    }
                }))
            }
            CipherDetailMessage::CopyTotp => {
                // Recompute the code at the moment of copy so the clipboard
                // holds a value still valid for ~30 s.
                let code = self
                    .selected_login()
                    .and_then(|l| l.totp.as_deref())
                    .map(str::to_owned)
                    .and_then(|s| bitwarden_vault::generate_totp(s, None).ok().map(|r| r.code));
                clipboard_outcome(code, Sensitivity::Sensitive, fl!("vault-toast-copied-totp"))
            }
            CipherDetailMessage::CopySshPrivateKey => clipboard_outcome(
                self.selected_ssh_key().map(|k| k.private_key.clone()),
                Sensitivity::Sensitive,
                fl!("vault-toast-copied-private-key"),
            ),
            CipherDetailMessage::CopySshPublicKey => clipboard_outcome(
                self.selected_ssh_key().map(|k| k.public_key.clone()),
                Sensitivity::Normal,
                fl!("vault-toast-copied-public-key"),
            ),
            CipherDetailMessage::CopySshFingerprint => clipboard_outcome(
                self.selected_ssh_key().map(|k| k.fingerprint.clone()),
                Sensitivity::Normal,
                fl!("vault-toast-copied-fingerprint"),
            ),
            CipherDetailMessage::CopyCustomField(idx) => {
                let Some(field) = self.selected_field(idx) else {
                    return Outcome::None;
                };
                let Some(value) = field.value.as_deref().map(str::to_owned) else {
                    return Outcome::None;
                };
                // Hidden fields are sensitive (auto-clear after timeout);
                // text/boolean are treated like a copied username.
                let sensitivity = match field.r#type {
                    bitwarden_vault::FieldType::Hidden => Sensitivity::Sensitive,
                    _ => Sensitivity::Normal,
                };
                clipboard_outcome(Some(value), sensitivity, fl!("vault-toast-copied-field"))
            }
            CipherDetailMessage::Delete => {
                // Open the confirm modal. Actual delete waits for the user
                // to press Confirm (`ConfirmDeleteSelected`).
                self.selection.confirm_delete.open();
                Outcome::None
            }
            // Close is intercepted at the caller's `.map()` and never reaches
            // this match — kept for exhaustiveness.
            CipherDetailMessage::Close => Outcome::None,
        }
    }

    pub(super) fn handle_detail_loaded(
        &mut self,
        ctx: &UpdateCtx<'_>,
        msg_uid: UserId,
        id: CipherId,
        result: Result<NoDebug<Box<CipherView>>, String>,
    ) -> Outcome<Self> {
        // Stale-check: user switched while full_cipher was in flight.
        if !ctx.is_active_user(&msg_uid) {
            tracing::debug!(
                uid = %msg_uid,
                cipher_id = %id,
                "full_cipher result dropped: active user changed while in flight"
            );
            return Outcome::None;
        }
        match result {
            Ok(NoDebug(view)) => {
                // Stale-check on the cipher id itself: if the user clicked
                // a different item between the perform and the callback,
                // drop the stale detail.
                if self.selection.id == view.id {
                    self.selection.detail = Some(*view);
                    self.selection.sheet_fade.open();
                    self.pane.open();
                } else {
                    tracing::debug!(
                        cipher_id = %id,
                        "full_cipher result dropped: selection changed while in flight"
                    );
                }
            }
            Err(err) => {
                tracing::error!(cipher_id = %id, %err, "full_cipher failed");
                return Outcome::toast(crate::components::toast::Toast::error(
                    fl!("vault-toast-decrypt-failed-body"),
                    Some(&fl!("vault-toast-decrypt-failed-title")),
                ));
            }
        }
        Outcome::None
    }
}
