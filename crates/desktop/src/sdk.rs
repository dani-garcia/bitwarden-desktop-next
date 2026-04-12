//! Per-user `PasswordManagerClient` management.
//!
//! At startup we deserialize `assets/mock-vault.json` (produced by the `fake-data` binary)
//! and build one `PasswordManagerClient` per user, pre-populating its `Cipher`/`Folder`
//! repos with the encrypted data from the JSON. The user's master password remains needed
//! to call [`ClientManager::unlock`], which in turn invokes
//! `crypto().initialize_user_crypto(...)` and unlocks the in-memory keystore.
//!
//! ## Dev passwords
//!
//! - `alice@example.com` → `password` (Personal, ~20 ciphers)
//! - `alice@acmecorp.com` → `123456` (Work, ~10 ciphers)
//! - `loadtest@example.com` → `loadtest` (Load Test, ~20k ciphers)
//!
//! Regenerate the JSON via `cargo run -p fake-data`.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bitwarden_core::{
    ClientSettings, UserId,
    key_management::{
        LocalUserDataKeyState, MasterPasswordUnlockData, SymmetricKeyId, UserKeyState,
        account_cryptographic_state::WrappedAccountCryptographicState,
        crypto::{InitUserCryptoMethod, InitUserCryptoRequest},
    },
};
use bitwarden_crypto::{EncString, Kdf};
use bitwarden_pm::PasswordManagerClient;
use bitwarden_state::repository::{Repository, RepositoryError, RepositoryItem};
use bitwarden_vault::{Cipher, CipherId, CipherListView, CipherView, Folder};
use serde::Deserialize;

use crate::state::{UnlockMethods, UserId as DesktopUserId};

const MOCK_VAULT_JSON: &[u8] = include_bytes!("../../../assets/mock-vault.json");

// ── Mock vault file schema ─────────────────────────────────────────────────
//
// Mirrored in `tools/fake-data/src/main.rs`. Keep field names + types in sync.

#[derive(Deserialize)]
struct MockVaultFile {
    users: Vec<MockUser>,
}

#[derive(Deserialize)]
struct MockUser {
    user_id: String,
    email: String,
    display_name: String,
    server_url: String,
    #[expect(dead_code)] // Read by tests / debug helpers, not by the runtime unlock path.
    master_password_dev_only: String,
    unlock_methods: UnlockMethodsCfg,
    kdf: Kdf,
    encrypted_user_key: EncString,
    private_key: EncString,
    ciphers: Vec<Cipher>,
    folders: Vec<Folder>,
}

#[derive(Deserialize)]
struct UnlockMethodsCfg {
    master_password: bool,
    pin: bool,
    biometrics: bool,
}

// ── Public client manager ──────────────────────────────────────────────────

struct UserEntry {
    client: PasswordManagerClient,
    // User profile data. These fields are the app-side stand-in for SDK
    // profile methods that don't exist yet. When the real SDK exposes
    // profile info on the client, the corresponding `ClientManager`
    // accessors can just delegate instead of reading these fields.
    email: String,
    display_name: String,
    server_url: String,
    unlock_methods: UnlockMethods,
    /// Stable SDK-side UUID for this user. Parsed from the mock-vault JSON at
    /// load time and reused on every `unlock()`. The SDK binds a `UserId` to a
    /// `Client` on first `initialize_user_crypto`, so lock/unlock cycles MUST
    /// pass the same ID — passing a fresh `UserId::new_v4()` each time would
    /// make the Client think it's serving a different user.
    sdk_user_id: UserId,
    // Crypto inputs needed by `unlock()`. Stored once at load time so the unlock path
    // doesn't need to re-parse JSON.
    kdf: Kdf,
    encrypted_user_key: EncString,
    private_key: EncString,
}

pub struct ClientManager {
    users: HashMap<DesktopUserId, UserEntry>,
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
            .has_symmetric_key(SymmetricKeyId::User)
    }

    fn lock(&self) {
        self.0.internal.get_key_store().clear();
    }
}

impl ClientManager {
    /// Load all users from the embedded mock vault JSON.
    pub fn load() -> Self {
        let parsed: MockVaultFile = serde_json::from_slice(MOCK_VAULT_JSON)
            .expect("mock-vault.json is malformed; regenerate via `cargo run -p fake-data`");

        let mut users = HashMap::with_capacity(parsed.users.len());
        for mu in parsed.users {
            tracing::debug!(
                user_id = %mu.user_id,
                email = %mu.email,
                ciphers = mu.ciphers.len(),
                folders = mu.folders.len(),
                "loaded mock user"
            );
            users.insert(mu.user_id.clone(), build_user_entry(mu));
        }

        tracing::info!(users = users.len(), "ClientManager loaded");
        Self { users }
    }

    pub fn user_ids(&self) -> impl Iterator<Item = &DesktopUserId> {
        self.users.keys()
    }

    pub fn email(&self, uid: &str) -> Option<&str> {
        self.users.get(uid).map(|e| e.email.as_str())
    }

    pub fn display_name(&self, uid: &str) -> Option<&str> {
        self.users.get(uid).map(|e| e.display_name.as_str())
    }

    pub fn server_url(&self, uid: &str) -> Option<&str> {
        self.users.get(uid).map(|e| e.server_url.as_str())
    }

    pub fn unlock_methods(&self, uid: &str) -> Option<&UnlockMethods> {
        self.users.get(uid).map(|e| &e.unlock_methods)
    }

    pub fn is_unlocked(&self, uid: &str) -> bool {
        self.users
            .get(uid)
            .is_some_and(|e| e.client.is_unlocked())
    }

    pub fn has_users(&self) -> bool {
        !self.users.is_empty()
    }

    pub fn has_unlocked_users(&self) -> bool {
        self.users.values().any(|e| e.client.is_unlocked())
    }

    /// Lock all users by clearing their crypto keystores.
    pub fn lock_all(&self) {
        for entry in self.users.values() {
            entry.client.lock();
        }
    }

    /// Initialize the SDK crypto state for the given user with their master password.
    /// On success, the user's keystore is unlocked in memory and subsequent decrypt
    /// calls will succeed. Returns an error if the user is unknown or the password is wrong.
    pub async fn unlock(&self, user_id: &str, password: String) -> Result<(), String> {
        let entry = self
            .users
            .get(user_id)
            .ok_or_else(|| format!("unknown user {user_id}"))?;

        let req = InitUserCryptoRequest {
            // Stable: reuse the Client's bound UserId on every unlock. See the
            // comment on `UserEntry::sdk_user_id` for the invariant.
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
            .map_err(|e| e.to_string())
    }

    /// Decrypt all ciphers belonging to the given user. Requires `unlock` to have been called
    /// first; otherwise the SDK keystore is empty and decryption fails.
    pub async fn list_ciphers(&self, user_id: &str) -> Result<Vec<CipherListView>, String> {
        let entry = self
            .users
            .get(user_id)
            .ok_or_else(|| format!("unknown user {user_id}"))?;

        let repo = entry
            .client
            .platform()
            .state()
            .get::<Cipher>()
            .map_err(|e| e.to_string())?;
        let ciphers = repo.list().await.map_err(|e| e.to_string())?;

        entry
            .client
            .vault()
            .ciphers()
            .decrypt_list(ciphers)
            .map_err(|e| e.to_string())
    }

    /// Fully decrypt a single cipher (including secrets like passwords). Used when the user
    /// opens the detail pane.
    pub async fn full_cipher(
        &self,
        user_id: &str,
        cipher_id: CipherId,
    ) -> Result<CipherView, String> {
        let entry = self
            .users
            .get(user_id)
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
            .map_err(|e| e.to_string())
    }
}

// ── Construction helpers ───────────────────────────────────────────────────

fn build_user_entry(mu: MockUser) -> UserEntry {
    let client = PasswordManagerClient::new(Some(ClientSettings {
        identity_url: "http://localhost:8080/identity".to_string(),
        api_url: "http://localhost:8080/api".to_string(),
        ..Default::default()
    }));

    // Stable SDK-side UUID: the mock JSON's `user_id` field is itself a valid
    // UUID (fake-data hardcodes stable v4 UUIDs per spec). Parsing fails loud
    // if that invariant is ever broken.
    let sdk_user_id = UserId::new(uuid::Uuid::parse_str(&mu.user_id).expect(
        "mock-vault user_id must be a valid UUID; regenerate via `cargo run -p fake-data`",
    ));

    // `initialize_user_crypto` writes to UserKeyState + LocalUserDataKeyState; cipher/folder
    // repos hold the encrypted vault data. All four must be registered before unlock.
    register_empty_repo::<UserKeyState>(&client);
    register_empty_repo::<LocalUserDataKeyState>(&client);

    let cipher_map: HashMap<String, Cipher> = mu
        .ciphers
        .into_iter()
        .map(|c| {
            let key =
                c.id.map(|id| id.to_string())
                    .expect("generated ciphers always have an id");
            (key, c)
        })
        .collect();
    let folder_map: HashMap<String, Folder> = mu
        .folders
        .into_iter()
        .map(|f| {
            let key =
                f.id.map(|id| id.to_string())
                    .expect("generated folders always have an id");
            (key, f)
        })
        .collect();

    let cipher_repo = Arc::new(MemoryRepo::<Cipher> {
        data: Mutex::new(cipher_map),
    });
    let folder_repo = Arc::new(MemoryRepo::<Folder> {
        data: Mutex::new(folder_map),
    });
    client
        .platform()
        .state()
        .register_client_managed(cipher_repo);
    client
        .platform()
        .state()
        .register_client_managed(folder_repo);

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
        sdk_user_id,
        kdf: mu.kdf,
        encrypted_user_key: mu.encrypted_user_key,
        private_key: mu.private_key,
    }
}

fn register_empty_repo<T: RepositoryItem + Clone>(client: &PasswordManagerClient) {
    let repo = Arc::new(MemoryRepo::<T> {
        data: Mutex::new(HashMap::new()),
    });
    client.platform().state().register_client_managed(repo);
}

// ── In-memory repository ───────────────────────────────────────────────────

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unlock_user_1_with_correct_password() {
        let mgr = ClientManager::load();
        mgr.unlock("11111111-1111-4111-a111-111111111111", "password".to_string())
            .await
            .expect("personal account should unlock with the dev password");
    }

    #[tokio::test]
    async fn unlock_user_2_with_correct_password() {
        let mgr = ClientManager::load();
        mgr.unlock("22222222-2222-4222-a222-222222222222", "123456".to_string())
            .await
            .expect("work account should unlock with the dev password");
    }

    #[tokio::test]
    async fn unlock_with_wrong_password_fails() {
        let mgr = ClientManager::load();
        let result = mgr.unlock("11111111-1111-4111-a111-111111111111", "hunter2".to_string()).await;
        assert!(result.is_err(), "wrong password should fail unlock");
    }

    #[tokio::test]
    async fn list_ciphers_after_unlock_returns_decrypted_items() {
        let mgr = ClientManager::load();
        mgr.unlock("11111111-1111-4111-a111-111111111111", "password".to_string()).await.unwrap();
        let items = mgr
            .list_ciphers("11111111-1111-4111-a111-111111111111")
            .await
            .expect("decrypt_list should succeed after unlock");
        assert!(!items.is_empty(), "personal vault should have items");
        // The Gmail mock entry has a plaintext name we can recognize.
        assert!(
            items.iter().any(|i| i.name == "Gmail"),
            "expected the Gmail entry to round-trip its plaintext name"
        );
    }

    #[tokio::test]
    async fn full_cipher_after_unlock_returns_login_view() {
        let mgr = ClientManager::load();
        mgr.unlock("11111111-1111-4111-a111-111111111111", "password".to_string()).await.unwrap();
        let list = mgr.list_ciphers("11111111-1111-4111-a111-111111111111").await.unwrap();
        let gmail_id = list
            .iter()
            .find(|i| i.name == "Gmail")
            .and_then(|i| i.id)
            .expect("Gmail item should exist with an id");

        let view = mgr
            .full_cipher("11111111-1111-4111-a111-111111111111", gmail_id)
            .await
            .expect("decrypt should succeed for a known cipher");
        let login = view.login.expect("Gmail is a login cipher");
        assert_eq!(login.username.as_deref(), Some("alice@example.com"));
        assert_eq!(login.password.as_deref(), Some("fake-password-123"));
    }
}
