use bitwarden_vault::CipherType;

pub type UserId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockMethod {
    Biometrics,
    Pin,
    MasterPassword,
}

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

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    // Initial screen while `ClientManager::load` runs on a background thread.
    // Switches to `Login` once the loaded manager arrives via
    // `SystemMessage::ClientManagerLoaded`.
    Loading,
    Login,
    Vault,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarFilter {
    AllItems,
    Favorites,
    Category(CipherType),
    Archive,
    Trash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarMode {
    Collapsed,
    Expanded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavSection {
    Vault,
    Send,
    Generator,
    Import,
    Export,
}
