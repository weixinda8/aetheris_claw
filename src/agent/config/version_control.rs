use crate::agent::config::config::{AgentConfig, AgentConfigError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentVersion {
    pub version: String,
    pub config: AgentConfig,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistory {
    pub agent_id: String,
    pub current_version: String,
    pub versions: Vec<AgentVersion>,
    pub max_versions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    pub success: bool,
    pub from_version: String,
    pub to_version: String,
    pub config: Option<AgentConfig>,
    pub message: String,
}

pub struct VersionManager {
    histories: RwLock<HashMap<String, VersionHistory>>,
    storage_dir: Option<PathBuf>,
    max_versions_per_agent: usize,
}

impl VersionManager {
    pub fn new() -> Self {
        Self {
            histories: RwLock::new(HashMap::new()),
            storage_dir: None,
            max_versions_per_agent: 10,
        }
    }

    pub fn with_storage_dir<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.storage_dir = Some(dir.into());
        self
    }

    pub fn with_max_versions(mut self, max: usize) -> Self {
        self.max_versions_per_agent = max;
        self
    }

    pub async fn create_version(
        &self,
        agent_id: &str,
        config: AgentConfig,
        description: Option<String>,
        author: Option<String>,
    ) -> Result<AgentVersion, AgentConfigError> {
        let checksum = compute_config_checksum(&config);
        let version = config.meta.version.clone();

        let agent_version = AgentVersion {
            version: version.clone(),
            config,
            created_at: chrono::Utc::now(),
            description,
            author,
            checksum,
        };

        let mut histories = self.histories.write().await;

        let history = histories
            .entry(agent_id.to_string())
            .or_insert_with(|| VersionHistory {
                agent_id: agent_id.to_string(),
                current_version: version.clone(),
                versions: Vec::new(),
                max_versions: self.max_versions_per_agent,
            });

        history.current_version = version.clone();
        history.versions.push(agent_version.clone());

        if history.versions.len() > history.max_versions {
            history.versions.remove(0);
        }

        if let Some(storage_dir) = &self.storage_dir {
            let _ = self
                .save_history_to_disk(agent_id, history, storage_dir)
                .await;
        }

        Ok(agent_version)
    }

    pub async fn get_current_version(&self, agent_id: &str) -> Option<AgentVersion> {
        let histories = self.histories.read().await;
        histories
            .get(agent_id)
            .and_then(|h| h.versions.last().cloned())
    }

    pub async fn get_version(&self, agent_id: &str, version: &str) -> Option<AgentVersion> {
        let histories = self.histories.read().await;
        histories
            .get(agent_id)
            .and_then(|h| h.versions.iter().find(|v| v.version == version).cloned())
    }

    pub async fn list_versions(&self, agent_id: &str) -> Vec<AgentVersion> {
        let histories = self.histories.read().await;
        histories
            .get(agent_id)
            .map(|h| h.versions.clone())
            .unwrap_or_default()
    }

    pub async fn rollback_to_version(
        &self,
        agent_id: &str,
        target_version: &str,
    ) -> Result<RollbackResult, AgentConfigError> {
        let mut histories = self.histories.write().await;

        let Some(history) = histories.get_mut(agent_id) else {
            return Ok(RollbackResult {
                success: false,
                from_version: "unknown".to_string(),
                to_version: target_version.to_string(),
                config: None,
                message: format!("No history found for agent: {}", agent_id),
            });
        };

        let from_version = history.current_version.clone();

        let Some(target) = history
            .versions
            .iter()
            .find(|v| v.version == target_version)
        else {
            return Ok(RollbackResult {
                success: false,
                from_version,
                to_version: target_version.to_string(),
                config: None,
                message: format!("Version not found: {}", target_version),
            });
        };

        history.current_version = target_version.to_string();

        let result = RollbackResult {
            success: true,
            from_version,
            to_version: target_version.to_string(),
            config: Some(target.config.clone()),
            message: "Rollback successful".to_string(),
        };

        if let Some(storage_dir) = &self.storage_dir {
            let _ = self
                .save_history_to_disk(agent_id, history, storage_dir)
                .await;
        }

        Ok(result)
    }

    pub async fn rollback_to_previous(
        &self,
        agent_id: &str,
    ) -> Result<RollbackResult, AgentConfigError> {
        let target_version = {
            let histories = self.histories.read().await;

            let Some(history) = histories.get(agent_id) else {
                return Ok(RollbackResult {
                    success: false,
                    from_version: "unknown".to_string(),
                    to_version: "unknown".to_string(),
                    config: None,
                    message: format!("No history found for agent: {}", agent_id),
                });
            };

            if history.versions.len() < 2 {
                return Ok(RollbackResult {
                    success: false,
                    from_version: history.current_version.clone(),
                    to_version: "unknown".to_string(),
                    config: None,
                    message: "No previous version available".to_string(),
                });
            }

            let previous_idx = history.versions.len() - 2;
            history.versions[previous_idx].version.clone()
        };

        self.rollback_to_version(agent_id, &target_version).await
    }

    pub async fn delete_version(
        &self,
        agent_id: &str,
        version: &str,
    ) -> Result<bool, AgentConfigError> {
        let mut histories = self.histories.write().await;

        let Some(history) = histories.get_mut(agent_id) else {
            return Ok(false);
        };

        let original_len = history.versions.len();
        history.versions.retain(|v| v.version != version);

        let deleted = history.versions.len() < original_len;

        if deleted && history.current_version == version {
            if let Some(latest) = history.versions.last() {
                history.current_version = latest.version.clone();
            }
        }

        if let Some(storage_dir) = &self.storage_dir {
            let _ = self
                .save_history_to_disk(agent_id, history, storage_dir)
                .await;
        }

        Ok(deleted)
    }

    pub async fn clear_history(&self, agent_id: &str) {
        let mut histories = self.histories.write().await;
        histories.remove(agent_id);

        if let Some(storage_dir) = &self.storage_dir {
            let history_file = storage_dir.join(format!("{}-history.json", agent_id));
            let _ = std::fs::remove_file(history_file);
        }
    }

    pub async fn load_history_from_disk(&self, agent_id: &str) -> Result<(), AgentConfigError> {
        let Some(storage_dir) = &self.storage_dir else {
            return Ok(());
        };

        let history_file = storage_dir.join(format!("{}-history.json", agent_id));
        if !history_file.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&history_file)?;
        let history: VersionHistory = serde_json::from_str(&content)?;

        let mut histories = self.histories.write().await;
        histories.insert(agent_id.to_string(), history);

        Ok(())
    }

    async fn save_history_to_disk(
        &self,
        agent_id: &str,
        history: &VersionHistory,
        storage_dir: &PathBuf,
    ) -> Result<(), AgentConfigError> {
        let _ = std::fs::create_dir_all(storage_dir);
        let history_file = storage_dir.join(format!("{}-history.json", agent_id));
        let content = serde_json::to_string_pretty(history)?;
        std::fs::write(history_file, content)?;
        Ok(())
    }

    pub async fn load_all_histories(&self) -> Result<(), AgentConfigError> {
        let Some(storage_dir) = &self.storage_dir else {
            return Ok(());
        };

        if !storage_dir.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(storage_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                    if filename.ends_with("-history.json") {
                        if let Some(agent_id) = filename.strip_suffix("-history.json") {
                            let _ = self.load_history_from_disk(agent_id).await;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for VersionManager {
    fn default() -> Self {
        Self::new()
    }
}

fn compute_config_checksum(config: &AgentConfig) -> String {
    use sha2::{Digest, Sha256};
    let content = serde_json::to_string(config).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_version_manager_creation() {
        let manager = VersionManager::new();
        let versions = manager.list_versions("test-agent").await;
        assert!(versions.is_empty());
    }

    #[tokio::test]
    async fn test_create_version() {
        let manager = VersionManager::new();
        let config = crate::agent::config::config::AgentConfig::default();

        let version = manager
            .create_version("test-agent", config.clone(), None, None)
            .await
            .unwrap();
        assert_eq!(version.version, config.meta.version);

        let versions = manager.list_versions("test-agent").await;
        assert_eq!(versions.len(), 1);

        let current = manager.get_current_version("test-agent").await;
        assert!(current.is_some());
    }

    #[tokio::test]
    async fn test_get_specific_version() {
        let manager = VersionManager::new();

        let mut config = crate::agent::config::config::AgentConfig::default();
        config.meta.version = "1.0.0".to_string();
        manager
            .create_version("test-agent", config, None, None)
            .await
            .unwrap();

        let mut config_v2 = crate::agent::config::config::AgentConfig::default();
        config_v2.meta.version = "1.1.0".to_string();
        manager
            .create_version("test-agent", config_v2, None, None)
            .await
            .unwrap();

        let version = manager.get_version("test-agent", "1.0.0").await;
        assert!(version.is_some());
        assert_eq!(version.unwrap().version, "1.0.0");
    }

    #[tokio::test]
    async fn test_rollback_to_version() {
        let manager = VersionManager::new();

        let mut config = crate::agent::config::config::AgentConfig::default();
        config.meta.version = "1.0.0".to_string();
        config.meta.name = "Version 1".to_string();
        manager
            .create_version("test-agent", config, None, None)
            .await
            .unwrap();

        let mut config_v2 = crate::agent::config::config::AgentConfig::default();
        config_v2.meta.version = "1.1.0".to_string();
        config_v2.meta.name = "Version 2".to_string();
        manager
            .create_version("test-agent", config_v2, None, None)
            .await
            .unwrap();

        let result = manager
            .rollback_to_version("test-agent", "1.0.0")
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.to_version, "1.0.0");
        assert_eq!(result.config.unwrap().meta.name, "Version 1");
    }

    #[tokio::test]
    async fn test_rollback_to_previous() {
        let manager = VersionManager::new();

        let mut config = crate::agent::config::config::AgentConfig::default();
        config.meta.version = "1.0.0".to_string();
        manager
            .create_version("test-agent", config, None, None)
            .await
            .unwrap();

        let mut config_v2 = crate::agent::config::config::AgentConfig::default();
        config_v2.meta.version = "1.1.0".to_string();
        manager
            .create_version("test-agent", config_v2, None, None)
            .await
            .unwrap();

        let result = manager.rollback_to_previous("test-agent").await.unwrap();
        assert!(result.success);
        assert_eq!(result.to_version, "1.0.0");
    }

    #[tokio::test]
    async fn test_compute_config_checksum() {
        let config = crate::agent::config::config::AgentConfig::default();

        let checksum1 = compute_config_checksum(&config);
        let checksum2 = compute_config_checksum(&config);
        assert_eq!(checksum1, checksum2);
        assert_eq!(checksum1.len(), 64);
    }

    #[tokio::test]
    async fn test_with_storage_dir() {
        let temp_dir = tempdir().unwrap();
        let manager = VersionManager::new().with_storage_dir(temp_dir.path());

        assert!(manager.storage_dir.is_some());
    }

    #[tokio::test]
    async fn test_with_max_versions() {
        let manager = VersionManager::new().with_max_versions(50);

        assert_eq!(manager.max_versions_per_agent, 50);
    }

    #[tokio::test]
    async fn test_delete_version() {
        let manager = VersionManager::new();

        let mut config = crate::agent::config::config::AgentConfig::default();
        config.meta.version = "1.0.0".to_string();
        manager
            .create_version("test-agent", config, None, None)
            .await
            .unwrap();

        let deleted = manager.delete_version("test-agent", "1.0.0").await.unwrap();
        assert!(deleted);

        let versions = manager.list_versions("test-agent").await;
        assert!(versions.is_empty());
    }

    #[tokio::test]
    async fn test_clear_history() {
        let manager = VersionManager::new();

        let config = crate::agent::config::config::AgentConfig::default();
        manager
            .create_version("test-agent", config, None, None)
            .await
            .unwrap();

        manager.clear_history("test-agent").await;

        let versions = manager.list_versions("test-agent").await;
        assert!(versions.is_empty());
    }
}
