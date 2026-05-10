//! Per-user `PasswordManagerClient` management.
//!
//! Users are discovered by listing `*.sqlite` files in `<workspace-root>/data/`
//! and pairing them with metadata from `data/mock.json`. Each user gets a
//! `PasswordManagerClient` whose state registry is backed by that user's
//! SQLite db. [`UnlockData::unlock`](crate::services::sdk::UnlockData::unlock)
//! invokes `initialize_user_crypto` and unlocks the in-memory keystore.
//!
//! ## Dev passwords
//!
//! - `alice@example.com` → `password` (Personal, ~20 ciphers)
//! - `alice@acmecorp.com` → `123456` (Work, ~10 ciphers)
//! - `loadtest@example.com` → `loadtest` (Load Test, ~20k ciphers)
//!
//! Regenerate `data/` via `cargo run -p fake-data`.
//!
//! ## Layout
//!
//! - [`types`] — public data types ([`Organization`], [`Collection`],
//!   [`AccountEntry`], [`PasswordHistoryEntry`]).
//! - [`client_ext`] — [`ClientExt`] trait + impl on `PasswordManagerClient`.
//!   All async SDK ops are methods here, called as
//!   `client.list_ciphers().await` from views.
//! - [`unlock`] — [`UnlockData`] + the free [`sync`] stub.
//! - [`loader`] — `mock.json` parsing + per-user client construction.
//! - [`user_entry`] — private `UserEntry` struct held by `ClientManager`.

mod client_ext;
mod loader;
mod types;
mod unlock;
mod user_entry;

use std::collections::HashMap;

use bitwarden_core::UserId;
use bitwarden_pm::PasswordManagerClient;
use bitwarden_send::{SendId, SendView};

use self::user_entry::UserEntry;

pub use client_ext::ClientExt;
pub use loader::verify_data_dir;
pub use types::{AccountEntry, Collection, Organization, PasswordHistoryEntry, VaultChoice};
pub use unlock::{UnlockData, sync};

use crate::domain::UnlockMethods;

/// Owns per-user state. Single-threaded: every method runs from the iced
/// update thread. Async SDK work is done by methods on [`ClientExt`] (see
/// [`client_ext`]) that take a [`PasswordManagerClient`] (or, for unlock, an
/// [`UnlockData`]) extracted via the accessors here — the manager itself is
/// never borrowed across `.await`, so no interior locking is needed.
/// Mutating methods take `&mut self` and run sync from `App::update`.
pub struct ClientManager {
    /// Boxed so the (rather large) `UserEntry` doesn't bloat the HashMap
    /// node; cloning a `PasswordManagerClient` for an async task only needs
    /// the inner `Arc`-backed handle, not the entry itself.
    users: HashMap<UserId, Box<UserEntry>>,
    /// In-memory send store keyed by user; not encrypted/persisted yet.
    sends: HashMap<UserId, Vec<SendView>>,
    /// In-memory generator history, keyed by user. Cleared on `log_out`.
    password_history: HashMap<UserId, Vec<PasswordHistoryEntry>>,
}

// `PasswordManagerClient` doesn't implement `Debug`, so we print just the count
// — needed so `Message` can derive `Debug` with the `ClientManagerLoaded` variant.
impl std::fmt::Debug for ClientManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientManager")
            .field("users", &self.users.len())
            .finish()
    }
}

impl ClientManager {
    /// Initial value while `load()` runs on a background task, so `App::new`
    /// can return and the window can appear before SQLite opens finish.
    pub fn empty() -> Self {
        Self {
            users: HashMap::new(),
            sends: HashMap::new(),
            password_history: HashMap::new(),
        }
    }

    pub async fn load() -> Self {
        Self {
            users: loader::load_users().await,
            sends: HashMap::new(),
            password_history: HashMap::new(),
        }
    }

    /// Cheap-to-clone SDK handle for one user. `PasswordManagerClient` wraps
    /// an internal `Arc`, so cloning bumps a refcount — async tasks capture
    /// this and call SDK methods on it directly without touching the manager.
    pub fn client_for(&self, uid: &UserId) -> Option<PasswordManagerClient> {
        self.users
            .get(uid)
            .map(|e| PasswordManagerClient(e.client.0.clone()))
    }

    /// Snapshot of the data `UnlockData::unlock` needs. Pulled sync at the
    /// call site so the async task never borrows the manager.
    pub fn unlock_data_for(&self, uid: &UserId) -> Option<UnlockData> {
        let e = self.users.get(uid)?;
        Some(UnlockData {
            client: PasswordManagerClient(e.client.0.clone()),
            sdk_user_id: e.sdk_user_id,
            kdf: e.kdf.clone(),
            email: e.email.clone(),
            encrypted_user_key: e.encrypted_user_key.clone(),
            private_key: e.private_key.clone(),
            organizations: e.organizations.clone(),
        })
    }

    /// Snapshot of the data `validate_master_password` needs.
    pub fn validation_data_for(&self, uid: &UserId) -> Option<(PasswordManagerClient, String)> {
        let e = self.users.get(uid)?;
        Some((
            PasswordManagerClient(e.client.0.clone()),
            e.encrypted_user_key.to_string(),
        ))
    }

    /// User IDs sorted by email. `HashMap` order is non-deterministic, so
    /// "first user" callers would otherwise land on a different account each
    /// launch.
    pub fn user_ids(&self) -> Vec<UserId> {
        let mut ids: Vec<UserId> = self.users.keys().copied().collect();
        ids.sort_by(|a, b| self.users[a].email.cmp(&self.users[b].email));
        ids
    }

    /// Callers cache the result on App rather than calling per view rebuild.
    pub fn accounts(&self) -> Vec<AccountEntry> {
        let mut entries: Vec<AccountEntry> = self
            .users
            .values()
            .map(|e| AccountEntry {
                user_id: e.sdk_user_id,
                email: e.email.clone(),
                display_name: e.display_name.clone(),
                server_url: e.server_url.clone(),
                locked: !e.client.is_unlocked(),
            })
            .collect();
        entries.sort_by(|a, b| a.email.cmp(&b.email));
        entries
    }

    pub fn unlock_methods(&self, uid: &UserId) -> Option<UnlockMethods> {
        self.users.get(uid).map(|e| e.unlock_methods.clone())
    }

    pub fn is_unlocked(&self, uid: &UserId) -> bool {
        self.users.get(uid).is_some_and(|e| e.client.is_unlocked())
    }

    pub fn has_users(&self) -> bool {
        !self.users.is_empty()
    }

    pub fn has_unlocked_users(&self) -> bool {
        self.users.values().any(|e| e.client.is_unlocked())
    }

    pub fn lock(&self, uid: &UserId) {
        if let Some(entry) = self.users.get(uid) {
            entry.client.lock();
        }
    }

    /// Clear the user's keystore and remove the entry. The dropped
    /// `Box<UserEntry>` may still be alive inside in-flight async tasks that
    /// cloned the inner client — fine today, but synchronous resource release
    /// (e.g. closing the SQLite handle) would need those tasks to drain first.
    pub fn log_out(&mut self, uid: &UserId) {
        if let Some(entry) = self.users.remove(uid) {
            entry.client.lock();
        }
        self.sends.remove(uid);
        self.password_history.remove(uid);
    }

    /// Seeded from `mock.json` (no sync path).
    pub fn list_organizations(&self, user_id: &UserId) -> Vec<Organization> {
        self.users
            .get(user_id)
            .map(|e| e.organizations.clone())
            .unwrap_or_default()
    }

    /// Collections across all of the user's orgs. Caller filters by
    /// `organization_id` as needed.
    pub fn list_collections(&self, user_id: &UserId) -> Vec<Collection> {
        self.users
            .get(user_id)
            .map(|e| e.collections.clone())
            .unwrap_or_default()
    }

    /// Generate the user's fingerprint phrase (five hyphenated words), used
    /// by the Account → Fingerprint phrase menu. Returns `None` if the user
    /// is unknown or locked (the SDK can't access the private key).
    pub fn user_fingerprint(&self, user_id: &UserId) -> Option<String> {
        let entry = self.users.get(user_id)?;
        entry
            .client
            .platform()
            .user_fingerprint(entry.sdk_user_id.to_string())
            .ok()
    }

    // ── Send stubs ────────────────────────────────────────────────────────
    //
    // Stubbed storage in `self.sends`. No SDK encrypt/decrypt roundtrip yet.
    // When the SDK send flow lands, the create/edit/delete paths will move
    // into `ClientExt`; for now they're sync `&mut self` because no SDK call
    // is involved.

    pub fn list_sends(&self, user_id: &UserId) -> Vec<SendView> {
        let mut list = self.sends.get(user_id).cloned().unwrap_or_default();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// Used when the user picks a row — we always open directly into the
    /// edit form, so this returns the full `SendView`.
    pub fn full_send(&self, user_id: &UserId, send_id: SendId) -> Option<SendView> {
        self.sends
            .get(user_id)
            .and_then(|sends| sends.iter().find(|s| s.id == Some(send_id)).cloned())
    }

    /// Insert or replace a send. New sends (no id) get an id + access_id
    /// generated locally so the UI can render the link placeholder.
    pub fn save_send(&mut self, user_id: UserId, mut view: SendView) -> SendView {
        // Mock-only: a real backend would assign id + access_id on POST and
        // return them. With no server, generate them here.
        let id = *view.id.get_or_insert_with(SendId::new_v4);
        if view.access_id.is_none() {
            view.access_id = Some(format!("{:.16}", uuid::Uuid::new_v4().simple()));
        }

        let entry = self.sends.entry(user_id).or_default();
        if let Some(slot) = entry.iter_mut().find(|s| s.id == Some(id)) {
            *slot = view.clone();
        } else {
            entry.push(view.clone());
        }
        view
    }

    /// In-memory stub; the real SDK flow would call `SendsClient::delete`.
    pub fn delete_send(&mut self, user_id: &UserId, send_id: SendId) {
        if let Some(entry) = self.sends.get_mut(user_id) {
            entry.retain(|s| s.id != Some(send_id));
        }
    }

    // ── Generator history ─────────────────────────────────────────────────

    /// Ordered oldest-first by insertion; the view reverses for display.
    pub fn password_history(&self, user_id: &UserId) -> Vec<PasswordHistoryEntry> {
        self.password_history
            .get(user_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn clear_password_history(&mut self, user_id: &UserId) {
        self.password_history.remove(user_id);
    }

    /// Append a freshly-generated value to the user's history. Returns the
    /// updated snapshot so the caller can ship it straight to the view in
    /// the same `App::update` round-trip.
    pub fn push_history(&mut self, user_id: UserId, value: String) -> Vec<PasswordHistoryEntry> {
        let list = self.password_history.entry(user_id).or_default();
        list.push(PasswordHistoryEntry {
            value,
            created: chrono::Utc::now(),
        });
        list.clone()
    }
}
