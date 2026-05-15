//! `VaultView::update` dispatch + the `load_list_task` factory.
//!
//! `update()` is a thin dispatch into `handle_*` methods that each own one
//! branch of the message tree. Per-domain handlers live in:
//!
//! - [`list`] — the cipher list, filter recomputation, list-loaded.
//! - [`detail`] — read-only detail pane: clipboard ops, detail-loaded.
//! - [`form`] — edit/new form, save/delete, form-options-loaded.

mod detail;
mod form;
mod list;

use std::sync::Arc;

use bitwarden_vault::{FieldView, LoginView, SshKeyView};
use iced::{Element, Task};

use crate::{
    app::{Outcome, Overlay, RenderCtx, UpdateCtx, View},
    debug_fmt::Summary,
    domain::UserId,
    services::sdk::{ClientExt, ClientManager},
    theme::AppTheme,
};

use super::{
    VaultEvent, VaultFilter, VaultMessage,
    message::ForUserMessage,
    state::VaultView,
    widgets::{cipher_detail, search_bar::SearchMessage},
};

// ── Task factory ───────────────────────────────────────────────────────────

impl VaultView {
    /// Build the task that decrypts the user's vault list and lands as
    /// `VaultMessage::ListLoaded`. Called from App handlers (unlock, user
    /// switch, sync) — the factory lives here so all `Task::perform` calls
    /// that produce `VaultMessage`s stay within the owning view.
    pub fn load_list_task(uid: UserId, mgr: &ClientManager) -> Task<VaultMessage> {
        let Some(client) = mgr.client_for(&uid) else {
            return Task::done(VaultMessage::ListLoaded(
                uid,
                Err(format!("unknown user {uid}")),
            ));
        };
        Task::perform(
            async move {
                client
                    .list_ciphers()
                    .await
                    .map(|items| items.into_iter().map(Arc::new).collect::<Vec<_>>())
            },
            move |result| VaultMessage::ListLoaded(uid, result.map(Summary)),
        )
    }
}

// ── Update dispatch ────────────────────────────────────────────────────────

impl View for VaultView {
    type Message = VaultMessage;
    type Event = VaultEvent;

    /// Compositional MVU update. Returns a task (for async work the view
    /// owns) and an optional event (cross-cutting fact for App to route).
    ///
    /// `ctx` carries the SDK handle + active user; it's built fresh on every
    /// `App::update` call so the view can construct `Task::perform` calls
    /// without owning shared state.
    fn update(&mut self, msg: VaultMessage, mut ctx: UpdateCtx<'_>) -> Outcome<Self> {
        match msg {
            VaultMessage::ItemList(m) => {
                return self.handle_item_list(&ctx, m);
            }
            VaultMessage::Search(SearchMessage::QueryChanged(query)) => {
                self.search_query = query;
                if let Some(uid) = ctx.active_user {
                    self.recompute_filtered(uid, ctx.active_vault_filter);
                }
            }
            VaultMessage::CloseCipherDetail => {
                // Start the outro and defer the selection clear so the
                // sheet content stays alive while the slide animates out.
                // The wide-mode pane closes immediately — it has no
                // animation to wait on.
                let task = self
                    .selection
                    .sheet_fade
                    .close_with_finalize(VaultMessage::FinalizeSheetClose);
                self.pane.close();
                return Outcome::task(task);
            }
            VaultMessage::FinalizeSheetClose => {
                self.selection.clear();
            }
            VaultMessage::PaneResized(event) => self.pane.set_ratio(event.ratio),
            VaultMessage::CipherDetail(m) => {
                return self.handle_cipher_detail(&ctx, m);
            }
            VaultMessage::CancelDeleteSelected => self.selection.confirm_delete.close(),
            VaultMessage::ConfirmDeleteSelected => {
                return self.handle_confirm_delete(&ctx);
            }
            VaultMessage::CipherEdit(m) => {
                return self.handle_cipher_edit(&ctx, m);
            }
            VaultMessage::AccountSwitcher(m) => {
                return m.route(&mut *ctx.open_overlay, Overlay::AccountSwitcher);
            }
            VaultMessage::OpenNewItemPicker => {
                return Outcome::event(VaultEvent::OpenNewItemPicker);
            }
            VaultMessage::NewItem(t) => {
                return self.handle_new_item(&mut ctx, t);
            }
            VaultMessage::ListLoaded(uid, res) => {
                return self.handle_list_loaded(&ctx, uid, res);
            }
            VaultMessage::ForUser(msg_uid, inner) => {
                // Single stale-user guard for every SDK completion: drop the
                // result if the active user changed during the await.
                if !ctx.is_active_user(&msg_uid) {
                    tracing::debug!(uid = %msg_uid, ?inner, "vault result dropped: active user changed");
                    return Outcome::None;
                }
                return match inner {
                    ForUserMessage::DetailLoaded(id, res) => {
                        self.handle_detail_loaded(&ctx, msg_uid, id, res)
                    }
                    ForUserMessage::FormOptionsLoaded(crate::debug_fmt::NoDebug(opts)) => {
                        self.handle_form_options_loaded(&ctx, msg_uid, opts)
                    }
                    ForUserMessage::SaveCompleted(res) => {
                        self.handle_save_completed(&ctx, msg_uid, res)
                    }
                    ForUserMessage::DeleteCompleted(id, res) => {
                        self.handle_delete_completed(&ctx, msg_uid, id, res)
                    }
                };
            }
            VaultMessage::AutoFocusSearchDelayed => {
                return Outcome::task(self.auto_focus_task());
            }
        }
        Outcome::None
    }

    fn view<'a>(&'a self, ctx: &RenderCtx<'a>) -> Element<'a, VaultMessage, AppTheme> {
        self.render(ctx)
    }

    /// Bottom-sheet (narrow-mode cipher detail) and the delete-confirm
    /// sub-modal, in z-order: sheet first (lower), confirm on top.
    fn overlays<'a>(&'a self, ctx: &RenderCtx<'a>) -> Vec<Element<'a, VaultMessage, AppTheme>> {
        crate::components::list_pane::overlays(self.render_sheet(ctx), self.render_overlay(ctx))
    }
}

impl VaultView {
    /// Called by App when the active vault filter changes. Clears the
    /// current selection (what's selected may no longer be in the list) and
    /// recomputes the cached list for the active user.
    pub fn apply_filter(&mut self, uid: &UserId, filter: VaultFilter) {
        self.selection.clear();
        self.pane.close();
        self.recompute_filtered(uid, filter);
    }
}

// ── Selection accessors ────────────────────────────────────────────────────
//
// Shared walk of `self.selection.detail.as_ref()` into the login / fields
// sub-objects. Removes the repeated four-line chain from every handler arm
// that needs a value to copy.

impl VaultView {
    pub(super) fn selected_login(&self) -> Option<&LoginView> {
        self.selection.detail.as_ref()?.login.as_ref()
    }

    pub(super) fn selected_field(&self, idx: usize) -> Option<&FieldView> {
        self.selection.detail.as_ref()?.fields.as_deref()?.get(idx)
    }

    pub(super) fn selected_login_uri(&self, idx: usize) -> Option<&str> {
        cipher_detail::login_uri_at(self.selected_login()?, idx)
    }

    pub(super) fn selected_ssh_key(&self) -> Option<&SshKeyView> {
        self.selection.detail.as_ref()?.ssh_key.as_ref()
    }
}

#[cfg(test)]
mod tests_sheet_close {
    use super::*;
    use crate::test_support::{OutcomeExt, ViewTestExt};
    use bitwarden_vault::{CipherRepromptType, CipherType, CipherView};
    use chrono::Utc;

    fn dummy_cipher_view() -> CipherView {
        let now = Utc::now();
        CipherView {
            id: None,
            organization_id: None,
            folder_id: None,
            collection_ids: Vec::new(),
            key: None,
            name: "test".to_string(),
            notes: None,
            r#type: CipherType::SecureNote,
            login: None,
            identity: None,
            card: None,
            secure_note: None,
            ssh_key: None,
            bank_account: None,
            drivers_license: None,
            passport: None,
            favorite: false,
            reprompt: CipherRepromptType::None,
            organization_use_totp: false,
            edit: true,
            permissions: None,
            view_password: true,
            local_data: None,
            attachments: None,
            attachment_decryption_failures: None,
            fields: None,
            password_history: None,
            creation_date: now,
            deleted_date: None,
            revision_date: now,
            archived_date: None,
        }
    }

    fn view_with_open_sheet() -> VaultView {
        let mut view = VaultView::new();
        view.selection.detail = Some(dummy_cipher_view());
        view.selection.item = Some(0);
        view.selection.sheet_fade.open();
        view
    }

    #[tokio::test(flavor = "current_thread")]
    async fn close_cipher_detail_starts_sheet_outro_but_keeps_selection() {
        // The two-message close exists so the bottom-sheet animation has
        // content to render during its slide-down. Asserting that
        // `selection` survives `CloseCipherDetail` pins this contract —
        // a refactor that cleared selection here would leave the
        // animation playing against an empty sheet.
        let mut view = view_with_open_sheet();
        assert!(view.selection.detail.is_some());
        assert!(view.selection.sheet_fade.is_open());

        let _outcome = view.run(VaultMessage::CloseCipherDetail).await;

        // sheet_fade.is_open() flips false the moment close starts.
        assert!(!view.selection.sheet_fade.is_open());
        // Selection is still populated — outro hasn't finalized yet.
        assert!(
            view.selection.detail.is_some(),
            "detail kept alive for the outro frame"
        );
        assert_eq!(view.selection.item, Some(0));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn finalize_sheet_close_clears_selection() {
        // The second half of the two-message close: this is what the
        // delayed Task dispatches once the outro duration elapses.
        let mut view = view_with_open_sheet();
        view.run(VaultMessage::CloseCipherDetail).await;
        // After the outro:
        view.run(VaultMessage::FinalizeSheetClose)
            .await
            .expect_none();

        assert!(view.selection.detail.is_none());
        assert!(view.selection.item.is_none());
        assert!(view.selection.id.is_none());
    }
}
