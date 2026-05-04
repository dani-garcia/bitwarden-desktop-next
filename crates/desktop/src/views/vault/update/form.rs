//! Cipher form handlers: new-item, edit, save, delete, form-options-loaded.

use bitwarden_vault::{CipherId, CipherView};
use iced::Task;

use crate::{
    app::{Outcome, UpdateCtx},
    components::toast::Toast,
    debug_fmt::NoDebug,
    domain::UserId,
    fl,
    services::sdk::ClientExt,
};

use crate::views::vault::{
    VaultEvent, VaultMessage,
    message::FormOptions,
    state::VaultView,
    widgets::cipher_edit::{CipherEditMessage, CipherForm, FolderOption, FormEvent},
};

impl VaultView {
    pub(super) fn handle_detail_edit(&mut self, ctx: &UpdateCtx<'_>) -> Outcome<Self> {
        let Some(detail) = self.selection.detail.clone() else {
            return Outcome::None;
        };
        let Some(uid) = ctx.active_user.copied() else {
            return Outcome::None;
        };
        self.selection.form = Some(CipherForm::edit(detail));
        Outcome::task(Self::load_form_options_task(ctx, uid))
    }

    pub(super) fn handle_new_item(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        cipher_type: bitwarden_vault::CipherType,
    ) -> Outcome<Self> {
        // Auto-dismiss the +New dropdown if it was the trigger.
        if *ctx.open_overlay == Some(crate::app::Overlay::NewItemMenu) {
            *ctx.open_overlay = None;
        }
        let Some(uid) = ctx.active_user.copied() else {
            return Outcome::None;
        };
        // Drop any prior selection so the right pane shows only the new
        // empty form — no stale read-only detail content peeking through.
        self.selection.clear();
        self.selection.form = Some(CipherForm::new(cipher_type, None));
        self.pane.open();
        self.selection.sheet_fade.open();

        Outcome::task(Task::batch([
            Self::load_form_options_task(ctx, uid),
            Self::focus_name_input_task(),
        ]))
    }

    /// Build the task that loads folders/orgs/collections for the cipher
    /// form. Shared by edit-mode and create-mode entry points; folders is
    /// the only async leg, the other two are sync reads we capture upfront.
    fn load_form_options_task(ctx: &UpdateCtx<'_>, uid: UserId) -> Task<VaultMessage> {
        let Some(client) = ctx.client_manager.client_for(&uid) else {
            return Task::none();
        };
        let organizations = ctx.client_manager.list_organizations(&uid);
        let collections = ctx.client_manager.list_collections(&uid);
        Task::perform(
            async move {
                let folders = client
                    .list_folders()
                    .await
                    .map(|folders| folders.into_iter().map(FolderOption::from).collect());
                FormOptions {
                    folders,
                    organizations,
                    collections,
                }
            },
            move |opts| VaultMessage::FormOptionsLoaded(uid, NoDebug(opts)),
        )
    }

    pub(super) fn handle_confirm_delete(&mut self, ctx: &UpdateCtx<'_>) -> Outcome<Self> {
        self.selection.confirm_delete.close();
        let Some(cipher_id) = self.selection.id else {
            return Outcome::None;
        };
        ctx.perform_with_active_client(
            move |client| client.soft_delete_cipher(cipher_id),
            move |uid, res| VaultMessage::DeleteCompleted(uid, cipher_id, res),
        )
    }

    pub(super) fn handle_cipher_edit(
        &mut self,
        ctx: &UpdateCtx<'_>,
        msg: CipherEditMessage,
    ) -> Outcome<Self> {
        let Some(form) = self.selection.form.as_mut() else {
            return Outcome::None;
        };
        match form.update(msg) {
            FormEvent::None => Outcome::None,
            FormEvent::Cancel => {
                self.selection.form = None;
                // No detail to fall back to (new-item flow) — close the pane
                // so the right side doesn't linger as an empty column. Edit
                // flows leave detail populated and the read-only pane shows.
                if self.selection.detail.is_none() {
                    let task = self
                        .selection
                        .sheet_fade
                        .close_with_finalize(VaultMessage::FinalizeSheetClose);
                    self.pane.close();
                    return Outcome::task(task);
                }
                Outcome::None
            }
            FormEvent::Save => {
                if !form.is_valid() {
                    return Outcome::toast(Toast::warning(fl!("toast-required-fields"), None));
                }
                form.saving = true;
                let cipher_view = form.modified.clone();
                ctx.perform_with_active_client(
                    move |client| client.save_cipher(cipher_view),
                    move |uid, res| {
                        VaultMessage::SaveCompleted(uid, res.map(|v| NoDebug(Box::new(v))))
                    },
                )
            }
        }
    }

    pub(super) fn handle_form_options_loaded(
        &mut self,
        ctx: &UpdateCtx<'_>,
        msg_uid: UserId,
        opts: FormOptions,
    ) -> Outcome<Self> {
        if !ctx.is_active_user(&msg_uid) {
            return Outcome::None;
        }
        let Some(form) = self.selection.form.as_mut() else {
            return Outcome::None;
        };
        match opts.folders {
            Ok(folders) => form.set_folders(folders),
            Err(err) => tracing::error!(%err, "list_folders failed"),
        }
        form.set_organizations(opts.organizations);
        form.collections = opts.collections;
        Outcome::None
    }

    pub(super) fn handle_save_completed(
        &mut self,
        ctx: &UpdateCtx<'_>,
        msg_uid: UserId,
        result: Result<NoDebug<Box<CipherView>>, String>,
    ) -> Outcome<Self> {
        if !ctx.is_active_user(&msg_uid) {
            return Outcome::None;
        }
        match result {
            Ok(NoDebug(view)) => {
                // For new-item saves `selection.id` was None until now; sync
                // it with the saved cipher so the upcoming list reload's
                // selection-tracking has something to match against.
                self.selection.id = view.id;
                self.selection.detail = Some(*view);
                self.selection.form = None;
                self.selection.sheet_fade.open();
                Outcome::event(VaultEvent::ItemSaved { uid: msg_uid })
            }
            Err(err) => {
                tracing::error!(%err, "save_cipher failed");
                if let Some(form) = self.selection.form.as_mut() {
                    form.saving = false;
                }
                Outcome::toast(Toast::error(
                    fl!("vault-toast-save-failed-body"),
                    Some(&fl!("vault-toast-save-failed-title")),
                ))
            }
        }
    }

    pub(super) fn handle_delete_completed(
        &mut self,
        ctx: &UpdateCtx<'_>,
        msg_uid: UserId,
        cipher_id: CipherId,
        result: Result<(), String>,
    ) -> Outcome<Self> {
        if !ctx.is_active_user(&msg_uid) {
            return Outcome::None;
        }
        match result {
            Ok(()) => {
                self.selection.clear();
                self.pane.close();
                Outcome::event(VaultEvent::ItemDeleted { uid: msg_uid })
            }
            Err(err) => {
                tracing::error!(cipher_id = %cipher_id, %err, "soft_delete_cipher failed");
                Outcome::toast(Toast::error(
                    fl!("vault-toast-delete-failed-body"),
                    Some(&fl!("vault-toast-delete-failed-title")),
                ))
            }
        }
    }
}
