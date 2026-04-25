//! `VaultView::update` dispatch + per-variant handlers + filter helpers +
//! the `load_list_task` factory.
//!
//! `update()` is a thin dispatch into `handle_*` methods that each own one
//! branch of the message tree. Selection accessors (`selected_login`,
//! `selected_field`) factor out the repeated
//! `self.selection.detail.as_ref().and_then(...)` walks.

use std::sync::Arc;

use bitwarden_vault::{
    CipherId, CipherListView, CipherListViewType, CipherView, FieldView, LoginView, SshKeyView,
};
use iced::Task;

use crate::{
    app::{Outcome, UpdateCtx},
    components::{sidebar::VaultFilter, toast::Toast},
    domain::UserId,
    fl,
    services::{clipboard::Sensitivity, sdk::ClientManager},
};

use super::{
    VaultEvent, VaultMessage,
    message::FormOptions,
    state::VaultView,
    widgets::{
        cipher_detail::{self, CipherDetailMessage},
        cipher_edit::{CipherForm, FolderOption, FormAction},
        item_list::ItemListMessage,
        search_bar::SearchMessage,
    },
};

// ── Task factory ───────────────────────────────────────────────────────────

impl VaultView {
    /// Build the task that decrypts the user's vault list and lands as
    /// `VaultMessage::ListLoaded`. Called from App handlers (unlock, user
    /// switch, sync) — the factory lives here so all `Task::perform` calls
    /// that produce `VaultMessage`s stay within the owning view.
    pub fn load_list_task(uid: UserId, mgr: &Arc<ClientManager>) -> Task<VaultMessage> {
        let mgr = mgr.clone();
        Task::perform(
            async move {
                mgr.list_ciphers(&uid)
                    .await
                    .map(|items| items.into_iter().map(Arc::new).collect::<Vec<_>>())
            },
            move |result| VaultMessage::ListLoaded(uid, result),
        )
    }
}

// ── Update dispatch ────────────────────────────────────────────────────────

impl VaultView {
    /// Compositional MVU update. Returns a task (for async work the view
    /// owns) and an optional event (cross-cutting fact for App to route).
    ///
    /// `ctx` carries the SDK handle + active user; it's built fresh on every
    /// `App::update` call so the view can construct `Task::perform` calls
    /// without owning shared state.
    pub fn update(&mut self, msg: VaultMessage, ctx: UpdateCtx<'_>) -> Outcome<Self> {
        let UpdateCtx {
            client_manager,
            active_user,
            active_vault_filter,
            open_overlay,
            ..
        } = ctx;
        match msg {
            VaultMessage::ItemList(m) => {
                return self.handle_item_list(m, client_manager, active_user);
            }
            VaultMessage::Search(SearchMessage::QueryChanged(query)) => {
                self.search_query = query;
                if let Some(uid) = active_user {
                    self.recompute_filtered(uid, active_vault_filter);
                }
            }
            VaultMessage::CloseCipherDetail => {
                // Start the outro and defer the selection clear so the
                // sheet content stays alive for ~180 ms while the slide
                // animates out. The wide-mode pane closes immediately —
                // it has no animation to wait on.
                self.selection.sheet_fade.close();
                self.pane.close();
                return Outcome::spawn(
                    tokio::time::sleep(std::time::Duration::from_millis(180)),
                    |_| VaultMessage::FinalizeSheetClose,
                );
            }
            VaultMessage::FinalizeSheetClose => {
                self.selection.clear();
            }
            VaultMessage::PaneResized(event) => self.pane.set_ratio(event.ratio),
            VaultMessage::CipherDetail(m) => {
                return self.handle_cipher_detail(m, client_manager, active_user);
            }
            VaultMessage::CancelDeleteSelected => self.selection.confirm_delete.close(),
            VaultMessage::ConfirmDeleteSelected => {
                return self.handle_confirm_delete(client_manager, active_user);
            }
            VaultMessage::CipherEdit(m) => {
                return self.handle_cipher_edit(m, client_manager, active_user);
            }
            VaultMessage::FormOptionsLoaded(uid, opts) => {
                return self.handle_form_options_loaded(uid, opts, active_user);
            }
            VaultMessage::SaveCompleted(uid, res) => {
                return self.handle_save_completed(uid, res, active_user);
            }
            VaultMessage::DeleteCompleted(uid, id, res) => {
                return self.handle_delete_completed(uid, id, res, active_user);
            }
            VaultMessage::AccountSwitcher(m) => {
                return Outcome::from_option(
                    m.consume(open_overlay, crate::app::Overlay::AccountSwitcher)
                        .map(VaultEvent::AccountSwitcher),
                );
            }
            VaultMessage::NewItem => {}
            VaultMessage::ListLoaded(uid, res) => {
                return self.handle_list_loaded(
                    uid,
                    res,
                    client_manager,
                    active_user,
                    active_vault_filter,
                );
            }
            VaultMessage::DetailLoaded(uid, id, res) => {
                return self.handle_detail_loaded(uid, id, res, active_user);
            }
        }
        Outcome::None
    }
}

// ── Selection accessors ────────────────────────────────────────────────────
//
// Shared walk of `self.selection.detail.as_ref()` into the login / fields
// sub-objects. Removes the repeated four-line chain from every handler arm
// that needs a value to copy.

impl VaultView {
    fn selected_login(&self) -> Option<&LoginView> {
        self.selection.detail.as_ref()?.login.as_ref()
    }

    fn selected_field(&self, idx: usize) -> Option<&FieldView> {
        self.selection.detail.as_ref()?.fields.as_deref()?.get(idx)
    }

    fn selected_login_uri(&self, idx: usize) -> Option<&str> {
        cipher_detail::login_uri_at(self.selected_login()?, idx)
    }

    fn selected_ssh_key(&self) -> Option<&SshKeyView> {
        self.selection.detail.as_ref()?.ssh_key.as_ref()
    }
}

// ── Per-variant handlers ───────────────────────────────────────────────────

impl VaultView {
    /// Called by App when the active vault filter changes. Clears the
    /// current selection (what's selected may no longer be in the list) and
    /// recomputes the cached list for the active user.
    pub fn apply_filter(&mut self, uid: &UserId, filter: VaultFilter) {
        self.selection.clear();
        self.pane.close();
        self.recompute_filtered(uid, filter);
    }

    fn handle_item_list(
        &mut self,
        msg: ItemListMessage,
        client_manager: &Arc<ClientManager>,
        active_user: Option<&UserId>,
    ) -> Outcome<Self> {
        match msg {
            ItemListMessage::ItemSelected(idx) => {
                let new_id = active_user
                    .and_then(|uid| self.items.get(uid))
                    .and_then(|ic| ic.cached.get(idx))
                    .and_then(|i| i.id);
                // Switching to a different cipher drops any open edit form
                // and disarms the inline delete confirmation — user clicked
                // the list to move on, they don't want the previous item's
                // transient UI bleeding onto the new one. `detail` is left
                // as-is so the pane doesn't flash empty during the fetch
                // (same pattern as re-selecting after an existing detail).
                if new_id != self.selection.id {
                    self.selection.form = None;
                    self.selection.confirm_delete.close();
                }
                self.selection.item = Some(idx);
                self.selection.id = new_id;
                let Some(id) = self.selection.id else {
                    return Outcome::None;
                };
                let Some(uid) = active_user.copied() else {
                    return Outcome::None;
                };
                let mgr = client_manager.clone();
                return Outcome::spawn(
                    async move { mgr.full_cipher(&uid, id).await },
                    move |res| VaultMessage::DetailLoaded(uid, id, res.map(Box::new)),
                );
            }
            ItemListMessage::OpenExternal(_)
            | ItemListMessage::CopyUsername(_)
            | ItemListMessage::MoreOptions(_) => {}
            ItemListMessage::Scrolled(viewport) => {
                self.list_scroll.track(viewport);
            }
        }
        Outcome::None
    }

    fn handle_cipher_detail(
        &mut self,
        msg: CipherDetailMessage,
        client_manager: &Arc<ClientManager>,
        active_user: Option<&UserId>,
    ) -> Outcome<Self> {
        match msg {
            CipherDetailMessage::Edit => self.handle_detail_edit(client_manager, active_user),
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

    fn handle_detail_edit(
        &mut self,
        client_manager: &Arc<ClientManager>,
        active_user: Option<&UserId>,
    ) -> Outcome<Self> {
        let Some(detail) = self.selection.detail.clone() else {
            return Outcome::None;
        };
        let Some(uid) = active_user.copied() else {
            return Outcome::None;
        };
        self.selection.form = Some(CipherForm::edit(detail));

        // Kick off all three option-list loads as one task. Folders goes
        // through the SDK repo (async); orgs + collections are pure reads
        // off the already-loaded ClientManager, so they complete inside
        // the same async block with near-zero cost. Delivering them
        // together means the handler runs a single stale-check + form-
        // exists check.
        let mgr = client_manager.clone();
        Outcome::spawn(
            async move {
                let folders = mgr
                    .list_folders(&uid)
                    .await
                    .map(|folders| folders.into_iter().map(FolderOption::from).collect());
                let organizations = mgr.list_organizations(&uid);
                let collections = mgr.list_collections(&uid);
                FormOptions {
                    folders,
                    organizations,
                    collections,
                }
            },
            move |opts| VaultMessage::FormOptionsLoaded(uid, opts),
        )
    }

    fn handle_confirm_delete(
        &mut self,
        client_manager: &Arc<ClientManager>,
        active_user: Option<&UserId>,
    ) -> Outcome<Self> {
        self.selection.confirm_delete.close();
        let Some(cipher_id) = self.selection.id else {
            return Outcome::None;
        };
        let Some(uid) = active_user.copied() else {
            return Outcome::None;
        };
        let mgr = client_manager.clone();
        Outcome::spawn(
            async move { mgr.soft_delete_cipher(&uid, cipher_id).await },
            move |res| VaultMessage::DeleteCompleted(uid, cipher_id, res),
        )
    }

    fn handle_cipher_edit(
        &mut self,
        msg: super::widgets::cipher_edit::CipherEditMessage,
        client_manager: &Arc<ClientManager>,
        active_user: Option<&UserId>,
    ) -> Outcome<Self> {
        let Some(form) = self.selection.form.as_mut() else {
            return Outcome::None;
        };
        match form.update(msg) {
            FormAction::None => Outcome::None,
            FormAction::Cancel => {
                self.selection.form = None;
                Outcome::None
            }
            FormAction::Save => {
                if !form.is_valid() {
                    return Outcome::event(VaultEvent::ToastRequested(Toast::warning(
                        fl!("toast-required-fields"),
                        None,
                    )));
                }
                let Some(uid) = active_user.copied() else {
                    return Outcome::None;
                };
                form.saving = true;
                let mgr = client_manager.clone();
                let cipher_view = form.modified.clone();
                Outcome::spawn(
                    async move { mgr.save_cipher(&uid, cipher_view).await },
                    move |res| VaultMessage::SaveCompleted(uid, res.map(Box::new)),
                )
            }
        }
    }

    fn handle_form_options_loaded(
        &mut self,
        msg_uid: UserId,
        opts: FormOptions,
        active_user: Option<&UserId>,
    ) -> Outcome<Self> {
        if active_user != Some(&msg_uid) {
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

    fn handle_save_completed(
        &mut self,
        msg_uid: UserId,
        result: Result<Box<CipherView>, String>,
        active_user: Option<&UserId>,
    ) -> Outcome<Self> {
        if active_user != Some(&msg_uid) {
            return Outcome::None;
        }
        let event = match result {
            Ok(view) => {
                self.selection.detail = Some(*view);
                self.selection.form = None;
                self.selection.sheet_fade.open();
                VaultEvent::ItemSaved { uid: msg_uid }
            }
            Err(err) => {
                tracing::error!(%err, "save_cipher failed");
                if let Some(form) = self.selection.form.as_mut() {
                    form.saving = false;
                }
                VaultEvent::ToastRequested(Toast::error(
                    fl!("vault-toast-save-failed-body"),
                    Some(&fl!("vault-toast-save-failed-title")),
                ))
            }
        };
        Outcome::event(event)
    }

    fn handle_delete_completed(
        &mut self,
        msg_uid: UserId,
        cipher_id: CipherId,
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
                VaultEvent::ItemDeleted { uid: msg_uid }
            }
            Err(err) => {
                tracing::error!(cipher_id = %cipher_id, %err, "soft_delete_cipher failed");
                VaultEvent::ToastRequested(Toast::error(
                    fl!("vault-toast-delete-failed-body"),
                    Some(&fl!("vault-toast-delete-failed-title")),
                ))
            }
        };
        Outcome::event(event)
    }

    fn handle_list_loaded(
        &mut self,
        msg_uid: UserId,
        result: Result<Vec<Arc<CipherListView>>, String>,
        client_manager: &Arc<ClientManager>,
        active_user: Option<&UserId>,
        active_filter: VaultFilter,
    ) -> Outcome<Self> {
        match result {
            Ok(items) => {
                tracing::info!(uid = %msg_uid, count = items.len(), "vault list loaded");
                let organizations = client_manager.list_organizations(&msg_uid);
                let cache = self.items.entry(msg_uid).or_default();
                cache.all = items;
                cache.organizations = organizations;
                // Only recompute the filtered view if this is the active user
                // — search_query / active_filter are view-global state that
                // may not match a background user's context.
                if active_user == Some(&msg_uid) {
                    self.recompute_filtered(&msg_uid, active_filter);
                }
            }
            Err(err) => {
                tracing::error!(uid = %msg_uid, %err, "list_ciphers failed");
            }
        }
        Outcome::None
    }

    fn handle_detail_loaded(
        &mut self,
        msg_uid: UserId,
        id: CipherId,
        result: Result<Box<CipherView>, String>,
        active_user: Option<&UserId>,
    ) -> Outcome<Self> {
        // Stale-check: user switched while full_cipher was in flight.
        if active_user != Some(&msg_uid) {
            tracing::debug!(
                uid = %msg_uid,
                cipher_id = %id,
                "full_cipher result dropped: active user changed while in flight"
            );
            return Outcome::None;
        }
        match result {
            Ok(view) => {
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
                return Outcome::event(VaultEvent::ToastRequested(Toast::error(
                    fl!("vault-toast-decrypt-failed-body"),
                    Some(&fl!("vault-toast-decrypt-failed-title")),
                )));
            }
        }
        Outcome::None
    }
}

// ── Filter helpers ─────────────────────────────────────────────────────────

impl VaultView {
    /// Recompute the filtered item list for a specific user. Uses the
    /// view-global `search_query` plus the app-level active vault filter
    /// to derive `cached` from `all`. Also reconciles the selection index
    /// against the new filtered list.
    pub(super) fn recompute_filtered(&mut self, uid: &UserId, filter: VaultFilter) {
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
        // Filter / search / reload shrinks or reshuffles the dataset —
        // reset the scroll offset so we don't end up past the new end.
        // The viewport_height stays cached from the last real scroll event.
        self.list_scroll.offset_y = 0.0;
    }
}

fn filter_items(
    all: &[Arc<CipherListView>],
    filter: VaultFilter,
    query: &str,
) -> Vec<Arc<CipherListView>> {
    let items = all.iter().filter(|item| match filter {
        // Archived/deleted ciphers only surface in their dedicated views —
        // every other filter hides them so a soft-deleted login doesn't keep
        // showing up under All Items / Favorites / Personal / etc.
        VaultFilter::AllItems => item.deleted_date.is_none() && item.archived_date.is_none(),
        VaultFilter::Personal => {
            item.organization_id.is_none()
                && item.deleted_date.is_none()
                && item.archived_date.is_none()
        }
        VaultFilter::Organization(org_id) => {
            item.organization_id == Some(org_id)
                && item.deleted_date.is_none()
                && item.archived_date.is_none()
        }
        VaultFilter::Favorites => {
            item.favorite && item.deleted_date.is_none() && item.archived_date.is_none()
        }
        VaultFilter::Category(cat) => {
            cat == cipher_list_view_type_to_type(&item.r#type)
                && item.deleted_date.is_none()
                && item.archived_date.is_none()
        }
        VaultFilter::Archive => item.archived_date.is_some(),
        VaultFilter::Trash => item.deleted_date.is_some(),
    });

    if query.is_empty() {
        items.cloned().collect()
    } else {
        items
            .filter(|item| crate::services::search::matches_query(item, query))
            .cloned()
            .collect()
    }
}

/// Project a `CipherListViewType` down to its discriminant `CipherType`.
fn cipher_list_view_type_to_type(t: &CipherListViewType) -> bitwarden_vault::CipherType {
    use bitwarden_vault::CipherType;
    match t {
        CipherListViewType::Login(_) => CipherType::Login,
        CipherListViewType::SecureNote => CipherType::SecureNote,
        CipherListViewType::Card(_) => CipherType::Card,
        CipherListViewType::Identity => CipherType::Identity,
        CipherListViewType::SshKey => CipherType::SshKey,
    }
}

// ── Event-construction helper ─────────────────────────────────────────────

/// Build the `Outcome` for a clipboard-copy action. `value.is_none()` means
/// the underlying field was empty/missing; the handler falls through silently.
fn clipboard_outcome(
    value: Option<String>,
    sensitivity: Sensitivity,
    toast_label: String,
) -> Outcome<VaultView> {
    Outcome::from_option(value.map(|value| VaultEvent::ClipboardCopyRequested {
        value,
        sensitivity,
        toast_label,
    }))
}
