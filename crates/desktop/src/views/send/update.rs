use std::sync::Arc;

use bitwarden_send::{SendId, SendType, SendView as SdkSendView};
use iced::Task;

use crate::{
    app::{Outcome, UpdateCtx},
    components::toast::Toast,
    debug_fmt::{NoDebug, Summary},
    domain::UserId,
    fl,
    services::sdk::ClientManager,
};

use super::{
    SendEvent, SendFilter, SendMessage,
    state::SendView,
    widgets::{
        send_edit::{FormEvent, SendEditMessage, SendForm},
        send_list::{SearchMessage, SendListMessage},
    },
};

// ── Task factory ───────────────────────────────────────────────────────────

impl SendView {
    /// Sync stub today (no SDK encrypt/decrypt yet); returned as a `Task` so
    /// the call shape lines up with `VaultView::load_list_task` and a future
    /// SDK-driven async path is a drop-in.
    pub fn load_list_task(uid: UserId, mgr: &ClientManager) -> Task<SendMessage> {
        let items: Vec<Arc<SdkSendView>> = mgr.list_sends(&uid).into_iter().map(Arc::new).collect();
        Task::done(SendMessage::ListLoaded(uid, Ok(Summary(items))))
    }
}

// ── Update dispatch ────────────────────────────────────────────────────────

impl SendView {
    pub fn update(&mut self, msg: SendMessage, mut ctx: UpdateCtx<'_>) -> Outcome<Self> {
        match msg {
            SendMessage::ItemList(m) => {
                return self.handle_item_list(&ctx, m);
            }
            SendMessage::Search(SearchMessage::QueryChanged(query)) => {
                self.search_query = query;
                if let Some(uid) = ctx.active_user {
                    self.recompute_filtered(uid, ctx.active_send_filter);
                }
            }
            SendMessage::CloseFormPane => {
                // Start the outro and defer the selection clear so the
                // sheet form stays alive while the slide animates out.
                // Wide-mode pane closes immediately.
                let task = self
                    .selection
                    .sheet_fade
                    .close_with_finalize(SendMessage::FinalizeSheetClose);
                self.pane.close();
                return Outcome::task(task);
            }
            SendMessage::FinalizeSheetClose => {
                self.selection.clear();
            }
            SendMessage::PaneResized(event) => self.pane.set_ratio(event.ratio),
            SendMessage::SendEdit(m) => {
                return self.handle_send_edit(&mut ctx, m);
            }
            SendMessage::CancelDeleteSelected => self.selection.confirm_delete.close(),
            SendMessage::ConfirmDeleteSelected => {
                return self.handle_confirm_delete(&mut ctx);
            }
            SendMessage::NewItem => {
                self.handle_new_item(&ctx);
            }
            SendMessage::AccountSwitcher(m) => {
                return Outcome::from_option(
                    m.consume(&mut *ctx.open_overlay, crate::app::Overlay::AccountSwitcher)
                        .map(SendEvent::AccountSwitcher),
                );
            }
            SendMessage::ListLoaded(uid, res) => {
                return self.handle_list_loaded(&ctx, uid, res);
            }
            SendMessage::DetailLoaded(uid, id, res) => {
                return self.handle_detail_loaded(&ctx, uid, id, res);
            }
            SendMessage::SaveCompleted(uid, res) => {
                return self.handle_save_completed(&ctx, uid, res);
            }
            SendMessage::DeleteCompleted(uid, id, res) => {
                return self.handle_delete_completed(&ctx, uid, id, res);
            }
            SendMessage::PasswordGenerated(res) => {
                // Mirror the official client: every generated value lands in
                // the user's history, regardless of which form triggered it.
                if let Ok(NoDebug(value)) = &res
                    && let Some(uid) = ctx.active_user.copied()
                {
                    ctx.client_manager.push_history(uid, value.clone());
                }
                return self.handle_password_generated(res);
            }
        }
        Outcome::None
    }
}

// ── Per-variant handlers ───────────────────────────────────────────────────

impl SendView {
    fn handle_item_list(&mut self, ctx: &UpdateCtx<'_>, msg: SendListMessage) -> Outcome<Self> {
        match msg {
            SendListMessage::ItemSelected(idx) => {
                self.selection.item = Some(idx);
                self.selection.id = ctx
                    .active_user
                    .and_then(|uid| self.items.get(uid))
                    .and_then(|ic| ic.cached.get(idx))
                    .and_then(|i| i.id);
                let Some(id) = self.selection.id else {
                    return Outcome::None;
                };
                let Some(uid) = ctx.active_user.copied() else {
                    return Outcome::None;
                };
                // Sync stub today; deliver via `Task::done` so the rest of
                // the view's flow (loading state, stale-uid guard) keeps the
                // same shape as the future async path.
                let result = ctx
                    .client_manager
                    .full_send(&uid, id)
                    .map(|v| NoDebug(Box::new(v)))
                    .ok_or_else(|| format!("send {id} not found"));
                return Outcome::task(Task::done(SendMessage::DetailLoaded(uid, id, result)));
            }
            SendListMessage::Scrolled(viewport) => {
                self.list_scroll.track(viewport);
            }
            SendListMessage::MoreOptions(_) => {}
        }
        Outcome::None
    }

    fn handle_new_item(&mut self, ctx: &UpdateCtx<'_>) {
        // The sub-filter dictates the default type. File sends currently have
        // no creation path beyond the placeholder "Choose file" button — the
        // form renders a disabled filename/size for them.
        let send_type = match ctx.active_send_filter {
            SendFilter::File => SendType::File,
            SendFilter::Text | SendFilter::AllItems => SendType::Text,
        };
        self.selection.clear();
        self.selection.form = Some(SendForm::new(send_type));
        self.selection.sheet_fade.open();
        self.pane.open();
    }

    fn handle_send_edit(&mut self, ctx: &mut UpdateCtx<'_>, msg: SendEditMessage) -> Outcome<Self> {
        let Some(form) = self.selection.form.as_mut() else {
            return Outcome::None;
        };
        match form.update(msg) {
            FormEvent::None => Outcome::None,
            FormEvent::Cancel => {
                let task = self
                    .selection
                    .sheet_fade
                    .close_with_finalize(SendMessage::FinalizeSheetClose);
                self.pane.close();
                Outcome::task(task)
            }
            FormEvent::Save => {
                if !form.is_valid() {
                    return Outcome::toast(Toast::warning(fl!("toast-required-fields"), None));
                }
                let Some(uid) = ctx.active_user.copied() else {
                    return Outcome::None;
                };
                form.saving = true;
                let view = form.to_send_view();
                // Sync save (no SDK encrypt yet); commit + dispatch the
                // completion message so handle_save_completed runs the same
                // path as the future async case.
                let saved = ctx.client_manager.save_send(uid, view);
                Outcome::task(Task::done(SendMessage::SaveCompleted(
                    uid,
                    Ok(NoDebug(Box::new(saved))),
                )))
            }
            FormEvent::Delete => {
                self.selection.confirm_delete.open();
                Outcome::None
            }
            FormEvent::CopyLink(url) => Outcome::event(SendEvent::ClipboardCopyRequested {
                value: url,
                sensitivity: crate::services::clipboard::Sensitivity::Normal,
                toast_label: fl!("send-toast-copied-link"),
            }),
            FormEvent::CopyPassword(pw) => Outcome::event(SendEvent::ClipboardCopyRequested {
                value: pw,
                sensitivity: crate::services::clipboard::Sensitivity::Sensitive,
                toast_label: fl!("send-toast-copied-password"),
            }),
            FormEvent::RegeneratePassword => Outcome::event(SendEvent::RegeneratePasswordRequested),
        }
    }

    fn handle_confirm_delete(&mut self, ctx: &mut UpdateCtx<'_>) -> Outcome<Self> {
        self.selection.confirm_delete.close();
        let Some(send_id) = self.selection.id else {
            // New-item form hasn't been saved yet — "delete" just dismisses
            // the draft via the same sheet-fade outro as cancel.
            let task = self
                .selection
                .sheet_fade
                .close_with_finalize(SendMessage::FinalizeSheetClose);
            self.pane.close();
            return Outcome::task(task);
        };
        let Some(uid) = ctx.active_user.copied() else {
            return Outcome::None;
        };
        ctx.client_manager.delete_send(&uid, send_id);
        Outcome::task(Task::done(SendMessage::DeleteCompleted(
            uid,
            send_id,
            Ok(()),
        )))
    }

    fn handle_list_loaded(
        &mut self,
        ctx: &UpdateCtx<'_>,
        msg_uid: UserId,
        result: Result<Summary<Vec<Arc<SdkSendView>>>, String>,
    ) -> Outcome<Self> {
        match result {
            Ok(Summary(items)) => {
                tracing::info!(uid = %msg_uid, count = items.len(), "send list loaded");
                let cache = self.items.entry(msg_uid).or_default();
                cache.all = items;
                if ctx.is_active_user(&msg_uid) {
                    self.recompute_filtered(&msg_uid, ctx.active_send_filter);
                }
            }
            Err(err) => {
                tracing::error!(uid = %msg_uid, %err, "list_sends failed");
            }
        }
        Outcome::None
    }

    fn handle_detail_loaded(
        &mut self,
        ctx: &UpdateCtx<'_>,
        msg_uid: UserId,
        id: SendId,
        result: Result<NoDebug<Box<SdkSendView>>, String>,
    ) -> Outcome<Self> {
        if !ctx.is_active_user(&msg_uid) {
            return Outcome::None;
        }
        match result {
            Ok(NoDebug(view)) => {
                if self.selection.id == Some(id) {
                    self.selection.form = Some(SendForm::edit(*view));
                    self.selection.sheet_fade.open();
                    self.pane.open();
                }
            }
            Err(err) => {
                tracing::error!(send_id = %id, %err, "full_send failed");
                return Outcome::toast(Toast::error(
                    fl!("send-toast-load-failed-body"),
                    Some(&fl!("send-toast-load-failed-title")),
                ));
            }
        }
        Outcome::None
    }

    fn handle_save_completed(
        &mut self,
        ctx: &UpdateCtx<'_>,
        msg_uid: UserId,
        result: Result<NoDebug<Box<SdkSendView>>, String>,
    ) -> Outcome<Self> {
        if !ctx.is_active_user(&msg_uid) {
            return Outcome::None;
        }
        match result {
            Ok(NoDebug(view)) => {
                // Re-bind selection to the persisted id (new sends start with
                // id=None; `save_send` assigns one) so subsequent reloads
                // keep the form open.
                self.selection.id = view.id;
                self.selection.form = Some(SendForm::edit(*view));
                self.selection.sheet_fade.open();
                Outcome::event(SendEvent::ItemSaved { uid: msg_uid })
            }
            Err(err) => {
                tracing::error!(%err, "save_send failed");
                if let Some(form) = self.selection.form.as_mut() {
                    form.saving = false;
                }
                Outcome::toast(Toast::error(
                    fl!("send-toast-save-failed-body"),
                    Some(&fl!("send-toast-save-failed-title")),
                ))
            }
        }
    }

    fn handle_password_generated(
        &mut self,
        result: Result<NoDebug<String>, String>,
    ) -> Outcome<Self> {
        match result {
            Ok(NoDebug(value)) => {
                if let Some(form) = self.selection.form.as_mut() {
                    form.apply_generated_password(value);
                }
                Outcome::None
            }
            Err(err) => {
                tracing::warn!(%err, "send password regenerate failed");
                Outcome::toast(Toast::warning(err, Some(&fl!("generator-toast-failed"))))
            }
        }
    }

    fn handle_delete_completed(
        &mut self,
        ctx: &UpdateCtx<'_>,
        msg_uid: UserId,
        send_id: SendId,
        result: Result<(), String>,
    ) -> Outcome<Self> {
        if !ctx.is_active_user(&msg_uid) {
            return Outcome::None;
        }
        match result {
            Ok(()) => {
                self.selection.clear();
                self.pane.close();
                Outcome::event(SendEvent::ItemDeleted { uid: msg_uid })
            }
            Err(err) => {
                tracing::error!(send_id = %send_id, %err, "delete_send failed");
                Outcome::toast(Toast::error(
                    fl!("send-toast-delete-failed-body"),
                    Some(&fl!("send-toast-delete-failed-title")),
                ))
            }
        }
    }
}

// ── Filter helpers ─────────────────────────────────────────────────────────

impl SendView {
    pub(super) fn recompute_filtered(&mut self, uid: &UserId, filter: SendFilter) {
        let query = self.search_query.to_lowercase();
        if let Some(cache) = self.items.get_mut(uid) {
            cache.cached = filter_items(&cache.all, filter, &query);
        }
        if let Some(id) = self.selection.id {
            self.selection.item = self
                .items
                .get(uid)
                .and_then(|c| c.cached.iter().position(|i| i.id == Some(id)));
        } else {
            self.selection.item = None;
        }
        self.list_scroll.offset_y = 0.0;
    }
}

fn filter_items(
    all: &[Arc<SdkSendView>],
    filter: SendFilter,
    query: &str,
) -> Vec<Arc<SdkSendView>> {
    let items = all.iter().filter(|item| match filter {
        SendFilter::AllItems => true,
        SendFilter::Text => matches!(item.r#type, SendType::Text),
        SendFilter::File => matches!(item.r#type, SendType::File),
    });
    if query.is_empty() {
        items.cloned().collect()
    } else {
        items
            .filter(|item| item.name.to_lowercase().contains(query))
            .cloned()
            .collect()
    }
}
