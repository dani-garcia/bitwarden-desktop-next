//! Per-user `PasswordManagerClient` management.
//!
//! Users are discovered by listing `*.sqlite` files in `<workspace-root>/data/`
//! and pairing them with metadata from `data/mock.json`. Each user gets a
//! `PasswordManagerClient` whose state registry is backed by that user's
//! SQLite db. [`ClientManager::unlock`] invokes `initialize_user_crypto` and
//! unlocks the in-memory keystore.
//!
//! ## Dev passwords
//!
//! - `alice@example.com` → `password` (Personal, ~20 ciphers)
//! - `alice@acmecorp.com` → `123456` (Work, ~10 ciphers)
//! - `loadtest@example.com` → `loadtest` (Load Test, ~20k ciphers)
//!
//! Regenerate `data/` via `cargo run -p fake-data`.

use std::{
    collections::HashMap,
    io::BufReader,
    path::Path,
    str::FromStr,
    sync::{Arc, Mutex, RwLock},
};

use bitwarden_collections::collection::CollectionId;
use bitwarden_core::{
    ClientBuilder, ClientSettings, OrganizationId, UserId,
    key_management::{
        LocalUserDataKeyState, MasterPasswordUnlockData, SymmetricKeySlotId,
        account_cryptographic_state::WrappedAccountCryptographicState,
        crypto::{InitOrgCryptoRequest, InitUserCryptoMethod, InitUserCryptoRequest},
    },
};
use bitwarden_crypto::{EncString, Kdf, UnsignedSharedKey};
use bitwarden_generators::{
    PassphraseGeneratorRequest, PasswordGeneratorRequest, UsernameGeneratorRequest,
};
use bitwarden_pm::PasswordManagerClient;
use bitwarden_send::{SendId, SendView};
use bitwarden_state::{
    DatabaseConfiguration,
    registry::StateRegistry,
    repository::{Repository, RepositoryError, RepositoryItem},
};
use bitwarden_vault::{Cipher, CipherId, CipherListView, CipherView, Folder, FolderView};
use serde::Deserialize;

use crate::domain::UnlockMethods;

// ── mock.json schema ───────────────────────────────────────────────────────
// Mirrored in `tools/fake-data/src/main.rs`. Keep field names + types in sync.

#[derive(Deserialize)]
struct MockVaultMeta {
    users: Vec<MockUserMeta>,
}

#[derive(Deserialize, Clone)]
struct MockUserMeta {
    user_id: UserId,
    email: String,
    display_name: String,
    server_url: String,
    #[expect(dead_code)] // Read by debug helpers, not by the runtime unlock path.
    master_password_dev_only: String,
    unlock_methods: UnlockMethodsCfg,
    kdf: Kdf,
    encrypted_user_key: EncString,
    private_key: EncString,
    #[serde(default)]
    organizations: Vec<Organization>,
    #[serde(default)]
    collections: Vec<Collection>,
}

/// App-level org metadata. The SDK has no `Repository<Organization>` for
/// this UI-only stub, so we read it from `mock.json` and hold it here.
#[derive(Deserialize, Clone, Debug)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    /// Org's symmetric key, wrapped with the user's public key. Replayed on
    /// `unlock` via `initialize_org_crypto`. `None` for orgs added without a key.
    #[serde(default)]
    pub wrapped_key: Option<UnsignedSharedKey>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Collection {
    pub id: CollectionId,
    pub organization_id: OrganizationId,
    pub name: String,
}

#[derive(Deserialize, Clone)]
struct UnlockMethodsCfg {
    master_password: bool,
    pin: bool,
    biometrics: bool,
}

// ── Public client manager ──────────────────────────────────────────────────

/// UI-facing snapshot of a single user account. App caches one
/// `Vec<AccountEntry>` and hands it to views via `RenderCtx::accounts`.
pub struct AccountEntry {
    pub user_id: UserId,
    pub email: String,
    #[expect(dead_code)] // Not displayed yet; reserved for future avatar / profile views.
    pub display_name: String,
    pub server_url: String,
    pub locked: bool,
}

struct UserEntry {
    client: PasswordManagerClient,
    // App-side stand-in for SDK profile methods that don't exist yet.
    email: String,
    display_name: String,
    server_url: String,
    unlock_methods: UnlockMethods,
    /// SDK-side UUID, parsed from the SQLite filename. The SDK binds a
    /// `UserId` to a `Client` on first `initialize_user_crypto`, so lock/unlock
    /// cycles MUST pass the same ID.
    sdk_user_id: UserId,
    kdf: Kdf,
    encrypted_user_key: EncString,
    private_key: EncString,
    /// Seeded from `mock.json` (no sync path in this stub).
    organizations: Vec<Organization>,
    collections: Vec<Collection>,
}

pub struct ClientManager {
    /// Entries are `Arc` so async methods can clone the handle out of the guard
    /// and drop the lock before awaiting — holding a read guard across `.await`
    /// risks deadlock with writers and is flagged by clippy.
    users: RwLock<HashMap<UserId, Arc<UserEntry>>>,
    /// In-memory send store, keyed by user. The real SDK flow will encrypt and
    /// persist via `Repository<Send>`; until then we hold decrypted `SendView`
    /// values directly. See `docs/todo.md`.
    sends: RwLock<HashMap<UserId, Vec<SendView>>>,
    /// In-memory generator history, keyed by user. Cleared on `log_out`.
    password_history: RwLock<HashMap<UserId, Vec<PasswordHistoryEntry>>>,
}

/// One entry in the in-memory generator history. Flat across password /
/// passphrase / username — matches the official Bitwarden client.
#[derive(Debug, Clone)]
pub struct PasswordHistoryEntry {
    pub value: String,
    pub created: chrono::DateTime<chrono::Utc>,
}

// `PasswordManagerClient` doesn't implement `Debug`, so we print just the count
// — needed so `Message` can derive `Debug` with the `ClientManagerLoaded` variant.
impl std::fmt::Debug for ClientManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientManager")
            .field("users", &self.users.read().unwrap().len())
            .finish()
    }
}

pub trait ClientExt {
    fn is_unlocked(&self) -> bool;
    fn lock(&self);
}

impl ClientExt for PasswordManagerClient {
    fn is_unlocked(&self) -> bool {
        self.0
            .internal
            .get_key_store()
            .context()
            .has_symmetric_key(SymmetricKeySlotId::User)
    }

    fn lock(&self) {
        self.0.internal.get_key_store().clear();
    }
}

use crate::paths::data_dir;

impl ClientManager {
    /// Initial value while `load()` runs on a background task, so `App::new`
    /// can return and the window can appear before SQLite opens finish.
    pub fn empty() -> Self {
        Self {
            users: RwLock::new(HashMap::new()),
            sends: RwLock::new(HashMap::new()),
            password_history: RwLock::new(HashMap::new()),
        }
    }

    pub fn verify_data_dir() {
        let data_dir = data_dir();
        let meta_path = data_dir.join("mock.json");
        if !meta_path.is_file() {
            tracing::error!(
                path = %meta_path.display(),
                "mock.json not found; regenerate via `cargo run -p fake-data`"
            );
            std::process::exit(0);
        }
    }

    pub async fn load() -> Self {
        let data_dir = data_dir();
        let meta_path = data_dir.join("mock.json");
        let file = std::fs::File::open(&meta_path)
            .expect("failed to open mock data directory; regenerate via `cargo run -p fake-data`");
        let meta: MockVaultMeta = serde_json::from_reader(BufReader::new(file))
            .expect("mock.json is malformed; regenerate via `cargo run -p fake-data`");

        let meta_by_id: HashMap<UserId, MockUserMeta> =
            meta.users.into_iter().map(|u| (u.user_id, u)).collect();

        let mut users = HashMap::with_capacity(meta_by_id.len());
        let entries = std::fs::read_dir(&data_dir)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", data_dir.display()));
        for entry in entries {
            let path = entry.expect("directory entry readable").path();
            if path.extension().and_then(|s| s.to_str()) != Some("sqlite") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let Ok(uid) = UserId::from_str(stem) else {
                tracing::warn!(file = %path.display(), "skipping sqlite file with non-UUID name");
                continue;
            };
            let Some(mu) = meta_by_id.get(&uid).cloned() else {
                tracing::warn!(user_id = %uid, "sqlite file has no entry in mock.json, skipping");
                continue;
            };
            tracing::debug!(user_id = %uid, email = %mu.email, "loading mock user");
            users.insert(uid, Arc::new(build_user_entry(mu, &data_dir).await));
        }

        tracing::info!(users = users.len(), "ClientManager loaded");
        Self {
            users: RwLock::new(users),
            sends: RwLock::new(HashMap::new()),
            password_history: RwLock::new(HashMap::new()),
        }
    }

    /// User IDs sorted by email. `HashMap` order is non-deterministic, so
    /// "first user" callers would otherwise land on a different account each
    /// launch.
    pub fn user_ids(&self) -> Vec<UserId> {
        let users = self.users.read().unwrap();
        let mut ids: Vec<UserId> = users.keys().copied().collect();
        ids.sort_by(|a, b| users[a].email.cmp(&users[b].email));
        ids
    }

    /// One `users.read()` per call; callers cache the result on App rather
    /// than calling per view rebuild.
    pub fn accounts(&self) -> Vec<AccountEntry> {
        let users = self.users.read().unwrap();
        let mut entries: Vec<AccountEntry> = users
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
        self.users
            .read()
            .unwrap()
            .get(uid)
            .map(|e| e.unlock_methods.clone())
    }

    pub fn is_unlocked(&self, uid: &UserId) -> bool {
        self.users
            .read()
            .unwrap()
            .get(uid)
            .is_some_and(|e| e.client.is_unlocked())
    }

    pub fn has_users(&self) -> bool {
        !self.users.read().unwrap().is_empty()
    }

    pub fn has_unlocked_users(&self) -> bool {
        self.users
            .read()
            .unwrap()
            .values()
            .any(|e| e.client.is_unlocked())
    }

    pub fn lock(&self, uid: &UserId) {
        let users = self.users.read().unwrap();
        if let Some(entry) = users.get(uid) {
            entry.client.lock();
        }
    }

    pub fn lock_all(&self) {
        let users = self.users.read().unwrap();
        for entry in users.values() {
            entry.client.lock();
        }
    }

    /// Clear the user's keystore and remove the entry. Named `log_out` (not
    /// `remove`) because the SDK will want server-side token revocation and
    /// local SQLite cleanup on this transition in the future.
    ///
    /// TODO: migrate to `async fn log_out(...) -> Result<(), _>` when the SDK
    /// exposes a real logout path. The `Arc<UserEntry>` dropped here may still
    /// be alive inside in-flight async tasks that cloned it — acceptable today,
    /// but future cleanup requiring synchronous resource release (e.g. closing
    /// the SQLite handle) needs those tasks to complete first.
    pub fn log_out(&self, uid: &UserId) {
        let mut users = self.users.write().unwrap();
        if let Some(entry) = users.remove(uid) {
            entry.client.lock();
        }
        self.sends.write().unwrap().remove(uid);
        self.password_history.write().unwrap().remove(uid);
    }

    /// Initialize the SDK crypto state with the user's master password. On
    /// success the keystore is unlocked in memory.
    pub async fn unlock(&self, user_id: &UserId, password: String) -> Result<(), String> {
        let entry = self
            .users
            .read()
            .unwrap()
            .get(user_id)
            .cloned()
            .ok_or_else(|| format!("unknown user {user_id}"))?;

        let req = InitUserCryptoRequest {
            // Reuse the Client's bound UserId on every unlock — see the
            // invariant on `UserEntry::sdk_user_id`.
            user_id: Some(entry.sdk_user_id),
            kdf_params: entry.kdf.clone(),
            email: entry.email.clone(),
            account_cryptographic_state: WrappedAccountCryptographicState::V1 {
                private_key: entry.private_key.clone(),
            },
            method: InitUserCryptoMethod::MasterPasswordUnlock {
                password,
                master_password_unlock: MasterPasswordUnlockData {
                    kdf: entry.kdf.clone(),
                    master_key_wrapped_user_key: entry.encrypted_user_key.clone(),
                    salt: entry.email.clone(),
                },
            },
            upgrade_token: None,
        };

        entry
            .client
            .crypto()
            .initialize_user_crypto(req)
            .await
            .map_err(|e| e.to_string())?;

        // Replay org keys so the vault client can encrypt/decrypt org-owned
        // ciphers. Orgs without a wrapped key contribute nothing.
        let org_keys: HashMap<OrganizationId, UnsignedSharedKey> = entry
            .organizations
            .iter()
            .filter_map(|o| o.wrapped_key.clone().map(|k| (o.id, k)))
            .collect();
        if !org_keys.is_empty() {
            entry
                .client
                .crypto()
                .initialize_org_crypto(InitOrgCryptoRequest {
                    organization_keys: org_keys,
                })
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// Requires `unlock` to have been called first.
    pub async fn list_ciphers(&self, user_id: &UserId) -> Result<Vec<CipherListView>, String> {
        let entry = self
            .users
            .read()
            .unwrap()
            .get(user_id)
            .cloned()
            .ok_or_else(|| format!("unknown user {user_id}"))?;

        let repo = entry
            .client
            .platform()
            .state()
            .get::<Cipher>()
            .map_err(|e| e.to_string())?;
        let ciphers = repo.list().await.map_err(|e| e.to_string())?;

        let mut list: Vec<CipherListView> = entry
            .client
            .vault()
            .ciphers()
            .decrypt_list(ciphers)
            .await
            .map_err(|e| e.to_string())?;

        list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(list)
    }

    /// Fully decrypt a single cipher (including secrets). Used by the detail pane.
    pub async fn full_cipher(
        &self,
        user_id: &UserId,
        cipher_id: CipherId,
    ) -> Result<CipherView, String> {
        let entry = self
            .users
            .read()
            .unwrap()
            .get(user_id)
            .cloned()
            .ok_or_else(|| format!("unknown user {user_id}"))?;

        let repo = entry
            .client
            .platform()
            .state()
            .get::<Cipher>()
            .map_err(|e| e.to_string())?;
        let cipher = repo
            .get(cipher_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("cipher {cipher_id} not found"))?;

        entry
            .client
            .vault()
            .ciphers()
            .decrypt(cipher)
            .await
            .map_err(|e| e.to_string())
    }

    /// Encrypt an edited `CipherView` and persist to the local SQLite repo.
    /// Bypasses the server PUT in `CiphersClient::edit()` (no backend). Returns
    /// the re-decrypted view so callers can refresh with normalizations the
    /// encrypt step applied (cipher key, TOTP checksums, …).
    ///
    /// NOTE: skips password-history tracking that `CiphersClient::edit()` does
    /// internally via non-public types — acceptable while there's no backend.
    pub async fn save_cipher(
        &self,
        user_id: &UserId,
        cipher_view: CipherView,
    ) -> Result<CipherView, String> {
        let entry = self
            .users
            .read()
            .unwrap()
            .get(user_id)
            .cloned()
            .ok_or_else(|| format!("unknown user {user_id}"))?;

        let ctx = entry
            .client
            .vault()
            .ciphers()
            .encrypt(cipher_view)
            .await
            .map_err(|e| e.to_string())?;
        let cipher = ctx.cipher;
        let id = cipher
            .id
            .ok_or_else(|| "encrypted cipher missing id".to_string())?;

        let repo = entry
            .client
            .platform()
            .state()
            .get::<Cipher>()
            .map_err(|e| e.to_string())?;
        repo.set(id, cipher.clone())
            .await
            .map_err(|e| e.to_string())?;

        entry
            .client
            .vault()
            .ciphers()
            .decrypt(cipher)
            .await
            .map_err(|e| e.to_string())
    }

    /// Soft-delete by marking `deleted_date` and writing it back. Skips the
    /// server PUT in `CiphersClient::soft_delete()` (no backend).
    pub async fn soft_delete_cipher(
        &self,
        user_id: &UserId,
        cipher_id: CipherId,
    ) -> Result<(), String> {
        let entry = self
            .users
            .read()
            .unwrap()
            .get(user_id)
            .cloned()
            .ok_or_else(|| format!("unknown user {user_id}"))?;

        let repo = entry
            .client
            .platform()
            .state()
            .get::<Cipher>()
            .map_err(|e| e.to_string())?;
        let mut cipher = repo
            .get(cipher_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("cipher {cipher_id} not found"))?;
        // `Cipher::soft_delete()` is `pub(crate)` in bitwarden-vault, but
        // `deleted_date` is public so we set it directly.
        cipher.deleted_date = Some(chrono::Utc::now());
        repo.set(cipher_id, cipher).await.map_err(|e| e.to_string())
    }

    pub async fn list_folders(&self, user_id: &UserId) -> Result<Vec<FolderView>, String> {
        let entry = self
            .users
            .read()
            .unwrap()
            .get(user_id)
            .cloned()
            .ok_or_else(|| format!("unknown user {user_id}"))?;

        let repo = entry
            .client
            .platform()
            .state()
            .get::<Folder>()
            .map_err(|e| e.to_string())?;
        let folders = repo.list().await.map_err(|e| e.to_string())?;

        entry
            .client
            .vault()
            .folders()
            .decrypt_list(folders)
            .map_err(|e| e.to_string())
    }

    /// Seeded from `mock.json` (no sync path).
    pub fn list_organizations(&self, user_id: &UserId) -> Vec<Organization> {
        self.users
            .read()
            .unwrap()
            .get(user_id)
            .map(|e| e.organizations.clone())
            .unwrap_or_default()
    }

    /// Collections across all of the user's orgs. Caller filters by
    /// `organization_id` as needed.
    pub fn list_collections(&self, user_id: &UserId) -> Vec<Collection> {
        self.users
            .read()
            .unwrap()
            .get(user_id)
            .map(|e| e.collections.clone())
            .unwrap_or_default()
    }

    // ── Send stubs ────────────────────────────────────────────────────────
    //
    // Stubbed storage in `self.sends`. No SDK encrypt/decrypt roundtrip yet.
    // When the SDK send flow lands, swap these bodies for real client calls;
    // the signatures are already a superset of what the real flow needs.

    /// Async so call sites can `Task::perform` the same way they do for
    /// ciphers — swapping in a real decrypt step later won't ripple.
    pub async fn list_sends(&self, user_id: &UserId) -> Result<Vec<SendView>, String> {
        let mut list = self
            .sends
            .read()
            .unwrap()
            .get(user_id)
            .cloned()
            .unwrap_or_default();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(list)
    }

    /// Used when the user picks a row — we always open directly into the
    /// edit form, so this returns the full `SendView`.
    pub async fn full_send(&self, user_id: &UserId, send_id: SendId) -> Result<SendView, String> {
        self.sends
            .read()
            .unwrap()
            .get(user_id)
            .and_then(|sends| sends.iter().find(|s| s.id == Some(send_id)).cloned())
            .ok_or_else(|| format!("send {send_id} not found"))
    }

    /// Insert or replace a send. New sends (no id) get an id + access_id
    /// generated locally so the UI can render the link placeholder.
    pub async fn save_send(
        &self,
        user_id: &UserId,
        mut view: SendView,
    ) -> Result<SendView, String> {
        if view.id.is_none() {
            view.id = Some(SendId::new_v4());
        }
        if view.access_id.is_none() {
            // Placeholder until the SDK generates a real one on create.
            view.access_id = Some(
                uuid::Uuid::new_v4()
                    .simple()
                    .to_string()
                    .chars()
                    .take(16)
                    .collect(),
            );
        }

        let id = view.id.expect("id generated above");
        let mut sends = self.sends.write().unwrap();
        let entry = sends.entry(*user_id).or_default();
        if let Some(slot) = entry.iter_mut().find(|s| s.id == Some(id)) {
            *slot = view.clone();
        } else {
            entry.push(view.clone());
        }
        Ok(view)
    }

    /// In-memory stub; the real SDK flow would call `SendsClient::delete`.
    pub async fn delete_send(&self, user_id: &UserId, send_id: SendId) -> Result<(), String> {
        let mut sends = self.sends.write().unwrap();
        if let Some(entry) = sends.get_mut(user_id) {
            entry.retain(|s| s.id != Some(send_id));
        }
        Ok(())
    }

    // ── Generator ─────────────────────────────────────────────────────────
    //
    // Each `generate_*` pushes the value into `password_history` and returns
    // the fresh snapshot alongside it. One round-trip spares the view a list
    // call after every regenerate — the generator auto-regenerates on every
    // option change, so this shaves one Task per keystroke.

    /// Sync in the SDK; wrapped in `async fn` so call sites use the same
    /// `Task::perform` path as the other generator methods.
    pub async fn generate_password(
        &self,
        user_id: &UserId,
        req: PasswordGeneratorRequest,
    ) -> Result<(String, Vec<PasswordHistoryEntry>), String> {
        let entry = self
            .users
            .read()
            .unwrap()
            .get(user_id)
            .cloned()
            .ok_or_else(|| format!("unknown user {user_id}"))?;
        let value = entry
            .client
            .generator()
            .password(req)
            .map_err(|e| e.to_string())?;
        Ok((value.clone(), self.push_history(user_id, value)))
    }

    pub async fn generate_passphrase(
        &self,
        user_id: &UserId,
        req: PassphraseGeneratorRequest,
    ) -> Result<(String, Vec<PasswordHistoryEntry>), String> {
        let entry = self
            .users
            .read()
            .unwrap()
            .get(user_id)
            .cloned()
            .ok_or_else(|| format!("unknown user {user_id}"))?;
        let value = entry
            .client
            .generator()
            .passphrase(req)
            .map_err(|e| e.to_string())?;
        Ok((value.clone(), self.push_history(user_id, value)))
    }

    /// SDK `username` is `async` because the `Forwarded` variant does HTTP;
    /// `Word`/`Subaddress`/`Catchall` resolve synchronously inside the future.
    pub async fn generate_username(
        &self,
        user_id: &UserId,
        req: UsernameGeneratorRequest,
    ) -> Result<(String, Vec<PasswordHistoryEntry>), String> {
        let entry = self
            .users
            .read()
            .unwrap()
            .get(user_id)
            .cloned()
            .ok_or_else(|| format!("unknown user {user_id}"))?;
        let value = entry
            .client
            .generator()
            .username(req)
            .await
            .map_err(|e| e.to_string())?;
        Ok((value.clone(), self.push_history(user_id, value)))
    }

    /// Ordered oldest-first by insertion; the view reverses for display.
    pub fn password_history(&self, user_id: &UserId) -> Vec<PasswordHistoryEntry> {
        self.password_history
            .read()
            .unwrap()
            .get(user_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn clear_password_history(&self, user_id: &UserId) {
        self.password_history.write().unwrap().remove(user_id);
    }

    fn push_history(&self, user_id: &UserId, value: String) -> Vec<PasswordHistoryEntry> {
        let mut store = self.password_history.write().unwrap();
        let list = store.entry(*user_id).or_default();
        list.push(PasswordHistoryEntry {
            value,
            created: chrono::Utc::now(),
        });
        list.clone()
    }
}

// ── Construction helpers ───────────────────────────────────────────────────

async fn build_user_entry(mu: MockUserMeta, data_dir: &Path) -> UserEntry {
    // TODO: migrate to `PasswordManagerClient::load_from_state` once the SDK
    // exposes it. We hand-assemble the client because `PasswordManagerClient::new`
    // pre-sets the database `OnceLock` to a memory db, blocking our per-user
    // `initialize_database` call.
    let token_handler =
        Arc::new(bitwarden_auth::token_management::PasswordManagerTokenHandler::default());
    let inner = ClientBuilder::new()
        .with_token_handler(token_handler)
        .with_settings(ClientSettings {
            identity_url: "http://localhost:8080/identity".to_string(),
            api_url: "http://localhost:8080/api".to_string(),
            ..Default::default()
        })
        .with_state(StateRegistry::new())
        .build();
    let client = PasswordManagerClient(inner);

    // `LocalUserDataKeyState` isn't in `get_sdk_managed_migrations()` but
    // `initialize_user_crypto` writes to it during unlock — register an empty
    // in-memory repo so unlock doesn't fail. Everything else comes from the
    // SDK-managed SQLite DB.
    register_empty_repo::<LocalUserDataKeyState>(&client);

    client
        .platform()
        .state()
        .initialize_database(
            DatabaseConfiguration::Sqlite {
                db_name: mu.user_id.to_string(),
                folder_path: data_dir.to_path_buf(),
            },
            bitwarden_pm::migrations::get_sdk_managed_migrations(),
        )
        .await
        .expect("sqlite database init must succeed");

    UserEntry {
        client,
        email: mu.email,
        display_name: mu.display_name,
        server_url: mu.server_url,
        unlock_methods: UnlockMethods {
            master_password: mu.unlock_methods.master_password,
            pin: mu.unlock_methods.pin,
            biometrics: mu.unlock_methods.biometrics,
        },
        sdk_user_id: mu.user_id,
        kdf: mu.kdf,
        encrypted_user_key: mu.encrypted_user_key,
        private_key: mu.private_key,
        organizations: mu.organizations,
        collections: mu.collections,
    }
}

fn register_empty_repo<T: RepositoryItem + Clone>(client: &PasswordManagerClient) {
    let repo = Arc::new(MemoryRepo::<T> {
        data: Mutex::new(HashMap::new()),
    });
    client.platform().state().register_client_managed(repo);
}

// ── In-memory repository ───────────────────────────────────────────────────
// Kept only for `LocalUserDataKeyState`, which isn't part of the SDK-managed
// migration list but is written to during crypto init.

struct MemoryRepo<T: RepositoryItem + Clone> {
    data: Mutex<HashMap<String, T>>,
}

#[async_trait::async_trait]
impl<T: RepositoryItem + Clone> Repository<T> for MemoryRepo<T> {
    async fn get(&self, key: T::Key) -> Result<Option<T>, RepositoryError> {
        Ok(self.data.lock().unwrap().get(&key.to_string()).cloned())
    }
    async fn list(&self) -> Result<Vec<T>, RepositoryError> {
        Ok(self.data.lock().unwrap().values().cloned().collect())
    }
    async fn set(&self, key: T::Key, value: T) -> Result<(), RepositoryError> {
        self.data.lock().unwrap().insert(key.to_string(), value);
        Ok(())
    }
    async fn set_bulk(&self, values: Vec<(T::Key, T)>) -> Result<(), RepositoryError> {
        let mut map = self.data.lock().unwrap();
        for (k, v) in values {
            map.insert(k.to_string(), v);
        }
        Ok(())
    }
    async fn remove(&self, key: T::Key) -> Result<(), RepositoryError> {
        self.data.lock().unwrap().remove(&key.to_string());
        Ok(())
    }
    async fn remove_bulk(&self, keys: Vec<T::Key>) -> Result<(), RepositoryError> {
        let mut map = self.data.lock().unwrap();
        for k in keys {
            map.remove(&k.to_string());
        }
        Ok(())
    }
    async fn remove_all(&self) -> Result<(), RepositoryError> {
        self.data.lock().unwrap().clear();
        Ok(())
    }
}
