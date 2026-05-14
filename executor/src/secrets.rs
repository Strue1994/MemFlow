use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::env;
use once_cell::sync::Lazy;

pub struct SecretsManager {
    cache: Arc<RwLock<HashMap<String, String>>>,
}

impl SecretsManager {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get(&self, key: &str) -> Result<String, String> {
        let cache = self.cache.read().await;
        
        if let Some(value) = cache.get(key) {
            return Ok(value.clone());
        }
        drop(cache);

        let env_key = format!("MEMFLOW_{}", key.to_uppercase());
        let value = env::var(&env_key)
            .map_err(|_| format!("Secret {} not found in environment", key))?;

        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), value.clone());
        
        Ok(value)
    }

    pub async fn get_optional(&self, key: &str) -> Option<String> {
        self.get(key).await.ok()
    }
}

impl Default for SecretsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "vault")]
pub mod vault {
    use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
    use vaultrs::kv2;

    pub struct VaultSecrets {
        client: VaultClient,
        mount: String,
    }

    impl VaultSecrets {
        pub fn new(addr: &str, token: &str, mount: &str) -> Result<Self, vaultrs::error::ClientError> {
            let settings = VaultClientSettingsBuilder::new()
                .address(addr)
                .token(token)
                .build();
            let client = VaultClient::new(settings)?;
            Ok(Self {
                client,
                mount: mount.to_string(),
            })
        }

        pub fn get(&self, path: &str, key: &str) -> Result<String, vaultrs::error::ClientError> {
            let secret = kv2::read(&self.client, &self.mount, path)?;
            Ok(secret.data.get(key).cloned().unwrap_or_default())
        }
    }
}

pub static SECRET_MANAGER: Lazy<SecretsManager> = Lazy::new(SecretsManager::new);

#[allow(dead_code)]
pub fn get_secret(key: &str) -> Result<String, String> {
    tokio::runtime::Handle::current()
        .block_on(async { SECRET_MANAGER.get(key).await })
}