//! Cross-cutting domain types referenced by three or more layers.
//!
//! View-local types live in the owning view's `state.rs` (or `mod.rs` while
//! the archetype split is pending). Keep this file small.

use bitwarden_core::OrganizationId;

pub use bitwarden_core::UserId;

use crate::services::sdk::Organization;

/// Source / destination vault picker shared by import and export modals:
/// the user's personal vault, or one of their organizations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultChoice {
    Personal,
    Org { id: OrganizationId, name: String },
}

impl VaultChoice {
    /// Caller supplies the localized "personal" label so each modal can use
    /// its own fluent key without this type owning either.
    pub fn label(&self, personal_label: &str) -> String {
        match self {
            Self::Personal => personal_label.to_string(),
            Self::Org { name, .. } => name.clone(),
        }
    }

    pub fn list_with_personal(orgs: &[Organization]) -> Vec<Self> {
        let mut choices = Vec::with_capacity(orgs.len() + 1);
        choices.push(Self::Personal);
        for org in orgs {
            choices.push(Self::Org {
                id: org.id,
                name: org.name.clone(),
            });
        }
        choices
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    /// Initial screen while `ClientManager::load` runs on a background thread.
    /// Switches to `Login` once the loaded manager arrives via
    /// `SystemMessage::ClientManagerLoaded`.
    Loading,
    Login,
    Vault,
    Send,
}

/// The unlock method the login view is currently presenting.
///
/// Shared between the SDK (`ClientManager::unlock_methods`) and the login
/// view because the SDK reports which methods are available and the view
/// picks one; keeping the protocol type here avoids a
/// `services → views/login` import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockMethod {
    Biometrics,
    Pin,
    MasterPassword,
}

/// The set of unlock methods a user has configured. Returned by the SDK;
/// consumed by the login view to drive the unlock UI.
#[derive(Debug, Clone)]
pub struct UnlockMethods {
    pub master_password: bool,
    pub pin: bool,
    pub biometrics: bool,
}

impl UnlockMethods {
    pub fn preferred(&self) -> UnlockMethod {
        if self.biometrics {
            UnlockMethod::Biometrics
        } else if self.pin {
            UnlockMethod::Pin
        } else {
            UnlockMethod::MasterPassword
        }
    }

    pub fn alternatives(&self, current: UnlockMethod) -> Vec<UnlockMethod> {
        let mut alts = Vec::new();
        if self.biometrics && current != UnlockMethod::Biometrics {
            alts.push(UnlockMethod::Biometrics);
        }
        if self.pin && current != UnlockMethod::Pin {
            alts.push(UnlockMethod::Pin);
        }
        if self.master_password && current != UnlockMethod::MasterPassword {
            alts.push(UnlockMethod::MasterPassword);
        }
        alts
    }
}
