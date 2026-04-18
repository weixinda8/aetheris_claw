use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConfigType {
    Aetheris,
    Soul,
    Skill,
    Agent,
    Security,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigVersion {
    pub version_id: String,
    pub config_type: ConfigType,
    pub config_id: String,
    pub version_number: u32,
    pub config_data: serde_json::Value,
    pub author: String,
    pub message: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub parent_version: Option<String>,
    pub tags: Vec<String>,
    pub is_immutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDiff {
    pub old_version: Option<String>,
    pub new_version: String,
    pub changes: Vec<ConfigChange>,
    pub diff_type: DiffType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiffType {
    Created,
    Modified,
    Deleted,
    Restored,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
    pub path: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub change_type: ChangeType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Modified,
    Removed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigBranch {
    pub branch_id: String,
    pub name: String,
    pub description: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub created_by: String,
    pub head_version: String,
    pub is_protected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMergeRequest {
    pub request_id: String,
    pub title: String,
    pub description: String,
    pub source_branch: String,
    pub target_branch: String,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub status: MergeRequestStatus,
    pub approved_by: Vec<String>,
    pub approvals_required: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MergeRequestStatus {
    Draft,
    Open,
    Approved,
    Rejected,
    Merged,
    Closed,
}

pub struct ConfigVersionControl {
    versions: Arc<DashMap<String, ConfigVersion>>,
    branches: Arc<DashMap<String, ConfigBranch>>,
    merge_requests: Arc<DashMap<String, ConfigMergeRequest>>,
    version_index: Arc<DashMap<(ConfigType, String), Vec<String>>>,
    tag_index: Arc<DashMap<String, Vec<String>>>,
    storage_path: PathBuf,
}

impl ConfigVersionControl {
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;

        let instance = Self {
            versions: Arc::new(DashMap::new()),
            branches: Arc::new(DashMap::new()),
            merge_requests: Arc::new(DashMap::new()),
            version_index: Arc::new(DashMap::new()),
            tag_index: Arc::new(DashMap::new()),
            storage_path,
        };

        instance.load()?;

        Ok(instance)
    }

    pub fn save(&self) -> Result<()> {
        let versions_path = self.storage_path.join("versions.json");
        let branches_path = self.storage_path.join("branches.json");
        let merge_requests_path = self.storage_path.join("merge_requests.json");
        let version_index_path = self.storage_path.join("version_index.json");
        let tag_index_path = self.storage_path.join("tag_index.json");

        let versions: Vec<ConfigVersion> =
            self.versions.iter().map(|v| v.value().clone()).collect();
        let branches: Vec<ConfigBranch> = self.branches.iter().map(|b| b.value().clone()).collect();
        let merge_requests: Vec<ConfigMergeRequest> = self
            .merge_requests
            .iter()
            .map(|m| m.value().clone())
            .collect();
        let version_index: Vec<((ConfigType, String), Vec<String>)> = self
            .version_index
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        let tag_index: Vec<(String, Vec<String>)> = self
            .tag_index
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        std::fs::write(versions_path, serde_json::to_string_pretty(&versions)?)?;
        std::fs::write(branches_path, serde_json::to_string_pretty(&branches)?)?;
        std::fs::write(
            merge_requests_path,
            serde_json::to_string_pretty(&merge_requests)?,
        )?;
        std::fs::write(
            version_index_path,
            serde_json::to_string_pretty(&version_index)?,
        )?;
        std::fs::write(tag_index_path, serde_json::to_string_pretty(&tag_index)?)?;

        info!("ConfigVersionControl saved to: {:?}", self.storage_path);

        Ok(())
    }

    fn load(&self) -> Result<()> {
        let versions_path = self.storage_path.join("versions.json");
        let branches_path = self.storage_path.join("branches.json");
        let merge_requests_path = self.storage_path.join("merge_requests.json");
        let version_index_path = self.storage_path.join("version_index.json");
        let tag_index_path = self.storage_path.join("tag_index.json");

        if versions_path.exists() {
            let content = std::fs::read_to_string(versions_path)?;
            let versions: Vec<ConfigVersion> = serde_json::from_str(&content)?;
            for version in versions {
                self.versions.insert(version.version_id.clone(), version);
            }
        }

        if branches_path.exists() {
            let content = std::fs::read_to_string(branches_path)?;
            let branches: Vec<ConfigBranch> = serde_json::from_str(&content)?;
            for branch in branches {
                self.branches.insert(branch.name.clone(), branch);
            }
        }

        if merge_requests_path.exists() {
            let content = std::fs::read_to_string(merge_requests_path)?;
            let merge_requests: Vec<ConfigMergeRequest> = serde_json::from_str(&content)?;
            for mr in merge_requests {
                self.merge_requests.insert(mr.request_id.clone(), mr);
            }
        }

        if version_index_path.exists() {
            let content = std::fs::read_to_string(version_index_path)?;
            let version_index: Vec<((ConfigType, String), Vec<String>)> =
                serde_json::from_str(&content)?;
            for (key, version_ids) in version_index {
                self.version_index.insert(key, version_ids);
            }
        }

        if tag_index_path.exists() {
            let content = std::fs::read_to_string(tag_index_path)?;
            let tag_index: Vec<(String, Vec<String>)> = serde_json::from_str(&content)?;
            for (tag, version_ids) in tag_index {
                self.tag_index.insert(tag, version_ids);
            }
        }

        info!("ConfigVersionControl loaded from: {:?}", self.storage_path);

        Ok(())
    }

    pub fn create_version(
        &self,
        config_type: ConfigType,
        config_id: String,
        config_data: serde_json::Value,
        author: String,
        message: String,
        parent_version: Option<String>,
    ) -> Result<ConfigVersion> {
        info!(
            "Creating config version: {} ({:?}) by {}",
            config_id, config_type, author
        );

        let key = (config_type.clone(), config_id.clone());
        let mut existing_versions = self.version_index.entry(key).or_default();

        let version_number = existing_versions.len() as u32 + 1;
        let version_id = uuid::Uuid::new_v4().to_string();

        let version = ConfigVersion {
            version_id: version_id.clone(),
            config_type: config_type.clone(),
            config_id: config_id.clone(),
            version_number,
            config_data,
            author,
            message,
            created_at: chrono::Utc::now(),
            parent_version,
            tags: Vec::new(),
            is_immutable: false,
        };

        self.versions.insert(version_id.clone(), version.clone());
        existing_versions.push(version_id.clone());

        self.save()?;

        Ok(version)
    }

    pub fn get_version(&self, version_id: &str) -> Option<ConfigVersion> {
        self.versions.get(version_id).map(|v| v.value().clone())
    }

    pub fn get_latest_version(
        &self,
        config_type: &ConfigType,
        config_id: &str,
    ) -> Option<ConfigVersion> {
        let key = (config_type.clone(), config_id.to_string());
        if let Some(version_ids) = self.version_index.get(&key) {
            version_ids
                .iter()
                .last()
                .and_then(|id| self.get_version(id))
        } else {
            None
        }
    }

    pub fn list_versions(&self, config_type: &ConfigType, config_id: &str) -> Vec<ConfigVersion> {
        let key = (config_type.clone(), config_id.to_string());
        if let Some(version_ids) = self.version_index.get(&key) {
            version_ids
                .iter()
                .filter_map(|id| self.get_version(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn compare_versions(
        &self,
        old_version_id: &str,
        new_version_id: &str,
    ) -> Result<ConfigDiff> {
        let old_version = self.get_version(old_version_id);
        let new_version = self.get_version(new_version_id).ok_or_else(|| {
            AetherisError::NotFound(format!("New version not found: {}", new_version_id))
        })?;

        let diff_type = match (&old_version, &new_version) {
            (None, _) => DiffType::Created,
            (Some(_), _) => DiffType::Modified,
        };

        let changes = self.calculate_changes(
            old_version.as_ref().map(|v| &v.config_data),
            &new_version.config_data,
        );

        Ok(ConfigDiff {
            old_version: old_version.map(|v| v.version_id),
            new_version: new_version.version_id,
            changes,
            diff_type,
        })
    }

    fn calculate_changes(
        &self,
        old_data: Option<&serde_json::Value>,
        new_data: &serde_json::Value,
    ) -> Vec<ConfigChange> {
        let mut changes = Vec::new();
        self.compare_json_values(old_data, new_data, "", &mut changes);
        changes
    }

    fn compare_json_values(
        &self,
        old_value: Option<&serde_json::Value>,
        new_value: &serde_json::Value,
        path: &str,
        changes: &mut Vec<ConfigChange>,
    ) {
        match (old_value, new_value) {
            (None, _) => {
                changes.push(ConfigChange {
                    path: path.to_string(),
                    old_value: None,
                    new_value: Some(new_value.clone()),
                    change_type: ChangeType::Added,
                });
            }
            (Some(old), new) if old != new => match (old, new) {
                (serde_json::Value::Object(old_obj), serde_json::Value::Object(new_obj)) => {
                    let old_keys: HashSet<_> = old_obj.keys().collect();
                    let new_keys: HashSet<_> = new_obj.keys().collect();

                    for key in old_keys.difference(&new_keys) {
                        let sub_path = if path.is_empty() {
                            key.to_string()
                        } else {
                            format!("{}.{}", path, key)
                        };
                        changes.push(ConfigChange {
                            path: sub_path,
                            old_value: old_obj.get(key.as_str()).cloned(),
                            new_value: None,
                            change_type: ChangeType::Removed,
                        });
                    }

                    for key in new_keys.difference(&old_keys) {
                        let sub_path = if path.is_empty() {
                            key.to_string()
                        } else {
                            format!("{}.{}", path, key)
                        };
                        changes.push(ConfigChange {
                            path: sub_path,
                            old_value: None,
                            new_value: new_obj.get(key.as_str()).cloned(),
                            change_type: ChangeType::Added,
                        });
                    }

                    for key in old_keys.intersection(&new_keys) {
                        let sub_path = if path.is_empty() {
                            key.to_string()
                        } else {
                            format!("{}.{}", path, key)
                        };
                        if let (Some(old_val), Some(new_val)) =
                            (old_obj.get(key.as_str()), new_obj.get(key.as_str()))
                        {
                            self.compare_json_values(Some(old_val), new_val, &sub_path, changes);
                        }
                    }
                }
                (serde_json::Value::Array(old_arr), serde_json::Value::Array(new_arr)) => {
                    for (i, (old_item, new_item)) in old_arr.iter().zip(new_arr.iter()).enumerate()
                    {
                        let sub_path = format!("{}[{}]", path, i);
                        self.compare_json_values(Some(old_item), new_item, &sub_path, changes);
                    }

                    if old_arr.len() > new_arr.len() {
                        for i in new_arr.len()..old_arr.len() {
                            let sub_path = format!("{}[{}]", path, i);
                            changes.push(ConfigChange {
                                path: sub_path,
                                old_value: old_arr.get(i).cloned(),
                                new_value: None,
                                change_type: ChangeType::Removed,
                            });
                        }
                    }

                    if new_arr.len() > old_arr.len() {
                        for i in old_arr.len()..new_arr.len() {
                            let sub_path = format!("{}[{}]", path, i);
                            changes.push(ConfigChange {
                                path: sub_path,
                                old_value: None,
                                new_value: new_arr.get(i).cloned(),
                                change_type: ChangeType::Added,
                            });
                        }
                    }
                }
                _ => {
                    changes.push(ConfigChange {
                        path: path.to_string(),
                        old_value: Some(old.clone()),
                        new_value: Some(new.clone()),
                        change_type: ChangeType::Modified,
                    });
                }
            },
            _ => {}
        }
    }

    pub fn restore_version(&self, version_id: &str, author: String) -> Result<ConfigVersion> {
        let version_to_restore = self
            .get_version(version_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Version not found: {}", version_id)))?;

        info!(
            "Restoring version: {} of config: {} ({:?})",
            version_id, version_to_restore.config_id, version_to_restore.config_type
        );

        let restored_version = self.create_version(
            version_to_restore.config_type.clone(),
            version_to_restore.config_id.clone(),
            version_to_restore.config_data.clone(),
            author,
            format!("Restored from version: {}", version_id),
            Some(version_id.to_string()),
        )?;

        Ok(restored_version)
    }

    pub fn create_branch(
        &self,
        name: String,
        description: String,
        created_by: String,
        from_branch: Option<String>,
    ) -> Result<ConfigBranch> {
        if self.branches.contains_key(&name) {
            return Err(AetherisError::Validation(format!(
                "Branch already exists: {}",
                name
            )));
        }

        let head_version = if let Some(from_name) = from_branch {
            let from_branch = self.branches.get(&from_name).ok_or_else(|| {
                AetherisError::NotFound(format!("Branch not found: {}", from_name))
            })?;
            from_branch.head_version.clone()
        } else {
            String::new()
        };

        let branch = ConfigBranch {
            branch_id: uuid::Uuid::new_v4().to_string(),
            name: name.clone(),
            description,
            created_at: chrono::Utc::now(),
            created_by,
            head_version,
            is_protected: false,
        };

        self.branches.insert(name, branch.clone());

        self.save()?;

        Ok(branch)
    }

    pub fn get_branch(&self, name: &str) -> Option<ConfigBranch> {
        self.branches.get(name).map(|b| b.value().clone())
    }

    pub fn list_branches(&self) -> Vec<ConfigBranch> {
        self.branches
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn create_merge_request(
        &self,
        title: String,
        description: String,
        source_branch: String,
        target_branch: String,
        created_by: String,
        approvals_required: u32,
    ) -> Result<ConfigMergeRequest> {
        let request = ConfigMergeRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            title,
            description,
            source_branch,
            target_branch,
            created_by,
            created_at: chrono::Utc::now(),
            status: MergeRequestStatus::Open,
            approved_by: Vec::new(),
            approvals_required,
        };

        self.merge_requests
            .insert(request.request_id.clone(), request.clone());

        self.save()?;

        Ok(request)
    }

    pub fn approve_merge_request(&self, request_id: &str, approved_by: String) -> Result<()> {
        let mut request = self.merge_requests.get_mut(request_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Merge request not found: {}", request_id))
        })?;

        if request.approved_by.contains(&approved_by) {
            return Err(AetherisError::Validation(
                "Already approved by this user".to_string(),
            ));
        }

        request.approved_by.push(approved_by);

        if request.approved_by.len() as u32 >= request.approvals_required {
            request.status = MergeRequestStatus::Approved;
        }

        self.save()?;

        Ok(())
    }

    pub fn merge_merge_request(&self, request_id: &str, merged_by: String) -> Result<()> {
        let mut request = self.merge_requests.get_mut(request_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Merge request not found: {}", request_id))
        })?;

        if request.status != MergeRequestStatus::Approved {
            return Err(AetherisError::Validation(
                "Merge request is not approved".to_string(),
            ));
        }

        request.status = MergeRequestStatus::Merged;

        self.save()?;

        info!("Merge request {} merged by {}", request_id, merged_by);

        Ok(())
    }

    pub fn get_merge_request(&self, request_id: &str) -> Option<ConfigMergeRequest> {
        self.merge_requests
            .get(request_id)
            .map(|r| r.value().clone())
    }

    pub fn list_merge_requests(
        &self,
        status: Option<MergeRequestStatus>,
    ) -> Vec<ConfigMergeRequest> {
        self.merge_requests
            .iter()
            .filter(|entry| {
                if let Some(s) = &status {
                    entry.value().status == *s
                } else {
                    true
                }
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn add_tag(&self, version_id: &str, tag: String) -> Result<()> {
        let mut version = self
            .versions
            .get_mut(version_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Version not found: {}", version_id)))?;

        if !version.tags.contains(&tag) {
            version.tags.push(tag.clone());
            self.tag_index
                .entry(tag)
                .or_default()
                .push(version_id.to_string());

            self.save()?;
        }

        Ok(())
    }

    pub fn get_versions_by_tag(&self, tag: &str) -> Vec<ConfigVersion> {
        if let Some(version_ids) = self.tag_index.get(tag) {
            version_ids
                .iter()
                .filter_map(|id| self.get_version(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn version_count(&self) -> usize {
        self.versions.len()
    }

    pub fn branch_count(&self) -> usize {
        self.branches.len()
    }

    pub fn merge_request_count(&self) -> usize {
        self.merge_requests.len()
    }
}

impl Default for ConfigVersionControl {
    fn default() -> Self {
        let storage_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".aetheris")
            .join("config-versions");

        Self::new(storage_path).unwrap_or_else(|_| {
            let temp_dir = tempfile::tempdir().unwrap();
            Self::new(temp_dir.path().to_path_buf()).unwrap()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_version_control_new() {
        let temp_dir = tempfile::tempdir().unwrap();
        let vcs = ConfigVersionControl::new(temp_dir.path().to_path_buf());
        assert!(vcs.is_ok());
    }

    #[test]
    fn test_create_version() {
        let temp_dir = tempfile::tempdir().unwrap();
        let vcs = ConfigVersionControl::new(temp_dir.path().to_path_buf()).unwrap();

        let config_data = serde_json::json!({"key": "value"});

        let version = vcs
            .create_version(
                ConfigType::Aetheris,
                "test-config".to_string(),
                config_data,
                "test-user".to_string(),
                "Initial version".to_string(),
                None,
            )
            .unwrap();

        assert_eq!(version.version_number, 1);
    }

    #[test]
    fn test_get_latest_version() {
        let temp_dir = tempfile::tempdir().unwrap();
        let vcs = ConfigVersionControl::new(temp_dir.path().to_path_buf()).unwrap();

        let config_data1 = serde_json::json!({"key": "value1"});
        let config_data2 = serde_json::json!({"key": "value2"});

        vcs.create_version(
            ConfigType::Aetheris,
            "test-config".to_string(),
            config_data1,
            "test-user".to_string(),
            "Version 1".to_string(),
            None,
        )
        .unwrap();

        vcs.create_version(
            ConfigType::Aetheris,
            "test-config".to_string(),
            config_data2,
            "test-user".to_string(),
            "Version 2".to_string(),
            None,
        )
        .unwrap();

        let latest = vcs.get_latest_version(&ConfigType::Aetheris, "test-config");
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().version_number, 2);
    }

    #[test]
    fn test_create_branch() {
        let temp_dir = tempfile::tempdir().unwrap();
        let vcs = ConfigVersionControl::new(temp_dir.path().to_path_buf()).unwrap();

        let branch = vcs
            .create_branch(
                "feature".to_string(),
                "Feature branch".to_string(),
                "test-user".to_string(),
                None,
            )
            .unwrap();

        assert_eq!(branch.name, "feature");
    }
}
