use std::collections::HashMap;

pub type UserId = String;

#[derive(Debug, Clone)]
pub struct AppState {
    pub users: HashMap<UserId, UserSession>,
    pub active_user: Option<UserId>,
    pub screen: Screen,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields mirror the SDK's PasswordManagerClient; not all used in stub UI yet
pub struct UserSession {
    pub email: String,
    pub display_name: String,
    pub server_url: String,
    pub locked: bool,
    pub vault_items: Vec<CipherItem>,
}

#[derive(Debug, Clone)]
pub struct CipherItem {
    pub id: String,
    pub name: String,
    pub username: Option<String>,
    pub url: Option<String>,
    pub category: CipherCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CipherCategory {
    Login,
    Card,
    Identity,
    SecureNote,
    SshKey,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Login,
    Vault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarFilter {
    AllItems,
    Favorites,
    Category(CipherCategory),
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
