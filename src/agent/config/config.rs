use crate::agent::base::{AgentCapabilities, AgentType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("JSON5 parse error: {0}")]
    Json5Parse(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Config not found: {0}")]
    NotFound(PathBuf),
    #[error("Invalid version: {0}")]
    InvalidVersion(String),
    #[error("Semver error: {0}")]
    Semver(#[from] semver::Error),
    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfigFormat {
    Yaml,
    Json5,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StorageBackend {
    #[default]
    Local,
    Etcd,
    Consul,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(rename = "type")]
    pub agent_type: AgentType,
    pub enabled: bool,
    pub hot_reload: bool,
    pub workspace: PathBuf,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub tags: Option<Vec<String>>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentPersona {
    pub soul_file: Option<PathBuf>,
    pub system_prompt: Option<String>,
    pub personality: Option<PersonalityConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityConfig {
    pub tone: Option<String>,
    pub style: Option<String>,
    pub language: Option<String>,
    pub humor_level: Option<f64>,
    pub formality: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub primary: String,
    pub fallback: Option<Vec<String>>,
    pub params: ModelParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParams {
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub top_k: Option<u32>,
    pub presence_penalty: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub timeout_seconds: Option<u64>,
}

impl Default for ModelParams {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            max_tokens: Some(4096),
            top_p: Some(0.95),
            top_k: None,
            presence_penalty: None,
            frequency_penalty: None,
            timeout_seconds: Some(30),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillsConfig {
    pub enabled: Vec<String>,
    pub permissions: Option<Vec<String>>,
    pub priority: Option<HashMap<String, SkillPriority>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillPriority {
    Mandatory,
    High,
    Medium,
    Low,
    OnDemand,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelsConfig {
    pub wecom: Option<WeComConfig>,
    pub dingtalk: Option<DingTalkConfig>,
    pub feishu: Option<FeishuConfig>,
    pub wechat: Option<WeChatConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeComConfig {
    pub enabled: bool,
    pub corpid: String,
    pub agentid: String,
    pub secret: String,
    pub token: Option<String>,
    pub encoding_aes_key: Option<String>,
    pub msg_format: Option<String>,
    pub rate_limit: Option<String>,
    pub webhook_url: Option<String>,
    pub webhook_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkConfig {
    pub enabled: bool,
    pub app_key: String,
    pub app_secret: String,
    pub robot_code: Option<String>,
    pub workbench_notice: Option<bool>,
    pub webhook_url: Option<String>,
    pub webhook_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    pub enabled: bool,
    pub app_id: String,
    pub app_secret: String,
    pub document_link: Option<bool>,
    pub rich_text: Option<bool>,
    pub webhook_url: Option<String>,
    pub webhook_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeChatConfig {
    pub enabled: bool,
    pub ilink_enabled: bool,
    pub ilink_server: Option<String>,
    pub ilink_token: Option<String>,
    pub webhook_url: Option<String>,
    pub poll_interval_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryConfig {
    pub short_term: Option<ShortTermMemoryConfig>,
    pub mid_term: Option<MidTermMemoryConfig>,
    pub long_term: Option<LongTermMemoryConfig>,
    pub versioned_state: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShortTermMemoryConfig {
    pub capacity: Option<usize>,
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MidTermMemoryConfig {
    pub enable: Option<bool>,
    pub persist: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LongTermMemoryConfig {
    pub enable: Option<bool>,
    pub vector_db: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    pub sandbox: Option<SandboxConfig>,
    pub rule_block: Option<bool>,
    pub audit: Option<bool>,
    pub human_intervene: Option<HumanInterveneConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SandboxMode {
    #[default]
    Isolated,
    Shared,
    Disabled,
}

impl Default for AgentMeta {
    fn default() -> Self {
        Self {
            id: "default-agent".to_string(),
            name: "Default Agent".to_string(),
            version: "1.0.0".to_string(),
            agent_type: AgentType::Generic,
            enabled: true,
            hot_reload: false,
            workspace: PathBuf::from("."),
            created_at: None,
            updated_at: None,
            tags: None,
            description: None,
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            primary: "gpt-4o".to_string(),
            fallback: None,
            params: ModelParams::default(),
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub mode: SandboxMode,
    pub image: Option<String>,
    pub network: Option<String>,
    pub mount_paths: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanInterveneConfig {
    pub enable: bool,
    pub channel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SchedulerConfig {
    pub concurrency: Option<usize>,
    pub priority: Option<u8>,
    pub retry: Option<u32>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct IndustrialProtocolIntegrationConfig {
    pub enabled: bool,
    pub protocol_config: Option<crate::protocol::industrial::types::IndustrialProtocolConfig>,
    pub subscription_config: Option<crate::protocol::industrial::types::SubscriptionConfig>,
    pub tag_mappings: Option<std::collections::HashMap<String, String>>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AgentConfig {
    pub meta: AgentMeta,
    pub persona: AgentPersona,
    pub model: ModelConfig,
    pub skills: SkillsConfig,
    pub channels: ChannelsConfig,
    pub memory: MemoryConfig,
    pub security: SecurityConfig,
    pub scheduler: SchedulerConfig,
    pub capabilities: Option<AgentCapabilities>,
    pub industrial_protocol: Option<IndustrialProtocolIntegrationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalAgentConfig {
    pub defaults: AgentDefaults,
    pub storage: Option<StorageConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefaults {
    pub model: ModelConfig,
    pub memory: MemoryConfig,
    pub security: SecurityConfig,
    pub scheduler: SchedulerConfig,
    pub skills: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub backend: StorageBackend,
    pub local: Option<LocalStorageConfig>,
    pub etcd: Option<EtcdStorageConfig>,
    pub consul: Option<ConsulStorageConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStorageConfig {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtcdStorageConfig {
    pub endpoints: Vec<String>,
    pub prefix: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsulStorageConfig {
    pub address: String,
    pub prefix: Option<String>,
    pub token: Option<String>,
}

impl AgentConfig {
    pub fn validate(&self) -> Result<(), AgentConfigError> {
        if self.meta.id.is_empty() {
            return Err(AgentConfigError::Validation(
                "Agent ID cannot be empty".to_string(),
            ));
        }

        if self.meta.name.is_empty() {
            return Err(AgentConfigError::Validation(
                "Agent name cannot be empty".to_string(),
            ));
        }

        if self.model.primary.is_empty() {
            return Err(AgentConfigError::Validation(
                "Primary model cannot be empty".to_string(),
            ));
        }

        semver::Version::parse(&self.meta.version)?;

        if let Some(wecom) = &self.channels.wecom {
            if wecom.enabled {
                if wecom.corpid.is_empty() {
                    return Err(AgentConfigError::Validation(
                        "WeCom corpid cannot be empty when enabled".to_string(),
                    ));
                }
                if wecom.agentid.is_empty() {
                    return Err(AgentConfigError::Validation(
                        "WeCom agentid cannot be empty when enabled".to_string(),
                    ));
                }
                if wecom.secret.is_empty() {
                    return Err(AgentConfigError::Validation(
                        "WeCom secret cannot be empty when enabled".to_string(),
                    ));
                }
            }
        }

        if let Some(dingtalk) = &self.channels.dingtalk {
            if dingtalk.enabled {
                if dingtalk.app_key.is_empty() {
                    return Err(AgentConfigError::Validation(
                        "DingTalk app_key cannot be empty when enabled".to_string(),
                    ));
                }
                if dingtalk.app_secret.is_empty() {
                    return Err(AgentConfigError::Validation(
                        "DingTalk app_secret cannot be empty when enabled".to_string(),
                    ));
                }
            }
        }

        if let Some(feishu) = &self.channels.feishu {
            if feishu.enabled {
                if feishu.app_id.is_empty() {
                    return Err(AgentConfigError::Validation(
                        "Feishu app_id cannot be empty when enabled".to_string(),
                    ));
                }
                if feishu.app_secret.is_empty() {
                    return Err(AgentConfigError::Validation(
                        "Feishu app_secret cannot be empty when enabled".to_string(),
                    ));
                }
            }
        }

        if let Some(wechat) = &self.channels.wechat {
            if wechat.enabled
                && wechat.ilink_enabled {
                    if wechat.ilink_server.is_none()
                        || wechat.ilink_server.as_ref().unwrap().is_empty()
                    {
                        return Err(AgentConfigError::Validation(
                            "WeChat ilink_server cannot be empty when ilink_enabled".to_string(),
                        ));
                    }
                    if wechat.ilink_token.is_none()
                        || wechat.ilink_token.as_ref().unwrap().is_empty()
                    {
                        return Err(AgentConfigError::Validation(
                            "WeChat ilink_token cannot be empty when ilink_enabled".to_string(),
                        ));
                    }
                }
        }

        Ok(())
    }

    pub fn merge_with_defaults(&mut self, defaults: &AgentDefaults) {
        if self.model.fallback.is_none() {
            self.model.fallback = defaults.model.fallback.clone();
        }
        if self.model.params.temperature.is_none() {
            self.model.params.temperature = defaults.model.params.temperature;
        }
        if self.model.params.max_tokens.is_none() {
            self.model.params.max_tokens = defaults.model.params.max_tokens;
        }
        if self.model.params.top_p.is_none() {
            self.model.params.top_p = defaults.model.params.top_p;
        }
        if self.model.params.top_k.is_none() {
            self.model.params.top_k = defaults.model.params.top_k;
        }
        if self.model.params.presence_penalty.is_none() {
            self.model.params.presence_penalty = defaults.model.params.presence_penalty;
        }
        if self.model.params.frequency_penalty.is_none() {
            self.model.params.frequency_penalty = defaults.model.params.frequency_penalty;
        }
        if self.model.params.timeout_seconds.is_none() {
            self.model.params.timeout_seconds = defaults.model.params.timeout_seconds;
        }

        if self.memory.short_term.is_none() {
            self.memory.short_term = defaults.memory.short_term.clone();
        }
        if self.memory.mid_term.is_none() {
            self.memory.mid_term = defaults.memory.mid_term.clone();
        }
        if self.memory.long_term.is_none() {
            self.memory.long_term = defaults.memory.long_term.clone();
        }
        if self.memory.versioned_state.is_none() {
            self.memory.versioned_state = defaults.memory.versioned_state;
        }

        if self.security.sandbox.is_none() {
            self.security.sandbox = defaults.security.sandbox.clone();
        }
        if self.security.rule_block.is_none() {
            self.security.rule_block = defaults.security.rule_block;
        }
        if self.security.audit.is_none() {
            self.security.audit = defaults.security.audit;
        }
        if self.security.human_intervene.is_none() {
            self.security.human_intervene = defaults.security.human_intervene.clone();
        }

        if self.scheduler.concurrency.is_none() {
            self.scheduler.concurrency = defaults.scheduler.concurrency;
        }
        if self.scheduler.priority.is_none() {
            self.scheduler.priority = defaults.scheduler.priority;
        }
        if self.scheduler.retry.is_none() {
            self.scheduler.retry = defaults.scheduler.retry;
        }
        if self.scheduler.timeout_seconds.is_none() {
            self.scheduler.timeout_seconds = defaults.scheduler.timeout_seconds;
        }

        if self.skills.enabled.is_empty() {
            if let Some(default_skills) = &defaults.skills {
                self.skills.enabled = default_skills.clone();
            }
        }
    }

    pub fn check_compatibility(&self, min_version: &str) -> Result<(), AgentConfigError> {
        let current = semver::Version::parse(&self.meta.version)?;
        let required = semver::Version::parse(min_version)?;

        if current < required {
            return Err(AgentConfigError::Validation(format!(
                "Config version {} is older than required {}",
                current, required
            )));
        }

        Ok(())
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
                agent_type: AgentType::Generic,
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
    fn test_agent_config_validation() {
        let temp_dir = tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let config = create_test_config(workspace);

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_version() {
        let temp_dir = tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let mut config = create_test_config(workspace);
        config.meta.version = "invalid-version".to_string();

        assert!(matches!(
            config.validate(),
            Err(AgentConfigError::Semver(_))
        ));
    }

    #[test]
    fn test_wecom_validation() {
        let temp_dir = tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let mut config = create_test_config(workspace);
        config.channels.wecom = Some(WeComConfig {
            enabled: true,
            corpid: "".to_string(),
            agentid: "agent-001".to_string(),
            secret: "secret-123".to_string(),
            token: None,
            encoding_aes_key: None,
            msg_format: None,
            rate_limit: None,
            webhook_url: None,
            webhook_secret: None,
        });

        assert!(matches!(
            config.validate(),
            Err(AgentConfigError::Validation(msg)) if msg.contains("corpid cannot be empty")
        ));
    }

    #[test]
    fn test_dingtalk_validation() {
        let temp_dir = tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let mut config = create_test_config(workspace);
        config.channels.dingtalk = Some(DingTalkConfig {
            enabled: true,
            app_key: "".to_string(),
            app_secret: "secret-123".to_string(),
            robot_code: None,
            workbench_notice: None,
            webhook_url: None,
            webhook_secret: None,
        });

        assert!(matches!(
            config.validate(),
            Err(AgentConfigError::Validation(msg)) if msg.contains("app_key cannot be empty")
        ));
    }

    #[test]
    fn test_feishu_validation() {
        let temp_dir = tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let mut config = create_test_config(workspace);
        config.channels.feishu = Some(FeishuConfig {
            enabled: true,
            app_id: "".to_string(),
            app_secret: "secret-123".to_string(),
            document_link: None,
            rich_text: None,
            webhook_url: None,
            webhook_secret: None,
        });

        assert!(matches!(
            config.validate(),
            Err(AgentConfigError::Validation(msg)) if msg.contains("app_id cannot be empty")
        ));
    }

    #[test]
    fn test_wechat_validation() {
        let temp_dir = tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let mut config = create_test_config(workspace);
        config.channels.wechat = Some(WeChatConfig {
            enabled: true,
            ilink_enabled: true,
            ilink_server: None,
            ilink_token: Some("token-123".to_string()),
            webhook_url: None,
            poll_interval_seconds: None,
        });

        assert!(matches!(
            config.validate(),
            Err(AgentConfigError::Validation(msg)) if msg.contains("ilink_server cannot be empty")
        ));
    }

    #[test]
    fn test_merge_with_defaults() {
        let temp_dir = tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let mut config = create_test_config(workspace);
        config.model.params.temperature = None;
        config.model.params.max_tokens = None;
        config.model.params.top_p = None;
        config.model.params.top_k = None;
        config.model.params.presence_penalty = None;
        config.model.params.frequency_penalty = None;
        config.model.params.timeout_seconds = None;
        config.memory.short_term = None;
        config.memory.mid_term = None;
        config.memory.long_term = None;
        config.memory.versioned_state = None;
        config.security.sandbox = None;
        config.security.rule_block = None;
        config.security.audit = None;
        config.security.human_intervene = None;
        config.scheduler.concurrency = None;
        config.scheduler.priority = None;
        config.scheduler.retry = None;
        config.scheduler.timeout_seconds = None;

        let defaults = AgentDefaults {
            model: ModelConfig {
                primary: "gpt-4o".to_string(),
                fallback: Some(vec!["gpt-4".to_string()]),
                params: ModelParams {
                    temperature: Some(0.8),
                    max_tokens: Some(2048),
                    top_p: Some(0.9),
                    top_k: Some(50),
                    presence_penalty: Some(0.1),
                    frequency_penalty: Some(0.1),
                    timeout_seconds: Some(60),
                },
            },
            memory: MemoryConfig {
                short_term: Some(ShortTermMemoryConfig {
                    capacity: Some(100),
                    ttl_seconds: Some(3600),
                }),
                mid_term: Some(MidTermMemoryConfig {
                    enable: Some(true),
                    persist: Some(true),
                }),
                long_term: Some(LongTermMemoryConfig {
                    enable: Some(true),
                    vector_db: Some("chroma".to_string()),
                }),
                versioned_state: Some(true),
            },
            security: SecurityConfig {
                sandbox: Some(SandboxConfig {
                    mode: SandboxMode::Isolated,
                    image: None,
                    network: None,
                    mount_paths: None,
                    env: None,
                }),
                rule_block: Some(true),
                audit: Some(true),
                human_intervene: Some(HumanInterveneConfig {
                    enable: false,
                    channel: None,
                }),
            },
            scheduler: SchedulerConfig {
                concurrency: Some(10),
                priority: Some(5),
                retry: Some(3),
                timeout_seconds: Some(300),
            },
            skills: Some(vec!["skill-1".to_string(), "skill-2".to_string()]),
        };

        config.merge_with_defaults(&defaults);

        assert_eq!(config.model.params.temperature, Some(0.8));
        assert_eq!(config.model.params.max_tokens, Some(2048));
        assert_eq!(config.model.params.top_p, Some(0.9));
        assert_eq!(config.model.params.top_k, Some(50));
        assert_eq!(config.model.params.presence_penalty, Some(0.1));
        assert_eq!(config.model.params.frequency_penalty, Some(0.1));
        assert_eq!(config.model.params.timeout_seconds, Some(60));

        assert!(config.memory.short_term.is_some());
        assert!(config.memory.mid_term.is_some());
        assert!(config.memory.long_term.is_some());
        assert_eq!(config.memory.versioned_state, Some(true));

        assert!(config.security.sandbox.is_some());
        assert_eq!(config.security.rule_block, Some(true));
        assert_eq!(config.security.audit, Some(true));
        assert!(config.security.human_intervene.is_some());

        assert_eq!(config.scheduler.concurrency, Some(10));
        assert_eq!(config.scheduler.priority, Some(5));
        assert_eq!(config.scheduler.retry, Some(3));
        assert_eq!(config.scheduler.timeout_seconds, Some(300));

        assert_eq!(
            config.skills.enabled,
            vec!["skill-1".to_string(), "skill-2".to_string()]
        );
    }

    #[test]
    fn test_check_compatibility() {
        let temp_dir = tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let config = create_test_config(workspace);

        assert!(config.check_compatibility("0.9.0").is_ok());
        assert!(config.check_compatibility("1.0.0").is_ok());
        assert!(matches!(
            config.check_compatibility("2.0.0"),
            Err(AgentConfigError::Validation(_))
        ));

        assert!(matches!(
            config.check_compatibility("invalid-version"),
            Err(AgentConfigError::Semver(_))
        ));
    }
}
