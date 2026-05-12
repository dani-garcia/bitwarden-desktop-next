//! Vault list interaction: row selection, list-loaded, filter recomputation.

use std::sync::Arc;

use bitwarden_vault::{CipherListView, CipherListViewType};

use crate::{
    app::{Outcome, UpdateCtx},
    debug_fmt::{NoDebug, Summary},
    domain::UserId,
    services::sdk::ClientExt,
};

use crate::views::vault::{
    VaultFilter, VaultMessage, state::VaultView, widgets::item_list::ItemListMessage,
};

impl VaultView {
    pub(super) fn handle_item_list(
        &mut self,
        ctx: &UpdateCtx<'_>,
        msg: ItemListMessage,
    ) -> Outcome<Self> {
        match msg {
            ItemListMessage::ItemSelected(idx) => {
                let new_id = ctx
                    .active_user
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
                return ctx.perform_with_active_client(
                    move |client| client.full_cipher(id),
                    move |uid, res| {
                        VaultMessage::DetailLoaded(uid, id, res.map(|v| NoDebug(Box::new(v))))
                    },
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

    pub(super) fn handle_list_loaded(
        &mut self,
        ctx: &UpdateCtx<'_>,
        msg_uid: UserId,
        result: Result<Summary<Vec<Arc<CipherListView>>>, String>,
    ) -> Outcome<Self> {
        match result {
            Ok(Summary(items)) => {
                tracing::info!(uid = %msg_uid, count = items.len(), "vault list loaded");
                let organizations = ctx.client_manager.list_organizations(&msg_uid);
                let cache = self.items.entry(msg_uid).or_default();
                cache.all = items;
                cache.organizations = organizations;
                // Only recompute the filtered view if this is the active user
                // — search_query / active_filter are view-global state that
                // may not match a background user's context.
                if ctx.is_active_user(&msg_uid) {
                    self.recompute_filtered(&msg_uid, ctx.active_vault_filter);
                }
            }
            Err(err) => {
                tracing::error!(uid = %msg_uid, %err, "list_ciphers failed");
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
    pub(in crate::views::vault) fn recompute_filtered(
        &mut self,
        uid: &UserId,
        filter: VaultFilter,
    ) {
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
        CipherListViewType::BankAccount => CipherType::BankAccount,
    }
}

/// Build the `Outcome` for a clipboard-copy action. `value.is_none()` means
/// the underlying field was empty/missing; the handler falls through silently.
pub(super) fn clipboard_outcome(
    value: Option<String>,
    sensitivity: crate::services::clipboard::Sensitivity,
    toast_label: String,
) -> Outcome<VaultView> {
    Outcome::from_option(value.map(|value| {
        crate::views::vault::VaultEvent::ClipboardCopyRequested {
            value,
            sensitivity,
            toast_label,
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::filter_items;
    use crate::{
        test_support::fixtures::{CipherSpec, login_kind, make_cipher},
        views::vault::VaultFilter,
    };
    use bitwarden_core::OrganizationId;
    use bitwarden_vault::{CipherListViewType, CipherType};

    #[test]
    fn all_items_excludes_deleted_and_archived() {
        let alive = make_cipher(CipherSpec {
            name: "Alive",
            ..Default::default()
        });
        let deleted = make_cipher(CipherSpec {
            name: "Trashed",
            deleted: true,
            ..Default::default()
        });
        let archived = make_cipher(CipherSpec {
            name: "Archived",
            archived: true,
            ..Default::default()
        });
        let all = vec![alive, deleted, archived];

        let out = filter_items(&all, VaultFilter::AllItems, "");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Alive");
    }

    #[test]
    fn trash_shows_only_deleted() {
        let alive = make_cipher(CipherSpec {
            name: "Alive",
            ..Default::default()
        });
        let deleted = make_cipher(CipherSpec {
            name: "Trashed",
            deleted: true,
            ..Default::default()
        });

        let out = filter_items(&[alive, deleted], VaultFilter::Trash, "");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Trashed");
    }

    #[test]
    fn archive_shows_only_archived() {
        let alive = make_cipher(CipherSpec {
            name: "Alive",
            ..Default::default()
        });
        let archived = make_cipher(CipherSpec {
            name: "Archived",
            archived: true,
            ..Default::default()
        });

        let out = filter_items(&[alive, archived], VaultFilter::Archive, "");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Archived");
    }

    #[test]
    fn favorites_excludes_deleted() {
        // Soft-deleting a favorite shouldn't leave it in the Favorites view.
        let fave = make_cipher(CipherSpec {
            name: "Fave",
            favorite: true,
            ..Default::default()
        });
        let deleted_fave = make_cipher(CipherSpec {
            name: "TrashedFave",
            favorite: true,
            deleted: true,
            ..Default::default()
        });
        let plain = make_cipher(CipherSpec {
            name: "Plain",
            ..Default::default()
        });

        let out = filter_items(&[fave, deleted_fave, plain], VaultFilter::Favorites, "");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Fave");
    }

    #[test]
    fn personal_excludes_org_items() {
        let org = OrganizationId::new_v4();
        let personal = make_cipher(CipherSpec {
            name: "Personal",
            ..Default::default()
        });
        let org_item = make_cipher(CipherSpec {
            name: "Org",
            organization: Some(org),
            ..Default::default()
        });

        let out = filter_items(&[personal, org_item], VaultFilter::Personal, "");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Personal");
    }

    #[test]
    fn organization_filter_shows_only_matching_org() {
        let org_a = OrganizationId::new_v4();
        let org_b = OrganizationId::new_v4();
        let a = make_cipher(CipherSpec {
            name: "A",
            organization: Some(org_a),
            ..Default::default()
        });
        let b = make_cipher(CipherSpec {
            name: "B",
            organization: Some(org_b),
            ..Default::default()
        });

        let out = filter_items(&[a, b], VaultFilter::Organization(org_a), "");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "A");
    }

    #[test]
    fn category_filter_matches_only_that_kind() {
        let login = make_cipher(CipherSpec {
            name: "Login",
            kind: login_kind(None),
            ..Default::default()
        });
        let note = make_cipher(CipherSpec {
            name: "Note",
            kind: CipherListViewType::SecureNote,
            ..Default::default()
        });

        let out = filter_items(&[login, note], VaultFilter::Category(CipherType::Login), "");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Login");
    }

    #[test]
    fn query_filter_combines_with_type_filter() {
        // "github" should match the login by URI, not the note that shares
        // the substring in its name — Category gates first, then query.
        let gh_login = make_cipher(CipherSpec {
            name: "GH",
            kind: login_kind(Some("https://github.com")),
            ..Default::default()
        });
        let gh_note = make_cipher(CipherSpec {
            name: "github tips",
            kind: CipherListViewType::SecureNote,
            ..Default::default()
        });

        let out = filter_items(
            &[gh_login, gh_note],
            VaultFilter::Category(CipherType::Login),
            "github",
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "GH");
    }
}
