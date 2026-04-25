//! Cipher substring search predicate, shared between the vault list and the
//! Magnify launcher. Pure logic over a decrypted `CipherListView` — no
//! service state — so it lives in `services/` rather than `domain.rs` for
//! discoverability ("how do I match a query?" → look here).

use bitwarden_vault::{CipherListView, CipherListViewType};

/// Substring match on the cipher's user-visible fields. `query` must already
/// be lowercased — callers do this once per filter pass instead of per item.
/// Used by the vault list filter (`views::vault::update::filter_items`) and
/// by the Magnify launcher's recompute pass (`views::magnify::state::recompute`).
pub fn matches_query(item: &CipherListView, query: &str) -> bool {
    if item.name.to_lowercase().contains(query) || item.subtitle.to_lowercase().contains(query) {
        return true;
    }
    if let CipherListViewType::Login(login) = &item.r#type
        && let Some(uri) = login
            .uris
            .as_ref()
            .and_then(|u| u.first())
            .and_then(|u| u.uri.as_deref())
        && uri.to_lowercase().contains(query)
    {
        return true;
    }
    false
}
