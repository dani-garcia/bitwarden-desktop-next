//! Cipher substring search predicate, shared between the vault list and the
//! Magnify launcher. Pure logic over a decrypted `CipherListView` — lives in
//! `services/` rather than `domain.rs` for discoverability.

use bitwarden_vault::{CipherListView, CipherListViewType};

/// Substring match on the cipher's user-visible fields. `query` must already
/// be lowercased — callers do this once per filter pass instead of per item.
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
