//! Per-user SDK state held by [`super::ClientManager`]. Fully private — the
//! only handle that leaves this module is `PasswordManagerClient` (cheap-to-
//! clone, internally `Arc`'d) or the small bundles assembled by accessors
//! on `ClientManager`.

use bitwarden_crypto::{EncString, Kdf};
use bitwarden_pm::PasswordManagerClient;

use crate::domain::UnlockMethods;

use super::types::{Collection, Organization};

pub(super) struct UserEntry {
    pub(super) client: PasswordManagerClient,
    // App-side stand-in for SDK profile methods that don't exist yet.
    pub(super) email: String,
    pub(super) display_name: String,
    pub(super) server_url: String,
    pub(super) unlock_methods: UnlockMethods,
    pub(super) kdf: Kdf,
    pub(super) encrypted_user_key: EncString,
    pub(super) private_key: EncString,
    /// Seeded from `mock.json` (no sync path in this stub).
    pub(super) organizations: Vec<Organization>,
    pub(super) collections: Vec<Collection>,
}
