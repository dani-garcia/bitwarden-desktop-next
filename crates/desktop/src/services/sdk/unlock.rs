//! Unlock entry point + sync stub.
//!
//! `unlock` lives on [`UnlockData`] (not on `ClientExt`) because it needs the
//! kdf params and key envelopes alongside the client. `sync` is a stub.

use std::collections::HashMap;

use bitwarden_core::{
    OrganizationId, UserId,
    key_management::{
        MasterPasswordUnlockData,
        account_cryptographic_state::WrappedAccountCryptographicState,
        crypto::{InitOrgCryptoRequest, InitUserCryptoMethod, InitUserCryptoRequest},
    },
};
use bitwarden_crypto::{EncString, Kdf, UnsignedSharedKey};
use bitwarden_pm::PasswordManagerClient;

use super::types::Organization;

/// Bundle of fields `UnlockData::unlock` needs. Pulled sync from the manager
/// via [`super::ClientManager::unlock_data_for`] so the async task is fully
/// owned.
pub struct UnlockData {
    pub client: PasswordManagerClient,
    pub sdk_user_id: UserId,
    pub kdf: Kdf,
    pub email: String,
    pub encrypted_user_key: EncString,
    pub private_key: EncString,
    pub organizations: Vec<Organization>,
}

impl UnlockData {
    /// Initialize the SDK crypto state with the user's master password. On
    /// success the keystore is unlocked in memory.
    pub async fn unlock(self, password: String) -> Result<(), String> {
        let req = InitUserCryptoRequest {
            // Reuse the Client's bound UserId on every unlock — see the
            // invariant on `UserEntry::sdk_user_id`.
            user_id: Some(self.sdk_user_id),
            kdf_params: self.kdf.clone(),
            email: self.email.clone(),
            account_cryptographic_state: WrappedAccountCryptographicState::V1 {
                private_key: self.private_key,
            },
            method: InitUserCryptoMethod::MasterPasswordUnlock {
                password,
                master_password_unlock: MasterPasswordUnlockData {
                    kdf: self.kdf,
                    master_key_wrapped_user_key: self.encrypted_user_key,
                    salt: self.email,
                },
            },
            upgrade_token: None,
        };

        self.client
            .crypto()
            .initialize_user_crypto(req)
            .await
            .map_err(|e| e.to_string())?;

        // Replay org keys so the vault client can encrypt/decrypt org-owned
        // ciphers. Orgs without a wrapped key contribute nothing.
        let org_keys: HashMap<OrganizationId, UnsignedSharedKey> = self
            .organizations
            .into_iter()
            .filter_map(|o| o.wrapped_key.map(|k| (o.id, k)))
            .collect();
        if !org_keys.is_empty() {
            self.client
                .crypto()
                .initialize_org_crypto(InitOrgCryptoRequest {
                    organization_keys: org_keys,
                })
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}

/// Stub for File → Sync now. The fake-data harness has no remote to sync
/// against, so this currently no-ops. Kept async + fallible so the call
/// site doesn't need to change when a real sync flow lands.
pub async fn sync() -> Result<(), String> {
    Ok(())
}
