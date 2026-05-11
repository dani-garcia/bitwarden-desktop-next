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

#[cfg(test)]
mod tests {
    use super::matches_query;
    use crate::test_support::fixtures::{CipherSpec, login_kind, make_cipher};

    #[test]
    fn name_substring_hits() {
        let cipher = make_cipher(CipherSpec {
            name: "GitHub",
            ..Default::default()
        });
        assert!(matches_query(&cipher, "hub"));
    }

    #[test]
    fn name_match_is_case_insensitive() {
        let cipher = make_cipher(CipherSpec {
            name: "Acme Corp",
            ..Default::default()
        });
        // Caller is required to lowercase the query before calling.
        assert!(matches_query(&cipher, "acme"));
    }

    #[test]
    fn subtitle_substring_hits() {
        let cipher = make_cipher(CipherSpec {
            name: "Generic",
            subtitle: "alice@example.com",
            ..Default::default()
        });
        assert!(matches_query(&cipher, "alice"));
    }

    #[test]
    fn login_uri_substring_hits() {
        let cipher = make_cipher(CipherSpec {
            kind: login_kind(Some("https://github.com/login")),
            ..Default::default()
        });
        assert!(matches_query(&cipher, "github.com"));
    }

    #[test]
    fn non_login_type_ignores_uri_like_fields() {
        // SecureNote ciphers have no `uris` field — only name + subtitle
        // can match. A query that matches no field is rejected.
        let cipher = make_cipher(CipherSpec {
            name: "Note",
            subtitle: "",
            ..Default::default()
        });
        assert!(!matches_query(&cipher, "github.com"));
    }

    #[test]
    fn empty_query_matches_anything_with_any_text() {
        // The empty string is a substring of every string, so every cipher
        // with a non-empty name should match. Documents existing behaviour
        // — callers gate filter passes on `query.is_empty()` instead.
        let cipher = make_cipher(CipherSpec {
            name: "GitHub",
            ..Default::default()
        });
        assert!(matches_query(&cipher, ""));
    }
}
