//! `SendView::update` dispatch + per-variant handlers + filter helpers.

use std::sync::Arc;

use bitwarden_send::{SendId, SendType, SendView as SdkSendView};
use iced::Task;

use crate::{
    app::{Outcome, UpdateCtx},
    components::{sidebar::SendFilter, toast::Toast},
    domain::UserId,
    fl,
    services::sdk::ClientManager,
};

use super::{
    SendEvent, SendMessage,
    state::SendView,
    widgets::{
        send_edit::{FormAction, SendForm, SendEditMessage},
        send_list::{SearchMessage, SendListMessage},
    },
};

// ── Task factory ───────────────────────────────────────────────────────────

impl SendView {
    pub fn load_list_task(uid: UserId, mgr: &Arc<ClientManager>) -> Task<SendMessage> {
        let mgr = mgr.clone();
        Task::perform(
            async move {
                mgr.list_sends(&uid)
                    .await
                    .map(|items| items.into_iter().map(Arc::new).collect::<Vec<_>>())
            },
            move |result| SendMessage::ListLoaded(uid, result),
        )
    }
}

// ── Update dispatch ────────────────────────────────────────────────────────

impl SendView {
    pub fn update(&mut self, msg: SendMessage, ctx: UpdateCtx<'_>) -> Outcome<Self> {
        let UpdateCtx {
            client_manager,
            active_user,
            active_send_filter,
            open_overlay,
            ..
        } = ctx;
        match msg {
            SendMessage::ItemList(m) => {
                return self.handle_item_list(m, client_manager, active_user);
            }
            SendMessage::Search(SearchMessage::QueryChanged(query)) => {
                self.search_query = query;
                if let Some(uid) = active_user {
                    self.recompute_filtered(uid, active_send_filter);
                }
            }
            SendMessage::CloseFormPane => {
                self.selection.clear();
                self.pane.close();
            }
            SendMessage::PaneResized(event) => self.pane.set_ratio(event.ratio),
            SendMessage::SendEdit(m) => {
                return self.handle_send_edit(m, client_manager, active_user);
            }
            SendMessage::CancelDeleteSelected => self.selection.confirm_delete.close(),
            SendMessage::ConfirmDeleteSelected => {
                return self.handle_confirm_delete(client_manager, active_user);
            }
            SendMessage::NewItem => {
                self.handle_new_item(active_send_filter);
            }
            SendMessage::AccountSwitcher(m) => {
                return Outcome::from_option(
                    m.consume(open_overlay, crate::app::Overlay::AccountSwitcher)
                        .map(SendEvent::AccountSwitcher),
                );
            }
            SendMessage::ListLoaded(uid, res) => {
                return self.handle_list_loaded(uid, res, active_user, active_send_filter);
            }
            SendMessage::DetailLoaded(uid, id, res) => {
                return self.handle_detail_loaded(uid, id, res, active_user);
            }
            SendMessage::SaveCompleted(uid, res) => {
                return self.handle_save_completed(uid, res, active_user);
            }
            SendMessage::DeleteCompleted(uid, id, res) => {
                return self.handle_delete_completed(uid, id, res, active_user);
            }
        }
        Outcome::None
    }
}

// ── Per-variant handlers ───────────────────────────────────────────────────

impl SendView {
    fn handle_item_list(
        &mut self,
        msg: SendListMessage,
        client_manager: &Arc<ClientManager>,
        active_user: Option<&UserId>,
    ) -> Outcome<Self> {
        match msg {
            SendListMessage::ItemSelected(idx) => {
                self.selection.item = Some(idx);
                self.selection.id = active_user
                    .and_then(|uid| self.items.get(uid))
                    .and_then(|ic| ic.cached.get(idx))
                    .and_then(|i| i.id);
                let Some(id) = self.selection.id else {
                    return Outcome::None;
                };
                let Some(uid) = active_user.copied() else {
                    return Outcome::None;
                };
                let mgr = client_manager.clone();
                return Outcome::spawn(
                    async move { mgr.full_send(&uid, id).await },
                    move |res| SendMessage::DetailLoaded(uid, id, res.map(Box::new)),
                );
            }
            SendListMessage::Scrolled(viewport) => {
                self.list_scroll.track(viewport);
            }
            SendListMessage::MoreOptions(_) => {}
        }
        Outcome::None
    }

    fn handle_new_item(&mut self, active_filter: SendFilter) {
        // The sub-filter dictates the default type. File sends currently have
        // no creation path beyond the placeholder "Choose file" button — the
        // form renders a disabled filename/size for them.
        let send_type = match active_filter {
            SendFilter::File => SendType::File,
            SendFilter::Text | SendFilter::AllItems => SendType::Text,
        };
        self.selection.clear();
        self.selection.form = Some(SendForm::new(send_type));
        self.pane.open();
    }

    fn handle_send_edit(
        &mut self,
        msg: SendEditMessage,
        client_manager: &Arc<ClientManager>,
        active_user: Option<&UserId>,
    ) -> Outcome<Self> {
        let Some(form) = self.selection.form.as_mut() else {
            return Outcome::None;
        };
        match form.update(msg) {
            FormAction::None => Outcome::None,
            FormAction::Cancel => {
                self.selection.clear();
                self.pane.close();
                Outcome::None
            }
            FormAction::Save => {
                if !form.is_valid() {
                    return Outcome::event(SendEvent::ToastRequested(Toast::warning(
                        fl!("toast-required-fields"),
                        None,
                    )));
                }
                let Some(uid) = active_user.copied() else {
                    return Outcome::None;
                };
                form.saving = true;
                let mgr = client_manager.clone();
                let view = form.to_send_view();
                Outcome::spawn(
                    async move { mgr.save_send(&uid, view).await },
                    move |res| SendMessage::SaveCompleted(uid, res.map(Box::new)),
                )
            }
            FormAction::Delete => {
                self.selection.confirm_delete.open();
                Outcome::None
            }
            FormAction::CopyLink(url) => Outcome::event(SendEvent::ClipboardCopyRequested {
                value: url,
                sensitivity: crate::services::clipboard::Sensitivity::Normal,
                toast_label: fl!("send-toast-copied-link"),
            }),
            FormAction::CopyPassword(pw) => Outcome::event(SendEvent::ClipboardCopyRequested {
                value: pw,
                sensitivity: crate::services::clipboard::Sensitivity::Sensitive,
                toast_label: fl!("send-toast-copied-password"),
            }),
        }
    }

    fn handle_confirm_delete(
        &mut self,
        client_manager: &Arc<ClientManager>,
        active_user: Option<&UserId>,
    ) -> Outcome<Self> {
        self.selection.confirm_delete.close();
        let Some(send_id) = self.selection.id else {
            // New-item form hasn't been saved yet — "delete" just dismisses
            // the draft.
            self.selection.clear();
            self.pane.close();
            return Outcome::None;
        };
        let Some(uid) = active_user.copied() else {
            return Outcome::None;
        };
        let mgr = client_manager.clone();
        Outcome::spawn(
            async move { mgr.delete_send(&uid, send_id).await },
            move |res| SendMessage::DeleteCompleted(uid, send_id, res),
        )
    }

    fn handle_list_loaded(
        &mut self,
        msg_uid: UserId,
        result: Result<Vec<Arc<SdkSendView>>, String>,
        active_user: Option<&UserId>,
        active_filter: SendFilter,
    ) -> Outcome<Self> {
        match result {
            Ok(items) => {
                tracing::info!(uid = %msg_uid, count = items.len(), "send list loaded");
                let cache = self.items.entry(msg_uid).or_default();
                cache.all = items;
                if active_user == Some(&msg_uid) {
                    self.recompute_filtered(&msg_uid, active_filter);
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
        msg_uid: UserId,
        id: SendId,
        result: Result<Box<SdkSendView>, String>,
        active_user: Option<&UserId>,
    ) -> Outcome<Self> {
        if active_user != Some(&msg_uid) {
            return Outcome::None;
        }
        match result {
            Ok(view) => {
                if self.selection.id == Some(id) {
                    self.selection.form = Some(SendForm::edit(*view));
                    self.pane.open();
                }
            }
            Err(err) => {
                tracing::error!(send_id = %id, %err, "full_send failed");
                return Outcome::event(SendEvent::ToastRequested(Toast::error(
                    fl!("send-toast-load-failed-body"),
                    Some(&fl!("send-toast-load-failed-title")),
                )));
            }
        }
        Outcome::None
    }

    fn handle_save_completed(
        &mut self,
        msg_uid: UserId,
        result: Result<Box<SdkSendView>, String>,
        active_user: Option<&UserId>,
    ) -> Outcome<Self> {
        if active_user != Some(&msg_uid) {
            return Outcome::None;
        }
        let event = match result {
            Ok(view) => {
                // Re-bind selection to the persisted id (new sends start with
                // id=None; `save_send` assigns one) so subsequent reloads
                // keep the form open.
                self.selection.id = view.id;
                self.selection.form = Some(SendForm::edit(*view));
                SendEvent::ItemSaved { uid: msg_uid }
            }
            Err(err) => {
                tracing::error!(%err, "save_send failed");
                if let Some(form) = self.selection.form.as_mut() {
                    form.saving = false;
                }
                SendEvent::ToastRequested(Toast::error(
                    fl!("send-toast-save-failed-body"),
                    Some(&fl!("send-toast-save-failed-title")),
                ))
            }
        };
        Outcome::event(event)
    }

    fn handle_delete_completed(
        &mut self,
        msg_uid: UserId,
        send_id: SendId,
        result: Result<(), String>,
        active_user: Option<&UserId>,
    ) -> Outcome<Self> {
        if active_user != Some(&msg_uid) {
            return Outcome::None;
        }
        let event = match result {
            Ok(()) => {
                self.selection.clear();
                self.pane.close();
                SendEvent::ItemDeleted { uid: msg_uid }
            }
            Err(err) => {
                tracing::error!(send_id = %send_id, %err, "delete_send failed");
                SendEvent::ToastRequested(Toast::error(
                    fl!("send-toast-delete-failed-body"),
                    Some(&fl!("send-toast-delete-failed-title")),
                ))
            }
        };
        Outcome::event(event)
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

