use crate::skill::stateful_skill::SkillState;
use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateSnapshot {
    pub snapshot_id: String,
    pub skill_id: String,
    pub state: SkillState,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub description: Option<String>,
}

impl StateSnapshot {
    pub fn new(skill_id: String, state: SkillState, description: Option<String>) -> Self {
        use uuid::Uuid;

        Self {
            snapshot_id: Uuid::new_v4().to_string(),
            skill_id,
            state,
            created_at: chrono::Utc::now(),
            description,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkillStateVersion {
    pub version: u64,
    pub state: SkillState,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct SkillStateManager {
    in_memory_states: DashMap<String, SkillState>,
    snapshots: DashMap<String, Vec<StateSnapshot>>,
    state_versions: DashMap<String, BTreeMap<u64, SkillStateVersion>>,
    storage_path: Option<PathBuf>,
    next_version: DashMap<String, u64>,
}

impl SkillStateManager {
    pub fn new() -> Self {
        Self {
            in_memory_states: DashMap::new(),
            snapshots: DashMap::new(),
            state_versions: DashMap::new(),
            storage_path: None,
            next_version: DashMap::new(),
        }
    }

    pub fn with_storage_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();

        if !path_buf.exists() {
            fs::create_dir_all(&path_buf).map_err(|e| {
                AetherisError::Skill(format!("Failed to create state storage directory: {}", e))
            })?;
        }

        Ok(Self {
            in_memory_states: DashMap::new(),
            snapshots: DashMap::new(),
            state_versions: DashMap::new(),
            storage_path: Some(path_buf),
            next_version: DashMap::new(),
        })
    }

    pub fn save_state(&self, skill_id: &str, state: SkillState) -> Result<u64> {
        info!("Saving state for skill: {}", skill_id);

        let version = self.get_next_version(skill_id);

        let state_version = SkillStateVersion {
            version,
            state: state.clone(),
            created_at: chrono::Utc::now(),
        };

        self.state_versions
            .entry(skill_id.to_string())
            .or_default()
            .insert(version, state_version);

        self.in_memory_states
            .insert(skill_id.to_string(), state.clone());

        if let Some(_storage_path) = &self.storage_path {
            self.persist_state_to_disk(skill_id, &state, version)?;
        }

        debug!(
            "State saved for skill: {} with version: {}",
            skill_id, version
        );
        Ok(version)
    }

    pub fn load_state(&self, skill_id: &str) -> Result<Option<SkillState>> {
        debug!("Loading state for skill: {}", skill_id);

        if let Some(state) = self.in_memory_states.get(skill_id) {
            return Ok(Some(state.clone()));
        }

        if let Some(_storage_path) = &self.storage_path {
            if let Some(state) = self.load_latest_state_from_disk(skill_id)? {
                self.in_memory_states
                    .insert(skill_id.to_string(), state.clone());
                return Ok(Some(state));
            }
        }

        Ok(None)
    }

    pub fn load_state_version(&self, skill_id: &str, version: u64) -> Result<Option<SkillState>> {
        debug!("Loading state version {} for skill: {}", version, skill_id);

        if let Some(versions) = self.state_versions.get(skill_id) {
            if let Some(state_version) = versions.get(&version) {
                return Ok(Some(state_version.state.clone()));
            }
        }

        if let Some(_storage_path) = &self.storage_path {
            if let Some(state) = self.load_state_from_disk(skill_id, version)? {
                return Ok(Some(state));
            }
        }

        Ok(None)
    }

    pub fn create_snapshot(&self, skill_id: &str, description: Option<String>) -> Result<String> {
        info!("Creating snapshot for skill: {}", skill_id);

        let state = self.load_state(skill_id)?.ok_or_else(|| {
            AetherisError::Skill(format!("No state found for skill: {}", skill_id))
        })?;

        let snapshot = StateSnapshot::new(skill_id.to_string(), state, description);
        let snapshot_id = snapshot.snapshot_id.clone();

        self.snapshots
            .entry(skill_id.to_string())
            .or_default()
            .push(snapshot.clone());

        if let Some(_storage_path) = &self.storage_path {
            self.persist_snapshot_to_disk(&snapshot)?;
        }

        debug!(
            "Snapshot created for skill: {} with id: {}",
            skill_id, snapshot_id
        );
        Ok(snapshot_id)
    }

    pub fn rollback_to_snapshot(&self, skill_id: &str, snapshot_id: &str) -> Result<()> {
        info!(
            "Rolling back to snapshot {} for skill: {}",
            snapshot_id, skill_id
        );

        let snapshot = self.get_snapshot(skill_id, snapshot_id)?.ok_or_else(|| {
            AetherisError::Skill(format!(
                "Snapshot not found: {} for skill: {}",
                snapshot_id, skill_id
            ))
        })?;

        self.save_state(skill_id, snapshot.state.clone())?;

        debug!(
            "Rollback completed for skill: {} to snapshot: {}",
            skill_id, snapshot_id
        );
        Ok(())
    }

    pub fn get_snapshot(&self, skill_id: &str, snapshot_id: &str) -> Result<Option<StateSnapshot>> {
        if let Some(snapshots) = self.snapshots.get(skill_id) {
            if let Some(snapshot) = snapshots.iter().find(|s| s.snapshot_id == snapshot_id) {
                return Ok(Some(snapshot.clone()));
            }
        }

        if let Some(_storage_path) = &self.storage_path {
            if let Some(snapshot) = self.load_snapshot_from_disk(skill_id, snapshot_id)? {
                return Ok(Some(snapshot));
            }
        }

        Ok(None)
    }

    pub fn list_snapshots(&self, skill_id: &str) -> Vec<StateSnapshot> {
        self.snapshots
            .get(skill_id)
            .map(|snapshots| snapshots.clone())
            .unwrap_or_default()
    }

    pub fn list_versions(&self, skill_id: &str) -> Vec<u64> {
        self.state_versions
            .get(skill_id)
            .map(|versions| versions.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn delete_state(&self, skill_id: &str) -> Result<()> {
        info!("Deleting state for skill: {}", skill_id);

        self.in_memory_states.remove(skill_id);
        self.snapshots.remove(skill_id);
        self.state_versions.remove(skill_id);
        self.next_version.remove(skill_id);

        if let Some(_storage_path) = &self.storage_path {
            self.delete_state_from_disk(skill_id)?;
        }

        Ok(())
    }

    pub fn clear(&self) {
        debug!("Clearing state manager");
        self.in_memory_states.clear();
        self.snapshots.clear();
        self.state_versions.clear();
        self.next_version.clear();
    }

    fn get_next_version(&self, skill_id: &str) -> u64 {
        let mut next = self.next_version.entry(skill_id.to_string()).or_insert(1);
        let version = *next;
        *next += 1;
        version
    }

    fn persist_state_to_disk(
        &self,
        skill_id: &str,
        state: &SkillState,
        version: u64,
    ) -> Result<()> {
        if let Some(storage_path) = &self.storage_path {
            let skill_dir = storage_path.join(skill_id);
            fs::create_dir_all(&skill_dir).map_err(|e| {
                AetherisError::Skill(format!("Failed to create skill state directory: {}", e))
            })?;

            let state_path = skill_dir.join(format!("state_v{}.json", version));
            let state_json = serde_json::to_string_pretty(state)
                .map_err(|e| AetherisError::Skill(format!("Failed to serialize state: {}", e)))?;

            fs::write(&state_path, &state_json).map_err(|e| {
                AetherisError::Skill(format!("Failed to write state to disk: {}", e))
            })?;

            let latest_path = skill_dir.join("state_latest.json");
            fs::write(latest_path, state_json).map_err(|e| {
                AetherisError::Skill(format!("Failed to write latest state: {}", e))
            })?;
        }
        Ok(())
    }

    fn load_latest_state_from_disk(&self, skill_id: &str) -> Result<Option<SkillState>> {
        if let Some(storage_path) = &self.storage_path {
            let latest_path = storage_path.join(skill_id).join("state_latest.json");
            if latest_path.exists() {
                let content = fs::read_to_string(&latest_path).map_err(|e| {
                    AetherisError::Skill(format!("Failed to read latest state: {}", e))
                })?;

                let state: SkillState = serde_json::from_str(&content).map_err(|e| {
                    AetherisError::Skill(format!("Failed to deserialize state: {}", e))
                })?;

                return Ok(Some(state));
            }
        }
        Ok(None)
    }

    fn load_state_from_disk(&self, skill_id: &str, version: u64) -> Result<Option<SkillState>> {
        if let Some(storage_path) = &self.storage_path {
            let state_path = storage_path
                .join(skill_id)
                .join(format!("state_v{}.json", version));
            if state_path.exists() {
                let content = fs::read_to_string(&state_path)
                    .map_err(|e| AetherisError::Skill(format!("Failed to read state: {}", e)))?;

                let state: SkillState = serde_json::from_str(&content).map_err(|e| {
                    AetherisError::Skill(format!("Failed to deserialize state: {}", e))
                })?;

                return Ok(Some(state));
            }
        }
        Ok(None)
    }

    fn persist_snapshot_to_disk(&self, snapshot: &StateSnapshot) -> Result<()> {
        if let Some(storage_path) = &self.storage_path {
            let skill_dir = storage_path.join(&snapshot.skill_id);
            let snapshots_dir = skill_dir.join("snapshots");
            fs::create_dir_all(&snapshots_dir).map_err(|e| {
                AetherisError::Skill(format!("Failed to create snapshots directory: {}", e))
            })?;

            let snapshot_path = snapshots_dir.join(format!("{}.json", snapshot.snapshot_id));
            let snapshot_json = serde_json::to_string_pretty(snapshot).map_err(|e| {
                AetherisError::Skill(format!("Failed to serialize snapshot: {}", e))
            })?;

            fs::write(&snapshot_path, snapshot_json).map_err(|e| {
                AetherisError::Skill(format!("Failed to write snapshot to disk: {}", e))
            })?;
        }
        Ok(())
    }

    fn load_snapshot_from_disk(
        &self,
        skill_id: &str,
        snapshot_id: &str,
    ) -> Result<Option<StateSnapshot>> {
        if let Some(storage_path) = &self.storage_path {
            let snapshot_path = storage_path
                .join(skill_id)
                .join("snapshots")
                .join(format!("{}.json", snapshot_id));

            if snapshot_path.exists() {
                let content = fs::read_to_string(&snapshot_path)
                    .map_err(|e| AetherisError::Skill(format!("Failed to read snapshot: {}", e)))?;

                let snapshot: StateSnapshot = serde_json::from_str(&content).map_err(|e| {
                    AetherisError::Skill(format!("Failed to deserialize snapshot: {}", e))
                })?;

                return Ok(Some(snapshot));
            }
        }
        Ok(None)
    }

    fn delete_state_from_disk(&self, skill_id: &str) -> Result<()> {
        if let Some(storage_path) = &self.storage_path {
            let skill_dir = storage_path.join(skill_id);
            if skill_dir.exists() {
                fs::remove_dir_all(&skill_dir).map_err(|e| {
                    AetherisError::Skill(format!("Failed to delete state directory: {}", e))
                })?;
            }
        }
        Ok(())
    }
}

impl Default for SkillStateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::stateful_skill::SkillState;
    use tempfile::tempdir;

    #[test]
    fn test_skill_state_manager_new() {
        let manager = SkillStateManager::new();
        assert!(manager.storage_path.is_none());
    }

    #[test]
    fn test_with_storage_path() {
        let temp_dir = tempdir().unwrap();
        let manager = SkillStateManager::with_storage_path(temp_dir.path()).unwrap();
        assert!(manager.storage_path.is_some());
    }

    #[test]
    fn test_save_and_load_state() {
        let manager = SkillStateManager::new();
        let state_data = serde_json::json!({"key": "value"});
        let state = SkillState::new(
            "test-skill".to_string(),
            "1.0.0".to_string(),
            state_data.clone(),
        );

        let version = manager.save_state("test-skill", state).unwrap();
        assert_eq!(version, 1);

        let loaded = manager.load_state("test-skill").unwrap().unwrap();
        assert_eq!(loaded.state_data, state_data);
    }

    #[test]
    fn test_state_versions() {
        let manager = SkillStateManager::new();

        let state1 = SkillState::new(
            "test-skill".to_string(),
            "1.0.0".to_string(),
            serde_json::json!({"v": 1}),
        );
        manager.save_state("test-skill", state1).unwrap();

        let state2 = SkillState::new(
            "test-skill".to_string(),
            "1.0.0".to_string(),
            serde_json::json!({"v": 2}),
        );
        manager.save_state("test-skill", state2).unwrap();

        let versions = manager.list_versions("test-skill");
        assert_eq!(versions, vec![1, 2]);

        let loaded_v1 = manager
            .load_state_version("test-skill", 1)
            .unwrap()
            .unwrap();
        assert_eq!(loaded_v1.state_data, serde_json::json!({"v": 1}));
    }

    #[test]
    fn test_snapshots() {
        let manager = SkillStateManager::new();
        let state = SkillState::new(
            "test-skill".to_string(),
            "1.0.0".to_string(),
            serde_json::json!({"key": "value"}),
        );
        manager.save_state("test-skill", state).unwrap();

        let snapshot_id = manager
            .create_snapshot("test-skill", Some("test snapshot".to_string()))
            .unwrap();

        let snapshots = manager.list_snapshots("test-skill");
        assert_eq!(snapshots.len(), 1);

        let retrieved = manager
            .get_snapshot("test-skill", &snapshot_id)
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.description, Some("test snapshot".to_string()));
    }

    #[test]
    fn test_rollback_to_snapshot() {
        let manager = SkillStateManager::new();

        let state1 = SkillState::new(
            "test-skill".to_string(),
            "1.0.0".to_string(),
            serde_json::json!({"v": 1}),
        );
        manager.save_state("test-skill", state1).unwrap();

        let snapshot_id = manager.create_snapshot("test-skill", None).unwrap();

        let state2 = SkillState::new(
            "test-skill".to_string(),
            "1.0.0".to_string(),
            serde_json::json!({"v": 2}),
        );
        manager.save_state("test-skill", state2).unwrap();

        let current = manager.load_state("test-skill").unwrap().unwrap();
        assert_eq!(current.state_data, serde_json::json!({"v": 2}));

        manager
            .rollback_to_snapshot("test-skill", &snapshot_id)
            .unwrap();

        let rolled_back = manager.load_state("test-skill").unwrap().unwrap();
        assert_eq!(rolled_back.state_data, serde_json::json!({"v": 1}));
    }

    #[test]
    fn test_delete_state() {
        let manager = SkillStateManager::new();
        let state = SkillState::new(
            "test-skill".to_string(),
            "1.0.0".to_string(),
            serde_json::json!({"key": "value"}),
        );
        manager.save_state("test-skill", state).unwrap();

        assert!(manager.load_state("test-skill").unwrap().is_some());

        manager.delete_state("test-skill").unwrap();

        assert!(manager.load_state("test-skill").unwrap().is_none());
    }

    #[tokio::test]
    async fn test_persistence_with_storage() {
        let temp_dir = tempdir().unwrap();
        let manager = SkillStateManager::with_storage_path(temp_dir.path()).unwrap();

        let state = SkillState::new(
            "test-skill".to_string(),
            "1.0.0".to_string(),
            serde_json::json!({"key": "value"}),
        );
        manager.save_state("test-skill", state).unwrap();

        let new_manager = SkillStateManager::with_storage_path(temp_dir.path()).unwrap();
        let loaded = new_manager
            .load_latest_state_from_disk("test-skill")
            .unwrap()
            .unwrap();
        assert_eq!(loaded.state_data, serde_json::json!({"key": "value"}));
    }
}
