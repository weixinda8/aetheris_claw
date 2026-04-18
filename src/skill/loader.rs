use crate::skill::{
    BaseSkill, CallMode, PermissionLevel, Skill, SkillMetadata, SubSkillManager, Version,
    agentskills::AgentSkillManifest,
    metadata_index::MetadataIndexStore,
};
use crate::utils::{AetherisError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfigFile {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub long_description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub author: Option<String>,
    pub call_mode: Option<String>,
    pub permission_level: Option<String>,
    pub required_permissions: Option<Vec<String>>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub example_input: Option<serde_json::Value>,
    pub example_output: Option<serde_json::Value>,
    pub dependencies: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub struct SkillLoader {
    base_path: String,
    metadata_index_store: Option<std::sync::Arc<crate::skill::InMemoryMetadataIndexStore>>,
    sub_skill_manager: Option<Arc<SubSkillManager>>,
}

impl SkillLoader {
    pub fn new() -> Self {
        Self {
            base_path: "./skills".to_string(),
            metadata_index_store: None,
            sub_skill_manager: None,
        }
    }

    pub fn with_base_path(base_path: String) -> Self {
        Self {
            base_path,
            metadata_index_store: None,
            sub_skill_manager: None,
        }
    }

    pub fn with_metadata_index(
        mut self,
        index_store: std::sync::Arc<crate::skill::InMemoryMetadataIndexStore>,
    ) -> Self {
        self.metadata_index_store = Some(index_store);
        self
    }

    pub fn with_sub_skill_manager(mut self, manager: Arc<SubSkillManager>) -> Self {
        self.sub_skill_manager = Some(manager);
        self
    }

    pub fn sub_skill_manager(&self) -> Option<&Arc<SubSkillManager>> {
        self.sub_skill_manager.as_ref()
    }

    pub async fn load_from_path(&self, path: &str) -> Result<Vec<Arc<dyn Skill>>> {
        info!("Loading skills from path: {}", path);

        let mut visited = HashSet::new();
        let mut skills = Vec::new();
        let path_obj = Path::new(path);

        if !path_obj.exists() {
            warn!("Path does not exist: {}", path);
            return Ok(skills);
        }

        if path_obj.is_dir() {
            skills.extend(self.load_from_directory_sync_with_cycle_detection(path, &mut visited)?);
        } else if path_obj.is_file() {
            if let Some(skill) = self.load_from_file_sync(path)? {
                if let Some(index_store) = &self.metadata_index_store {
                    index_store.index_metadata(skill.metadata().clone())?;
                }
                skills.push(skill);
            }
        }

        info!("Loaded {} skills from path: {}", skills.len(), path);
        Ok(skills)
    }

    fn load_from_directory_sync_with_cycle_detection(
        &self,
        dir_path: &str,
        visited: &mut HashSet<String>,
    ) -> Result<Vec<Arc<dyn Skill>>> {
        let dir_path_buf = Path::new(dir_path);
        let canonical_dir_path = dir_path_buf
            .canonicalize()
            .map_err(|e| AetherisError::Skill(format!("Failed to canonicalize path: {}", e)))?;
        let canonical_dir_str = canonical_dir_path.to_string_lossy().to_string();

        if visited.contains(&canonical_dir_str) {
            return Err(AetherisError::Skill(format!(
                "Circular dependency detected in skill directory: {}",
                canonical_dir_str
            )));
        }

        visited.insert(canonical_dir_str.clone());
        debug!(
            "Loading skills from directory with cycle detection: {}",
            dir_path
        );

        let mut skills = Vec::new();
        let dir = fs::read_dir(dir_path)?;

        for entry in dir {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let skill_md_path = path.join("SKILL.md");
                if skill_md_path.exists() {
                    if let Some(skill) =
                        self.load_from_skill_md_sync(skill_md_path.to_str().unwrap())?
                    {
                        if let Some(index_store) = &self.metadata_index_store {
                            index_store.index_metadata(skill.metadata().clone())?;
                        }
                        skills.push(skill);
                    }

                    let sub_skills_dir = path.join("sub-skills");
                    if sub_skills_dir.is_dir() {
                        info!(
                            "Found sub-skills directory, loading recursively: {:?}",
                            sub_skills_dir
                        );
                        if let Ok(mut sub_skills) = self
                            .load_from_directory_sync_with_cycle_detection(
                                sub_skills_dir.to_str().unwrap(),
                                visited,
                            )
                        {
                            for sub_skill in sub_skills.iter() {
                                if let Some(index_store) = &self.metadata_index_store {
                                    index_store.index_metadata(sub_skill.metadata().clone())?;
                                }
                            }
                            skills.append(&mut sub_skills);
                        }
                    }
                } else if let Ok(mut dir_skills) = self
                    .load_from_directory_sync_with_cycle_detection(path.to_str().unwrap(), visited)
                {
                    for skill in dir_skills.iter() {
                        if let Some(index_store) = &self.metadata_index_store {
                            index_store.index_metadata(skill.metadata().clone())?;
                        }
                    }
                    skills.append(&mut dir_skills);
                }
            } else if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name == "SKILL.md" {
                    if let Some(skill) = self.load_from_skill_md_sync(path.to_str().unwrap())? {
                        if let Some(index_store) = &self.metadata_index_store {
                            index_store.index_metadata(skill.metadata().clone())?;
                        }
                        skills.push(skill);
                    }
                } else if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if ext_str == "toml" || ext_str == "yaml" || ext_str == "yml" {
                        if let Some(skill) = self.load_from_file_sync(path.to_str().unwrap())? {
                            if let Some(index_store) = &self.metadata_index_store {
                                index_store.index_metadata(skill.metadata().clone())?;
                            }
                            skills.push(skill);
                        }
                    }
                }
            }
        }

        visited.remove(&canonical_dir_str);
        Ok(skills)
    }

    fn load_from_directory_sync(&self, dir_path: &str) -> Result<Vec<Arc<dyn Skill>>> {
        let mut visited = HashSet::new();
        self.load_from_directory_sync_with_cycle_detection(dir_path, &mut visited)
    }

    fn load_from_file_sync(&self, file_path: &str) -> Result<Option<Arc<dyn Skill>>> {
        debug!("Loading skill from file: {}", file_path);

        let content = fs::read_to_string(file_path)?;
        let path = Path::new(file_path);
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let config: SkillConfigFile = match ext.to_lowercase().as_str() {
            "toml" => toml::from_str(&content)
                .map_err(|e| AetherisError::Skill(format!("TOML parse error: {}", e)))?,
            "yaml" | "yml" => serde_yaml::from_str(&content)
                .map_err(|e| AetherisError::Skill(format!("YAML parse error: {}", e)))?,
            _ => {
                warn!("Unsupported file format: {}", ext);
                return Ok(None);
            }
        };

        let metadata = self.config_to_metadata(config)?;
        let skill = Arc::new(BaseSkill::new(metadata));

        Ok(Some(skill))
    }

    fn load_from_skill_md_sync(&self, file_path: &str) -> Result<Option<Arc<dyn Skill>>> {
        debug!("Loading skill from SKILL.md: {}", file_path);

        let manifest = AgentSkillManifest::from_path(file_path)?;
        let metadata = self.agent_skill_manifest_to_metadata(manifest)?;
        let skill = Arc::new(BaseSkill::new(metadata));

        Ok(Some(skill))
    }

    fn agent_skill_manifest_to_metadata(
        &self,
        manifest: AgentSkillManifest,
    ) -> Result<SkillMetadata> {
        let version = Version::from_string(&manifest.metadata.version)
            .unwrap_or_else(|_| Version::new(0, 1, 0));

        let mut metadata = SkillMetadata::new(
            manifest.metadata.id,
            manifest.metadata.name,
            version,
            manifest.metadata.description,
        );

        metadata.long_description = manifest.metadata.long_description;
        metadata.author = manifest.metadata.author;
        metadata.tags = manifest.metadata.tags;
        metadata.categories = manifest.metadata.categories;
        metadata.dependencies = manifest.dependencies;
        metadata.required_permissions = manifest.permissions;
        metadata.is_deprecated = manifest.metadata.deprecated;

        Ok(metadata)
    }

    fn config_to_metadata(&self, config: SkillConfigFile) -> Result<SkillMetadata> {
        let version =
            Version::from_string(&config.version).unwrap_or_else(|_| Version::new(0, 1, 0));

        let call_mode = match config.call_mode.as_deref() {
            Some("api") => CallMode::Api,
            Some("database") => CallMode::Database,
            Some("image") => CallMode::Image,
            Some("audio") => CallMode::Audio,
            Some("hybrid") => CallMode::Hybrid,
            _ => CallMode::Text,
        };

        let permission_level = match config.permission_level.as_deref() {
            Some("internal") => PermissionLevel::Internal,
            Some("restricted") => PermissionLevel::Restricted,
            Some("admin") => PermissionLevel::Admin,
            _ => PermissionLevel::Public,
        };

        let mut metadata = SkillMetadata::new(config.id, config.name, version, config.description);

        metadata.long_description = config.long_description;
        metadata.tags = config.tags.unwrap_or_default();
        metadata.categories = config.categories.unwrap_or_default();
        metadata.author = config.author;
        metadata.call_mode = call_mode;
        metadata.permission_level = permission_level;
        metadata.required_permissions = config.required_permissions.unwrap_or_default();
        metadata.input_schema = config.input_schema;
        metadata.output_schema = config.output_schema;
        metadata.example_input = config.example_input;
        metadata.example_output = config.example_output;
        metadata.dependencies = config.dependencies.unwrap_or_default();
        metadata.metadata = config.metadata.unwrap_or_default();

        Ok(metadata)
    }

    pub async fn load_all(&self) -> Result<Vec<Arc<dyn Skill>>> {
        self.load_from_path(&self.base_path).await
    }

    pub fn create_in_memory_skill(
        &self,
        id: String,
        name: String,
        version: Version,
        description: String,
    ) -> Arc<dyn Skill> {
        let metadata = SkillMetadata::new(id, name, version, description);
        Arc::new(BaseSkill::new(metadata))
    }
}

impl Default for SkillLoader {
    fn default() -> Self {
        Self::new()
    }
}
