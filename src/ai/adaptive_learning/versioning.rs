use super::*;
use dashmap::DashMap;

pub struct ModelVersionManager {
    versions: DashMap<String, ModelVersion>,
    model_versions: DashMap<String, Vec<String>>,
    active_version: DashMap<String, String>,
}

impl ModelVersionManager {
    pub fn new() -> Self {
        Self {
            versions: DashMap::new(),
            model_versions: DashMap::new(),
            active_version: DashMap::new(),
        }
    }

    pub fn create_version(&self, version: ModelVersion) -> String {
        let version_id = version.id.clone();
        let model_id = version.model_id.clone();

        self.model_versions
            .entry(model_id.clone())
            .or_default()
            .push(version_id.clone());

        if !self.active_version.contains_key(&model_id) {
            self.active_version.insert(model_id, version_id.clone());
        }

        self.versions.insert(version_id.clone(), version);

        version_id
    }

    pub fn get_version(&self, version_id: &str) -> Option<ModelVersion> {
        self.versions.get(version_id).map(|v| v.clone())
    }

    pub fn get_active_version(&self, model_id: &str) -> Option<ModelVersion> {
        self.active_version
            .get(model_id)
            .and_then(|id| self.get_version(&id))
    }

    pub fn list_versions(&self, model_id: &str) -> Vec<ModelVersion> {
        self.model_versions
            .get(model_id)
            .map(|ids| {
                let mut versions: Vec<ModelVersion> =
                    ids.iter().filter_map(|id| self.get_version(id)).collect();

                versions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                versions
            })
            .unwrap_or_default()
    }

    pub fn set_active_version(&self, model_id: &str, version_id: &str) -> bool {
        if let Some(mut version) = self.versions.get_mut(version_id) {
            version.is_active = true;
            self.active_version
                .insert(model_id.to_string(), version_id.to_string());

            if let Some(ids) = self.model_versions.get(model_id) {
                for id in ids.iter() {
                    if id != version_id {
                        if let Some(mut v) = self.versions.get_mut(id) {
                            v.is_active = false;
                        }
                    }
                }
            }

            true
        } else {
            false
        }
    }

    pub fn rollback_to_version(&self, model_id: &str, version_id: &str) -> bool {
        self.set_active_version(model_id, version_id)
    }

    pub fn compare_versions(
        &self,
        version_id1: &str,
        version_id2: &str,
    ) -> Option<VersionComparison> {
        let v1 = self.get_version(version_id1)?;
        let v2 = self.get_version(version_id2)?;

        let created_at_diff = v1.created_at.signed_duration_since(v2.created_at);

        Some(VersionComparison {
            version1: v1,
            version2: v2,
            created_at_diff,
        })
    }

    pub fn delete_version(&self, version_id: &str) -> bool {
        if let Some((_, version)) = self.versions.remove(version_id) {
            if let Some(mut ids) = self.model_versions.get_mut(&version.model_id) {
                ids.retain(|id| id != version_id);
            }
            if let Some(active_id) = self.active_version.get(&version.model_id) {
                if *active_id == version_id {
                    self.active_version.remove(&version.model_id);
                }
            }
            true
        } else {
            false
        }
    }
}

impl Default for ModelVersionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionComparison {
    pub version1: ModelVersion,
    pub version2: ModelVersion,
    pub created_at_diff: chrono::Duration,
}
