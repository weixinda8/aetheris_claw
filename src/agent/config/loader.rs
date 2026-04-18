use crate::agent::config::config::*;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::env;
use std::path::PathBuf;

lazy_static::lazy_static! {
    static ref ENV_VAR_REGEX: Regex = Regex::new(r"\$\{([A-Z0-9_]+)\}").unwrap();
}

#[derive(Clone)]
pub struct AgentConfigLoader {
    global_config: Option<GlobalAgentConfig>,
}

impl AgentConfigLoader {
    pub fn new() -> Self {
        Self {
            global_config: None,
        }
    }

    pub fn with_global_config(mut self, global_config: GlobalAgentConfig) -> Self {
        self.global_config = Some(global_config);
        self
    }

    pub fn load_from_yaml_file<P: Into<PathBuf>>(
        &self,
        path: P,
    ) -> Result<AgentConfig, AgentConfigError> {
        let path = path.into();
        if !path.exists() {
            return Err(AgentConfigError::NotFound(path));
        }

        let content = std::fs::read_to_string(&path)?;
        let processed_content = self.process_env_vars(&content);
        let mut config: AgentConfig = serde_yaml::from_str(&processed_content)?;

        if let Some(global) = &self.global_config {
            config.merge_with_defaults(&global.defaults);
        }

        config.validate()?;

        Ok(config)
    }

    pub fn load_from_json5_file<P: Into<PathBuf>>(
        &self,
        path: P,
    ) -> Result<AgentConfig, AgentConfigError> {
        let path = path.into();
        if !path.exists() {
            return Err(AgentConfigError::NotFound(path));
        }

        let content = std::fs::read_to_string(&path)?;
        let processed_content = self.process_env_vars(&content);
        let mut config: AgentConfig = json5::from_str(&processed_content)
            .map_err(|e| AgentConfigError::Json5Parse(format!("JSON5 parse error: {}", e)))?;

        if let Some(global) = &self.global_config {
            config.merge_with_defaults(&global.defaults);
        }

        config.validate()?;

        Ok(config)
    }

    pub fn load_from_str(
        &self,
        content: &str,
        format: ConfigFormat,
    ) -> Result<AgentConfig, AgentConfigError> {
        let processed_content = self.process_env_vars(content);
        let mut config: AgentConfig = match format {
            ConfigFormat::Yaml => serde_yaml::from_str(&processed_content)?,
            ConfigFormat::Json5 => json5::from_str(&processed_content)
                .map_err(|e| AgentConfigError::Json5Parse(format!("JSON5 parse error: {}", e)))?,
        };

        if let Some(global) = &self.global_config {
            config.merge_with_defaults(&global.defaults);
        }

        config.validate()?;

        Ok(config)
    }

    pub fn load_global_config<P: Into<PathBuf>>(
        path: P,
    ) -> Result<GlobalAgentConfig, AgentConfigError> {
        let path = path.into();
        if !path.exists() {
            return Err(AgentConfigError::NotFound(path));
        }

        let content = std::fs::read_to_string(&path)?;
        let processed_content = Self::process_env_vars_static(&content);
        let config: GlobalAgentConfig = serde_yaml::from_str(&processed_content)?;

        Ok(config)
    }

    pub fn load_all_from_directory<P: Into<PathBuf>>(
        &self,
        dir: P,
    ) -> Result<Vec<AgentConfig>, AgentConfigError> {
        let dir = dir.into();
        let mut configs = Vec::new();

        if !dir.is_dir() {
            return Ok(configs);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    match ext_str.as_str() {
                        "yaml" | "yml" => {
                            if let Ok(config) = self.load_from_yaml_file(&path) {
                                if config.meta.enabled {
                                    configs.push(config);
                                }
                            }
                        }
                        "json5" => {
                            if let Ok(config) = self.load_from_json5_file(&path) {
                                if config.meta.enabled {
                                    configs.push(config);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(configs)
    }

    pub fn save_to_yaml_file<P: Into<PathBuf>>(
        config: &AgentConfig,
        path: P,
    ) -> Result<(), AgentConfigError> {
        let path = path.into();
        let content = serde_yaml::to_string(config)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn save_to_json5_file<P: Into<PathBuf>>(
        config: &AgentConfig,
        path: P,
    ) -> Result<(), AgentConfigError> {
        let path = path.into();
        let content = json5::to_string(config)
            .map_err(|e| AgentConfigError::Json5Parse(format!("JSON5 serialize error: {}", e)))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn compute_file_hash<P: Into<PathBuf>>(path: P) -> Result<String, AgentConfigError> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)?;
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    pub fn compute_str_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    pub fn detect_changes<P: Into<PathBuf>>(
        path: P,
        previous_hash: &str,
    ) -> Result<bool, AgentConfigError> {
        let path = path.into();
        if !path.exists() {
            return Ok(true);
        }
        let current_hash = Self::compute_file_hash(&path)?;
        Ok(current_hash != previous_hash)
    }

    fn process_env_vars(&self, content: &str) -> String {
        Self::process_env_vars_static(content)
    }

    fn process_env_vars_static(content: &str) -> String {
        let mut result = content.to_string();

        for cap in ENV_VAR_REGEX.captures_iter(content) {
            if let Some(var_name) = cap.get(1) {
                if let Ok(var_value) = env::var(var_name.as_str()) {
                    result = result.replace(&cap[0], &var_value);
                }
            }
        }

        result
    }
}

impl Default for AgentConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_config(workspace: PathBuf) -> AgentConfig {
        AgentConfig {
            meta: AgentMeta {
                id: "test-agent-001".to_string(),
                name: "Test Agent".to_string(),
                version: "1.0.0".to_string(),
                agent_type: crate::agent::base::AgentType::Generic,
                enabled: true,
                hot_reload: true,
                workspace,
                created_at: None,
                updated_at: None,
                tags: None,
                description: None,
            },
            persona: AgentPersona {
                soul_file: None,
                system_prompt: None,
                personality: None,
            },
            model: ModelConfig {
                primary: "gpt-4o".to_string(),
                fallback: None,
                params: ModelParams::default(),
            },
            skills: SkillsConfig {
                enabled: vec![],
                permissions: None,
                priority: None,
            },
            channels: ChannelsConfig {
                wecom: None,
                dingtalk: None,
                feishu: None,
                wechat: None,
            },
            memory: MemoryConfig {
                short_term: None,
                mid_term: None,
                long_term: None,
                versioned_state: None,
            },
            security: SecurityConfig {
                sandbox: None,
                rule_block: None,
                audit: None,
                human_intervene: None,
            },
            scheduler: SchedulerConfig {
                concurrency: None,
                priority: None,
                retry: None,
                timeout_seconds: None,
            },
            capabilities: None,
        }
    }

    #[test]
    fn test_env_var_processing() {
        unsafe {
            std::env::set_var("TEST_VAR", "test_value");
        }

        let content = "model: ${TEST_VAR}";
        let processed = AgentConfigLoader::process_env_vars_static(content);

        assert_eq!(processed, "model: test_value");
    }

    #[test]
    fn test_save_and_load_yaml() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test-agent.yaml");

        let config = create_test_config(temp_dir.path().to_path_buf());

        AgentConfigLoader::save_to_yaml_file(&config, &file_path).unwrap();

        let loader = AgentConfigLoader::new();
        let loaded_config = loader.load_from_yaml_file(&file_path).unwrap();

        assert_eq!(loaded_config.meta.id, config.meta.id);
        assert_eq!(loaded_config.meta.name, config.meta.name);
    }

    #[test]
    fn test_compute_str_hash() {
        let content1 = "test content 1";
        let content2 = "test content 2";

        let hash1 = AgentConfigLoader::compute_str_hash(content1);
        let hash2 = AgentConfigLoader::compute_str_hash(content1);
        let hash3 = AgentConfigLoader::compute_str_hash(content2);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_compute_file_hash() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test-hash.txt");

        std::fs::write(&file_path, "test content").unwrap();

        let hash = AgentConfigLoader::compute_file_hash(&file_path).unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_detect_changes() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test-change.yaml");

        let config = create_test_config(temp_dir.path().to_path_buf());
        AgentConfigLoader::save_to_yaml_file(&config, &file_path).unwrap();

        let initial_hash = AgentConfigLoader::compute_file_hash(&file_path).unwrap();

        assert!(!AgentConfigLoader::detect_changes(&file_path, &initial_hash).unwrap());

        let mut config2 = config.clone();
        config2.meta.name = "Modified Name".to_string();
        AgentConfigLoader::save_to_yaml_file(&config2, &file_path).unwrap();

        assert!(AgentConfigLoader::detect_changes(&file_path, &initial_hash).unwrap());

        let non_existent_path = temp_dir.path().join("non-existent.yaml");
        assert!(AgentConfigLoader::detect_changes(&non_existent_path, &initial_hash).unwrap());
    }
}
