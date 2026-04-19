//! Generate real FIDO2/WebAuthn credentials for fake vault data.
//!
//! `Fido2Client::register` is the end-to-end WebAuthn flow:
//!   1. The caller hands us a `CredentialCreationOptions` JSON + origin.
//!   2. The authenticator calls `Fido2UserInterface::check_user_and_pick_credential_for_creation`
//!      to get a `CipherView` to attach the new credential to.
//!   3. The authenticator generates a P-256 keypair, wraps it into a
//!      `Fido2CredentialFullView`, pushes it onto the view, encrypts the
//!      whole cipher via the user's key store, and hands the encrypted
//!      `Cipher` to `Fido2CredentialStore::save_credential`.
//!
//! This module wires up the two trait impls with no-op UX (auto-approve
//! verification, pick the target cipher the caller told us about) and a
//! `register_passkey` helper that drives one registration and returns the
//! encrypted `Cipher` — ready to persist directly, skipping the normal
//! `client.vault().ciphers().encrypt(view)` path.

use std::sync::Mutex;

use bitwarden_core::Client;
use bitwarden_fido::{
    CheckUserOptions, CheckUserResult, ClientData, Fido2Authenticator, Fido2CallbackError,
    Fido2Client, Fido2CredentialStore, Fido2UserInterface, Origin, UiHint,
};
use bitwarden_vault::{
    Cipher, CipherListView, CipherView, EncryptionContext, Fido2CredentialNewView,
};
use rand::RngCore;
use serde_json::json;

/// What we want to register on a given login. All strings are the plaintext
/// values that would come from a real relying party.
#[derive(Debug, Clone)]
pub struct PasskeySpec {
    /// WebAuthn `rp.id` — the registrable domain (`"github.com"`).
    pub rp_id: String,
    /// WebAuthn `rp.name` — human-readable site name (`"GitHub"`).
    pub rp_name: String,
    /// WebAuthn `user.name` — the account identifier on the RP.
    pub user_name: String,
    /// WebAuthn `user.displayName`.
    pub user_display_name: String,
}

/// Auto-approving user interface. Every registration treats the user as
/// present + verified and returns the pre-stashed target cipher; there's
/// no actual user for fake data to interact with.
struct AutoApproveUi {
    target: Mutex<Option<CipherView>>,
}

#[async_trait::async_trait]
impl Fido2UserInterface for AutoApproveUi {
    async fn check_user<'a>(
        &self,
        _options: CheckUserOptions,
        _hint: UiHint<'a, CipherView>,
    ) -> Result<CheckUserResult, Fido2CallbackError> {
        Ok(CheckUserResult {
            user_present: true,
            user_verified: true,
        })
    }

    async fn pick_credential_for_authentication(
        &self,
        _available_credentials: Vec<CipherView>,
    ) -> Result<CipherView, Fido2CallbackError> {
        // Only used during authenticate(); register() doesn't hit this.
        Err(Fido2CallbackError::Unknown(
            "pick_credential_for_authentication unreachable in fake-data".into(),
        ))
    }

    async fn check_user_and_pick_credential_for_creation(
        &self,
        _options: CheckUserOptions,
        _new_credential: Fido2CredentialNewView,
    ) -> Result<(CipherView, CheckUserResult), Fido2CallbackError> {
        let cipher = self
            .target
            .lock()
            .expect("target mutex is not poisoned")
            .take()
            .ok_or_else(|| {
                Fido2CallbackError::Unknown(
                    "AutoApproveUi target not set before Fido2Client::register".into(),
                )
            })?;
        Ok((
            cipher,
            CheckUserResult {
                user_present: true,
                user_verified: true,
            },
        ))
    }

    fn is_verification_enabled(&self) -> bool {
        true
    }
}

/// Capturing credential store. `save_credential` is the only method hit
/// during register; the other two only matter for authenticate or
/// exclude-list handling, which a fresh target cipher won't trigger.
struct CapturingStore {
    saved: Mutex<Option<Cipher>>,
}

#[async_trait::async_trait]
impl Fido2CredentialStore for CapturingStore {
    async fn find_credentials(
        &self,
        _ids: Option<Vec<Vec<u8>>>,
        _rp_id: String,
        _user_handle: Option<Vec<u8>>,
    ) -> Result<Vec<CipherView>, Fido2CallbackError> {
        Ok(Vec::new())
    }

    async fn all_credentials(&self) -> Result<Vec<CipherListView>, Fido2CallbackError> {
        Ok(Vec::new())
    }

    async fn save_credential(&self, cred: EncryptionContext) -> Result<(), Fido2CallbackError> {
        *self.saved.lock().expect("saved mutex is not poisoned") = Some(cred.cipher);
        Ok(())
    }
}

type DynError = Box<dyn std::error::Error + Send + Sync>;

/// Run one WebAuthn registration against the given `Client`/`CipherView`
/// pair and return the encrypted `Cipher`, with the new passkey embedded
/// in `login.fido2_credentials`. The `CipherView` is consumed — registration
/// attaches the credential to it, so callers should drop their own copy.
pub async fn register_passkey(
    client: &Client,
    target: CipherView,
    spec: &PasskeySpec,
) -> Result<Cipher, DynError> {
    let ui = AutoApproveUi {
        target: Mutex::new(Some(target)),
    };
    let store = CapturingStore {
        saved: Mutex::new(None),
    };

    let mut rng = rand::rng();
    let mut challenge = [0u8; 32];
    rng.fill_bytes(&mut challenge);
    let mut user_handle = [0u8; 32];
    rng.fill_bytes(&mut user_handle);

    let request = json!({
        "publicKey": {
            "rp": { "id": spec.rp_id, "name": spec.rp_name },
            "user": {
                "id": base64url(&user_handle),
                "name": spec.user_name,
                "displayName": spec.user_display_name,
            },
            "challenge": base64url(&challenge),
            // ES256 (-7). Matches the algorithm `fill_with_credential` hardcodes
            // for Bitwarden-stored passkeys.
            "pubKeyCredParams": [ { "type": "public-key", "alg": -7 } ],
            "authenticatorSelection": {
                "requireResidentKey": true,
                "residentKey": "required",
                "userVerification": "preferred",
            },
            "attestation": "none",
        }
    })
    .to_string();

    let authenticator = Fido2Authenticator::new(client, &ui, &store);
    let mut fido_client = Fido2Client { authenticator };
    fido_client
        .register(
            Origin::Web(format!("https://{}", spec.rp_id)),
            request,
            ClientData::DefaultWithCustomHash {
                // 32-byte custom hash — the passkey crate just signs
                // whatever we give it. Fake data doesn't need a real
                // collected-client-data JSON.
                hash: vec![0u8; 32],
            },
        )
        .await
        .map_err(|e| format!("Fido2Client::register failed: {e:?}"))?;

    let cipher = store
        .saved
        .into_inner()
        .expect("saved mutex is not poisoned")
        .ok_or("Fido2CredentialStore::save_credential was never invoked")?;
    Ok(cipher)
}

fn base64url(bytes: &[u8]) -> String {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.encode(bytes)
}
