use std::{collections::HashMap, sync::Arc};

use bitwarden_pm::PasswordManagerClient;
use bitwarden_state::repository::{Repository, RepositoryError, RepositoryItem};
use bitwarden_vault::{CipherId, FolderId};

use crate::state::UserId;

/// Manages one `PasswordManagerClient` per logged-in user.
pub struct ClientManager {
    clients: HashMap<UserId, PasswordManagerClient>,
}

#[expect(dead_code)]
impl ClientManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    /// Create a manager pre-populated with two fake users for development.
    pub fn mock() -> Self {
        let mut clients = HashMap::new();
        clients.insert("user-1".to_string(), mock_personal_client());
        clients.insert("user-2".to_string(), mock_work_client());
        Self { clients }
    }

    pub fn get(&self, user_id: &str) -> Option<&PasswordManagerClient> {
        self.clients.get(user_id)
    }

    pub fn list(&self) -> impl Iterator<Item = (&UserId, &PasswordManagerClient)> {
        self.clients.iter()
    }
}

fn settings_for_user() -> bitwarden_core::ClientSettings {
    bitwarden_core::ClientSettings {
        identity_url: "http://localhost:8080/identity".to_string(),
        api_url: "http://localhost:8080/api".to_string(),
        ..Default::default()
    }
}

fn mock_personal_client() -> PasswordManagerClient {
    mock_client([], [])
}

fn mock_work_client() -> PasswordManagerClient {
    mock_client([], [])
}

fn mock_client(
    folder_data: impl Into<HashMap<String, bitwarden_vault::Folder>>,
    cipher_data: impl Into<HashMap<String, bitwarden_vault::Cipher>>,
) -> PasswordManagerClient {
    let client = PasswordManagerClient::new(Some(settings_for_user()));

    let folder_repo = MemoryRepo::<bitwarden_vault::Folder>::new(folder_data);
    let cipher_repo = MemoryRepo::<bitwarden_vault::Cipher>::new(cipher_data);

    client
        .platform()
        .state()
        .register_client_managed(folder_repo);
    client
        .platform()
        .state()
        .register_client_managed(cipher_repo);

    client
}

struct MemoryRepo<T: RepositoryItem + Clone> {
    data: std::sync::Mutex<HashMap<String, T>>,
}

impl<T: RepositoryItem + Clone> MemoryRepo<T> {
    fn new(data: impl Into<HashMap<String, T>>) -> Arc<Self> {
        Arc::new(Self {
            data: std::sync::Mutex::new(data.into()),
        })
    }
}

#[async_trait::async_trait]
impl<T: RepositoryItem + Clone> Repository<T> for MemoryRepo<T> {
    async fn get(&self, key: T::Key) -> Result<Option<T>, RepositoryError> {
        let map = self.data.lock().unwrap();
        Ok(map.get(&key.to_string()).cloned())
    }
    async fn list(&self) -> Result<Vec<T>, RepositoryError> {
        let map = self.data.lock().unwrap();
        Ok(map.values().cloned().collect())
    }
    async fn set(&self, key: T::Key, value: T) -> Result<(), RepositoryError> {
        let mut map = self.data.lock().unwrap();
        map.insert(key.to_string(), value);
        Ok(())
    }
    async fn set_bulk(&self, values: Vec<(T::Key, T)>) -> Result<(), RepositoryError> {
        let mut map = self.data.lock().unwrap();
        for (key, value) in values {
            map.insert(key.to_string(), value);
        }
        Ok(())
    }
    async fn remove(&self, key: T::Key) -> Result<(), RepositoryError> {
        let mut map = self.data.lock().unwrap();
        map.remove(&key.to_string());
        Ok(())
    }
    async fn remove_bulk(&self, keys: Vec<T::Key>) -> Result<(), RepositoryError> {
        let mut map = self.data.lock().unwrap();
        for key in keys {
            map.remove(&key.to_string());
        }
        Ok(())
    }
    async fn remove_all(&self) -> Result<(), RepositoryError> {
        let mut map = self.data.lock().unwrap();
        map.clear();
        Ok(())
    }
}
