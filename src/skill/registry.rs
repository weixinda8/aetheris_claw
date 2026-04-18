use crate::skill::{PermissionLevel, Skill, SkillEvaluation, SkillMetadata, Version};
use crate::utils::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VulnerabilitySeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillVersionMetadata {
    pub commit_hash: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub security_approved: bool,
    pub changelog: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub severity: VulnerabilitySeverity,
    pub description: String,
    pub discovered_at: DateTime<Utc>,
    pub fixed_in_version: Option<Version>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityScanResult {
    pub scan_id: String,
    pub scanned_at: DateTime<Utc>,
    pub vulnerabilities: Vec<Vulnerability>,
    pub passed: bool,
}

pub struct SkillRegistry {
    skills: DashMap<String, Arc<dyn Skill>>,
    skill_versions: DashMap<String, BTreeMap<Version, Arc<dyn Skill>>>,
    evaluations: DashMap<String, SkillEvaluation>,
    version_metadata: DashMap<String, BTreeMap<Version, SkillVersionMetadata>>,
    security_scans: DashMap<String, Vec<SecurityScanResult>>,
    sub_skill_manager: Option<Arc<crate::skill::SubSkillManager>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: DashMap::new(),
            skill_versions: DashMap::new(),
            evaluations: DashMap::new(),
            version_metadata: DashMap::new(),
            security_scans: DashMap::new(),
            sub_skill_manager: None,
        }
    }

    pub fn with_sub_skill_manager(mut self, manager: Arc<crate::skill::SubSkillManager>) -> Self {
        self.sub_skill_manager = Some(manager);
        self
    }

    pub fn sub_skill_manager(&self) -> Option<&Arc<crate::skill::SubSkillManager>> {
        self.sub_skill_manager.as_ref()
    }

    pub fn register_with_metadata(
        &self,
        skill: Arc<dyn Skill>,
        metadata: SkillVersionMetadata,
    ) -> Result<()> {
        let skill_metadata = skill.metadata();
        let skill_id = skill_metadata.id.clone();
        let version = skill_metadata.version.clone();

        info!(
            "Registering skill with metadata: {} v{}",
            skill_id,
            version.to_string()
        );

        self.register(skill);

        self.version_metadata
            .entry(skill_id.clone())
            .or_default()
            .insert(version, metadata);

        debug!("Skill with metadata registered successfully: {}", skill_id);
        Ok(())
    }

    pub fn get_version_metadata(
        &self,
        id: &str,
        version: &Version,
    ) -> Result<Option<SkillVersionMetadata>> {
        debug!(
            "Getting version metadata for skill: {} v{}",
            id,
            version.to_string()
        );

        if let Some(skill_metadata) = self.version_metadata.get(id) {
            Ok(skill_metadata.get(version).cloned())
        } else {
            Ok(None)
        }
    }

    pub fn record_security_scan(&self, id: &str, scan_result: SecurityScanResult) -> Result<()> {
        info!(
            "Recording security scan for skill: {}, scan_id: {}",
            id, scan_result.scan_id
        );

        if !self.exists(id) {
            return Err(crate::utils::AetherisError::Skill(format!(
                "Skill not found: {}",
                id
            )));
        }

        self.security_scans
            .entry(id.to_string())
            .or_default()
            .push(scan_result);

        debug!("Security scan recorded successfully for: {}", id);
        Ok(())
    }

    pub fn list_versions_with_metadata(
        &self,
        id: &str,
    ) -> Result<Vec<(Version, SkillMetadata, Option<SkillVersionMetadata>)>> {
        debug!("Listing versions with metadata for skill: {}", id);

        let mut result = Vec::new();

        if let Some(versions) = self.skill_versions.get(id) {
            for (version, skill) in versions.iter() {
                let metadata = skill.metadata().clone();
                let version_metadata = self.get_version_metadata(id, version)?;
                result.push((version.clone(), metadata, version_metadata));
            }
        }

        Ok(result)
    }

    pub fn get_security_scan_history(&self, id: &str) -> Result<Vec<SecurityScanResult>> {
        debug!("Getting security scan history for skill: {}", id);

        Ok(self
            .security_scans
            .get(id)
            .map(|scans| scans.clone())
            .unwrap_or_default())
    }

    pub fn register(&self, skill: Arc<dyn Skill>) {
        let metadata = skill.metadata();
        let skill_id = metadata.id.clone();
        let version = metadata.version.clone();

        info!("Registering skill: {} v{}", skill_id, version.to_string());

        self.skills.insert(skill_id.clone(), skill.clone());

        self.skill_versions
            .entry(skill_id.clone())
            .or_default()
            .insert(version, skill);

        debug!("Skill registered successfully: {}", skill_id);
    }

    pub fn register_with_version(&self, skill: Arc<dyn Skill>, make_default: bool) {
        let metadata = skill.metadata();
        let skill_id = metadata.id.clone();
        let version = metadata.version.clone();

        info!(
            "Registering skill with version: {} v{} (default: {})",
            skill_id,
            version.to_string(),
            make_default
        );

        self.skill_versions
            .entry(skill_id.clone())
            .or_default()
            .insert(version.clone(), skill.clone());

        if make_default {
            self.skills.insert(skill_id, skill);
        }

        debug!("Skill with version registered successfully");
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Skill>> {
        self.skills.get(id).map(|s| s.clone())
    }

    pub fn get_version(&self, id: &str, version: &Version) -> Option<Arc<dyn Skill>> {
        self.skill_versions
            .get(id)
            .and_then(|versions| versions.get(version).cloned())
    }

    pub fn get_latest_version(&self, id: &str) -> Option<Arc<dyn Skill>> {
        self.skill_versions
            .get(id)
            .and_then(|versions| versions.iter().next_back().map(|(_, skill)| skill.clone()))
    }

    pub fn get_compatible_version(
        &self,
        id: &str,
        required_version: &Version,
    ) -> Option<Arc<dyn Skill>> {
        self.skill_versions.get(id).and_then(|versions| {
            versions
                .iter()
                .rev()
                .find(|(v, _)| v.is_compatible_with(required_version))
                .map(|(_, skill)| skill.clone())
        })
    }

    pub fn list(&self) -> Vec<SkillMetadata> {
        self.skills.iter().map(|s| s.metadata().clone()).collect()
    }

    pub fn list_all_versions(&self, id: &str) -> Vec<(Version, SkillMetadata)> {
        self.skill_versions
            .get(id)
            .map(|versions| {
                versions
                    .iter()
                    .map(|(v, skill)| (v.clone(), skill.metadata().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn list_active(&self) -> Vec<SkillMetadata> {
        self.skills
            .iter()
            .filter(|s| s.metadata().is_active)
            .map(|s| s.metadata().clone())
            .collect()
    }

    pub fn find_by_tag(&self, tag: &str) -> Vec<Arc<dyn Skill>> {
        self.skills
            .iter()
            .filter(|s| s.metadata().tags.contains(&tag.to_string()))
            .map(|s| s.clone())
            .collect()
    }

    pub fn find_by_category(&self, category: &str) -> Vec<Arc<dyn Skill>> {
        self.skills
            .iter()
            .filter(|s| s.metadata().categories.contains(&category.to_string()))
            .map(|s| s.clone())
            .collect()
    }

    pub fn find_by_call_mode(&self, mode: &crate::skill::CallMode) -> Vec<Arc<dyn Skill>> {
        self.skills
            .iter()
            .filter(|s| s.metadata().call_mode == *mode)
            .map(|s| s.clone())
            .collect()
    }

    pub fn find_by_permission_level(&self, level: &PermissionLevel) -> Vec<Arc<dyn Skill>> {
        self.skills
            .iter()
            .filter(|s| s.metadata().permission_level == *level)
            .map(|s| s.clone())
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<Arc<dyn Skill>> {
        let query_lower = query.to_lowercase();
        self.skills
            .iter()
            .filter(|s| {
                let metadata = s.metadata();
                metadata.name.to_lowercase().contains(&query_lower)
                    || metadata.description.to_lowercase().contains(&query_lower)
                    || metadata
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .map(|s| s.clone())
            .collect()
    }

    pub fn unregister(&self, id: &str) -> Result<()> {
        info!("Unregistering skill: {}", id);
        self.skills.remove(id);
        self.skill_versions.remove(id);
        self.evaluations
            .retain(|key, _| !key.starts_with(&format!("{}:", id)));
        self.version_metadata.remove(id);
        self.security_scans.remove(id);
        Ok(())
    }

    pub fn unregister_version(&self, id: &str, version: &Version) -> Result<()> {
        info!(
            "Unregistering skill version: {} v{}",
            id,
            version.to_string()
        );

        if let Some(mut versions) = self.skill_versions.get_mut(id) {
            versions.remove(version);

            if let Some(current) = self.skills.get(id) {
                if current.metadata().version == *version {
                    if let Some((_, new_default)) = versions.iter().next_back() {
                        self.skills.insert(id.to_string(), new_default.clone());
                    } else {
                        self.skills.remove(id);
                    }
                }
            }
        }

        if let Some(mut metadata) = self.version_metadata.get_mut(id) {
            metadata.remove(version);
        }

        Ok(())
    }

    pub fn set_default_version(&self, id: &str, version: &Version) -> Result<()> {
        if let Some(skill) = self.get_version(id, version) {
            info!(
                "Setting default version for {} to v{}",
                id,
                version
            );
            self.skills.insert(id.to_string(), skill);
            Ok(())
        } else {
            Err(crate::utils::AetherisError::Skill(format!(
                "Skill version not found: {} v{}",
                id,
                version
            )))
        }
    }

    pub fn deactivate_skill(&self, id: &str) -> Result<()> {
        if let Some(mut skill_entry) = self.skills.get_mut(id) {
            warn!("Deactivating skill: {}", id);
            let mut metadata = skill_entry.metadata().clone();
            metadata.is_active = false;

            let new_skill = crate::skill::BaseSkill::new_arc(metadata);
            *skill_entry = new_skill;
        }
        Ok(())
    }

    pub fn activate_skill(&self, id: &str) -> Result<()> {
        if let Some(mut skill_entry) = self.skills.get_mut(id) {
            info!("Activating skill: {}", id);
            let mut metadata = skill_entry.metadata().clone();
            metadata.is_active = true;

            let new_skill = crate::skill::BaseSkill::new_arc(metadata);
            *skill_entry = new_skill;
        }
        Ok(())
    }

    pub fn deprecate_skill(&self, id: &str, reason: &str) -> Result<()> {
        if let Some(mut skill_entry) = self.skills.get_mut(id) {
            warn!("Deprecating skill: {} - Reason: {}", id, reason);
            let mut metadata = skill_entry.metadata().clone();
            metadata.is_deprecated = true;
            metadata.deprecation_reason = Some(reason.to_string());

            let new_skill = crate::skill::BaseSkill::new_arc(metadata);
            *skill_entry = new_skill;
        }
        Ok(())
    }

    pub fn save_evaluation(&self, evaluation: SkillEvaluation) {
        let key = format!("{}:{}", evaluation.skill_id, evaluation.version);
        self.evaluations.insert(key, evaluation);
    }

    pub fn get_evaluation(&self, skill_id: &str, version: &Version) -> Option<SkillEvaluation> {
        let key = format!("{}:{}", skill_id, version);
        self.evaluations.get(&key).map(|e| e.clone())
    }

    pub fn list_evaluations(&self, skill_id: &str) -> Vec<SkillEvaluation> {
        self.evaluations
            .iter()
            .filter(|e| e.skill_id == skill_id)
            .map(|e| e.clone())
            .collect()
    }

    pub fn skill_count(&self) -> usize {
        self.skills.len()
    }

    pub fn version_count(&self, id: &str) -> usize {
        self.skill_versions
            .get(id)
            .map(|versions| versions.len())
            .unwrap_or(0)
    }

    pub fn exists(&self, id: &str) -> bool {
        self.skills.contains_key(id)
    }

    pub fn version_exists(&self, id: &str, version: &Version) -> bool {
        self.skill_versions
            .get(id)
            .map(|versions| versions.contains_key(version))
            .unwrap_or(false)
    }
}

impl SkillRegistry {
    pub fn with_progressive_disclosure(
        &self,
        strategy: crate::skill::LoadingStrategy,
    ) -> Result<crate::skill::ProgressiveDisclosureManager> {
        crate::skill::ProgressiveDisclosureManager::new(Arc::new(self.clone()), strategy)
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SkillRegistry {
    fn clone(&self) -> Self {
        Self {
            skills: self.skills.clone(),
            skill_versions: self.skill_versions.clone(),
            evaluations: self.evaluations.clone(),
            version_metadata: self.version_metadata.clone(),
            security_scans: self.security_scans.clone(),
            sub_skill_manager: self.sub_skill_manager.clone(),
        }
    }
}
