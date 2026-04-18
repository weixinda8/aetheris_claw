use crate::agent::config::config::{AgentConfig, AgentConfigError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StorageBackendType {
    Local,
    Etcd,
    Consul,
}

#[derive(Debug, Clone)]
pub struct ConfigStorageConfig {
    pub backend_type: StorageBackendType,
    pub local_path: Option<PathBuf>,
    pub etcd_endpoints: Option<Vec<String>>,
    pub etcd_prefix: Option<String>,
    pub consul_endpoint: Option<String>,
    pub consul_prefix: Option<String>,
}

#[async_trait]
pub trait ConfigStorage: Send + Sync {
    async fn get(&self, agent_id: &str) -> Result<Option<AgentConfig>, AgentConfigError>;
    async fn put(&self, agent_id: &str, config: &AgentConfig) -> Result<(), AgentConfigError>;
    async fn delete(&self, agent_id: &str) -> Result<bool, AgentConfigError>;
    async fn list(&self) -> Result<Vec<String>, AgentConfigError>;
    async fn list_all(&self) -> Result<Vec<AgentConfig>, AgentConfigError>;
    async fn exists(&self, agent_id: &str) -> Result<bool, AgentConfigError>;
}

pub struct LocalStorage {
    storage_path: PathBuf,
}

impl LocalStorage {
    pub fn new(storage_path: PathBuf) -> Result<Self, AgentConfigError> {
        if !storage_path.exists() {
            std::fs::create_dir_all(&storage_path)?;
        }
        Ok(Self { storage_path })
    }

    fn config_path(&self, agent_id: &str) -> PathBuf {
        self.storage_path.join(format!("{}.yaml", agent_id))
    }
}

#[async_trait]
impl ConfigStorage for LocalStorage {
    async fn get(&self, agent_id: &str) -> Result<Option<AgentConfig>, AgentConfigError> {
        let path = self.config_path(agent_id);
        if !path.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(path).await?;
        let config = serde_yaml::from_str(&content)?;
        Ok(Some(config))
    }

    async fn put(&self, agent_id: &str, config: &AgentConfig) -> Result<(), AgentConfigError> {
        let path = self.config_path(agent_id);
        let content = serde_yaml::to_string(config)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    async fn delete(&self, agent_id: &str) -> Result<bool, AgentConfigError> {
        let path = self.config_path(agent_id);
        if !path.exists() {
            return Ok(false);
        }
        tokio::fs::remove_file(path).await?;
        Ok(true)
    }

    async fn list(&self) -> Result<Vec<String>, AgentConfigError> {
        let mut agent_ids = Vec::new();

        if !self.storage_path.exists() {
            return Ok(agent_ids);
        }

        let mut entries = tokio::fs::read_dir(&self.storage_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "yaml" || ext == "yml" {
                        if let Some(stem) = path.file_stem() {
                            if let Some(id) = stem.to_str() {
                                agent_ids.push(id.to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(agent_ids)
    }

    async fn list_all(&self) -> Result<Vec<AgentConfig>, AgentConfigError> {
        let agent_ids = self.list().await?;
        let mut configs = Vec::new();

        for agent_id in agent_ids {
            if let Some(config) = self.get(&agent_id).await? {
                configs.push(config);
            }
        }

        Ok(configs)
    }

    async fn exists(&self, agent_id: &str) -> Result<bool, AgentConfigError> {
        let path = self.config_path(agent_id);
        Ok(path.exists())
    }
}

pub struct InMemoryStorage {
    configs: RwLock<HashMap<String, AgentConfig>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            configs: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConfigStorage for InMemoryStorage {
    async fn get(&self, agent_id: &str) -> Result<Option<AgentConfig>, AgentConfigError> {
        let configs = self.configs.read().await;
        Ok(configs.get(agent_id).cloned())
    }

    async fn put(&self, agent_id: &str, config: &AgentConfig) -> Result<(), AgentConfigError> {
        let mut configs = self.configs.write().await;
        configs.insert(agent_id.to_string(), config.clone());
        Ok(())
    }

    async fn delete(&self, agent_id: &str) -> Result<bool, AgentConfigError> {
        let mut configs = self.configs.write().await;
        Ok(configs.remove(agent_id).is_some())
    }

    async fn list(&self) -> Result<Vec<String>, AgentConfigError> {
        let configs = self.configs.read().await;
        Ok(configs.keys().cloned().collect())
    }

    async fn list_all(&self) -> Result<Vec<AgentConfig>, AgentConfigError> {
        let configs = self.configs.read().await;
        Ok(configs.values().cloned().collect())
    }

    async fn exists(&self, agent_id: &str) -> Result<bool, AgentConfigError> {
        let configs = self.configs.read().await;
        Ok(configs.contains_key(agent_id))
    }
}

pub struct StorageManager {
    storage: Arc<dyn ConfigStorage>,
}

impl StorageManager {
    pub fn new(storage: Arc<dyn ConfigStorage>) -> Self {
        Self { storage }
    }

    pub fn with_local(storage_path: PathBuf) -> Result<Self, AgentConfigError> {
        let storage = LocalStorage::new(storage_path)?;
        Ok(Self::new(Arc::new(storage)))
    }

    pub fn with_in_memory() -> Self {
        Self::new(Arc::new(InMemoryStorage::new()))
    }

    pub async fn get_config(
        &self,
        agent_id: &str,
    ) -> Result<Option<AgentConfig>, AgentConfigError> {
        self.storage.get(agent_id).await
    }

    pub async fn save_config(
        &self,
        agent_id: &str,
        config: &AgentConfig,
    ) -> Result<(), AgentConfigError> {
        self.storage.put(agent_id, config).await
    }

    pub async fn delete_config(&self, agent_id: &str) -> Result<bool, AgentConfigError> {
        self.storage.delete(agent_id).await
    }

    pub async fn list_agents(&self) -> Result<Vec<String>, AgentConfigError> {
        self.storage.list().await
    }

    pub async fn list_all_configs(&self) -> Result<Vec<AgentConfig>, AgentConfigError> {
        self.storage.list_all().await
    }

    pub async fn config_exists(&self, agent_id: &str) -> Result<bool, AgentConfigError> {
        self.storage.exists(agent_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_in_memory_storage_basic() {
        let storage = InMemoryStorage::new();

        let mut config = AgentConfig::default();
        config.meta.id = "test-agent".to_string();

        storage.put("test-agent", &config).await.unwrap();

        let loaded = storage.get("test-agent").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().meta.id, "test-agent");

        let exists = storage.exists("test-agent").await.unwrap();
        assert!(exists);

        let deleted = storage.delete("test-agent").await.unwrap();
        assert!(deleted);
    }

    #[tokio::test]
    async fn test_in_memory_storage_multiple() {
        let storage = InMemoryStorage::new();

        let mut config1 = AgentConfig::default();
        config1.meta.id = "agent-1".to_string();

        let mut config2 = AgentConfig::default();
        config2.meta.id = "agent-2".to_string();

        storage.put("agent-1", &config1).await.unwrap();
        storage.put("agent-2", &config2).await.unwrap();

        let list = storage.list().await.unwrap();
        assert_eq!(list.len(), 2);

        let all_configs = storage.list_all().await.unwrap();
        assert_eq!(all_configs.len(), 2);
    }

    #[tokio::test]
    async fn test_local_storage_basic() {
        let temp_dir = tempdir().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let mut config = AgentConfig::default();
        config.meta.id = "test-local-agent".to_string();

        storage.put("test-local-agent", &config).await.unwrap();

        let loaded = storage.get("test-local-agent").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().meta.id, "test-local-agent");

        let exists = storage.exists("test-local-agent").await.unwrap();
        assert!(exists);

        let deleted = storage.delete("test-local-agent").await.unwrap();
        assert!(deleted);
    }

    #[tokio::test]
    async fn test_storage_manager_in_memory() {
        let manager = StorageManager::with_in_memory();

        let mut config = AgentConfig::default();
        config.meta.id = "manager-test".to_string();

        manager.save_config("manager-test", &config).await.unwrap();

        let loaded = manager.get_config("manager-test").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().meta.id, "manager-test");

        let exists = manager.config_exists("manager-test").await.unwrap();
        assert!(exists);

        let list = manager.list_agents().await.unwrap();
        assert_eq!(list.len(), 1);

        let all_configs = manager.list_all_configs().await.unwrap();
        assert_eq!(all_configs.len(), 1);

        let deleted = manager.delete_config("manager-test").await.unwrap();
        assert!(deleted);
    }

    #[tokio::test]
    async fn test_storage_manager_local() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::with_local(temp_dir.path().to_path_buf()).unwrap();

        let mut config = AgentConfig::default();
        config.meta.id = "local-manager-test".to_string();

        manager
            .save_config("local-manager-test", &config)
            .await
            .unwrap();

        let loaded = manager.get_config("local-manager-test").await.unwrap();
        assert!(loaded.is_some());

        let deleted = manager.delete_config("local-manager-test").await.unwrap();
        assert!(deleted);
    }

    #[tokio::test]
    async fn test_storage_backend_type() {
        assert_eq!(StorageBackendType::Local, StorageBackendType::Local);
        assert_eq!(StorageBackendType::Etcd, StorageBackendType::Etcd);
        assert_eq!(StorageBackendType::Consul, StorageBackendType::Consul);
    }

    #[tokio::test]
    async fn test_storage_get_nonexistent() {
        let storage = InMemoryStorage::new();
        let loaded = storage.get("nonexistent").await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_storage_delete_nonexistent() {
        let storage = InMemoryStorage::new();
        let deleted = storage.delete("nonexistent").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_storage_exists_nonexistent() {
        let storage = InMemoryStorage::new();
        let exists = storage.exists("nonexistent").await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_in_memory_storage_default() {
        let storage = InMemoryStorage::default();
        assert!(storage.list().await.unwrap().is_empty());
    }
}
