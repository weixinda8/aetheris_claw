use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OpenClawConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON5 parse error: {0}")]
    Parse(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Config not found: {0}")]
    NotFound(PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawModelConfig {
    pub model: String,
    pub provider: String,
    pub workspace: Option<String>,
    pub heartbeat: Option<OpenClawHeartbeatConfig>,
    pub compaction: Option<OpenClawCompactionConfig>,
    pub memory_search: Option<OpenClawMemorySearchConfig>,
    pub allowed_models: Option<Vec<String>>,
    pub model_fallbacks: Option<HashMap<String, Vec<String>>>,
    pub typing_interval_seconds: Option<f64>,
    pub envelope_timezone: Option<String>,
    pub sandbox: Option<OpenClawSandboxConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawHeartbeatConfig {
    pub every: String,
    pub light_context: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawCompactionConfig {
    pub threshold: f64,
    pub strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawMemorySearchConfig {
    pub enabled: bool,
    pub citations: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawSandboxConfig {
    pub enabled: bool,
    pub image: Option<String>,
    pub mount_paths: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub network: Option<String>,
    pub setup_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawAgent {
    pub agent_id: String,
    pub name: String,
    pub model: Option<String>,
    pub skills: Option<Vec<String>>,
    pub tools: Option<Vec<String>>,
    pub workspace: Option<String>,
    pub identity: Option<String>,
    pub sandbox: Option<OpenClawSandboxConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawBindings {
    pub agent_id: String,
    pub channel: String,
    pub account: Option<String>,
    pub peer: Option<String>,
    pub guild: Option<String>,
    pub roles: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawSessionConfig {
    pub dm_scope: Option<String>,
    pub identity_links: Option<bool>,
    pub reset_triggers: Option<Vec<String>>,
    pub send_policy: Option<String>,
    pub store: Option<String>,
    pub main_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawGatewayAuth {
    pub mode: String,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawGatewayTls {
    pub enabled: Option<bool>,
    pub cert: Option<String>,
    pub key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawGatewayConfig {
    pub port: u16,
    pub auth: Option<OpenClawGatewayAuth>,
    pub tls: Option<OpenClawGatewayTls>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawConfig {
    pub agents: OpenClawAgentsConfig,
    pub bindings: Vec<OpenClawBindings>,
    pub session: Option<OpenClawSessionConfig>,
    pub gateway: Option<OpenClawGatewayConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawAgentsConfig {
    pub defaults: OpenClawModelConfig,
    pub list: Vec<OpenClawAgent>,
}

impl OpenClawConfig {
    pub fn from_path(path: PathBuf) -> Result<Self, OpenClawConfigError> {
        if !path.exists() {
            return Err(OpenClawConfigError::NotFound(path));
        }

        let content = std::fs::read_to_string(&path)?;
        content.parse()
    }

    fn validate(config: &OpenClawConfig) -> Result<(), OpenClawConfigError> {
        if config.agents.list.is_empty() {
            return Err(OpenClawConfigError::Validation(
                "At least one agent must be defined".to_string(),
            ));
        }

        let mut agent_ids = std::collections::HashSet::new();
        for agent in &config.agents.list {
            if agent.agent_id.is_empty() {
                return Err(OpenClawConfigError::Validation(
                    "Agent ID cannot be empty".to_string(),
                ));
            }
            if agent.name.is_empty() {
                return Err(OpenClawConfigError::Validation(
                    "Agent name cannot be empty".to_string(),
                ));
            }
            if !agent_ids.insert(agent.agent_id.clone()) {
                return Err(OpenClawConfigError::Validation(format!(
                    "Duplicate agent ID: {}",
                    agent.agent_id
                )));
            }
        }

        for binding in &config.bindings {
            if !agent_ids.contains(&binding.agent_id) {
                return Err(OpenClawConfigError::Validation(format!(
                    "Binding references unknown agent ID: {}",
                    binding.agent_id
                )));
            }
        }

        Ok(())
    }

    pub fn to_aetheris_json5(&self) -> String {
        let mut result = String::new();

        result.push_str("// Aetheris config (migrated from OpenClaw)\n");
        result.push_str("{\n");

        result.push_str("  // ============================================\n");
        result.push_str("  // OpenClaw 兼容配置\n");
        result.push_str("  // ============================================\n");
        result.push_str("  openclaw: {\n");
        result.push_str("    compatible: true,\n");
        result.push_str("    migrated_from: \"openclaw.json\",\n");
        result.push_str("  },\n\n");

        result.push_str("  // ============================================\n");
        result.push_str("  // Agents 配置\n");
        result.push_str("  // ============================================\n");
        result.push_str("  agents: {\n");

        result.push_str("    defaults: {\n");
        result.push_str(&format!(
            "      model: \"{}\",\n",
            self.agents.defaults.model
        ));
        result.push_str(&format!(
            "      provider: \"{}\",\n",
            self.agents.defaults.provider
        ));
        if let Some(workspace) = &self.agents.defaults.workspace {
            result.push_str(&format!("      workspace: \"{}\",\n", workspace));
        }
        result.push_str("    },\n");

        result.push_str("    list: [\n");
        for agent in &self.agents.list {
            result.push_str("      {\n");
            result.push_str(&format!("        agent_id: \"{}\",\n", agent.agent_id));
            result.push_str(&format!("        name: \"{}\",\n", agent.name));
            if let Some(model) = &agent.model {
                result.push_str(&format!("        model: \"{}\",\n", model));
            }
            if let Some(skills) = &agent.skills {
                result.push_str("        skills: [\n");
                for skill in skills {
                    result.push_str(&format!("          \"{}\",\n", skill));
                }
                result.push_str("        ],\n");
            }
            if let Some(tools) = &agent.tools {
                result.push_str("        tools: [\n");
                for tool in tools {
                    result.push_str(&format!("          \"{}\",\n", tool));
                }
                result.push_str("        ],\n");
            }
            if let Some(workspace) = &agent.workspace {
                result.push_str(&format!("        workspace: \"{}\",\n", workspace));
            }
            if let Some(identity) = &agent.identity {
                result.push_str(&format!("        identity: \"{}\",\n", identity));
            }
            result.push_str("      },\n");
        }
        result.push_str("    ],\n");
        result.push_str("  },\n\n");

        result.push_str("  // ============================================\n");
        result.push_str("  // 绑定配置\n");
        result.push_str("  // ============================================\n");
        result.push_str("  bindings: [\n");
        for binding in &self.bindings {
            result.push_str("    {\n");
            result.push_str(&format!("      agent_id: \"{}\",\n", binding.agent_id));
            result.push_str(&format!("      channel: \"{}\",\n", binding.channel));
            if let Some(account) = &binding.account {
                result.push_str(&format!("      account: \"{}\",\n", account));
            }
            if let Some(peer) = &binding.peer {
                result.push_str(&format!("      peer: \"{}\",\n", peer));
            }
            if let Some(guild) = &binding.guild {
                result.push_str(&format!("      guild: \"{}\",\n", guild));
            }
            if let Some(roles) = &binding.roles {
                result.push_str("      roles: [\n");
                for role in roles {
                    result.push_str(&format!("        \"{}\",\n", role));
                }
                result.push_str("      ],\n");
            }
            result.push_str("    },\n");
        }
        result.push_str("  ],\n");

        result.push_str("}\n");

        result
    }

    pub fn save_aetheris_config(&self, path: PathBuf) -> Result<(), OpenClawConfigError> {
        let content = self.to_aetheris_json5();
        std::fs::write(&path, content)?;
        Ok(())
    }
}

impl std::str::FromStr for OpenClawConfig {
    type Err = OpenClawConfigError;

    fn from_str(content: &str) -> std::result::Result<Self, Self::Err> {
        let config: OpenClawConfig = json5::from_str(content)
            .map_err(|e| OpenClawConfigError::Parse(format!("JSON5 parse error: {}", e)))?;

        Self::validate(&config)?;

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openclaw_config_parse() {
        let content = r#"{
  agents: {
    defaults: {
      model: "claude-3-opus",
      provider: "anthropic",
      workspace: "~/.openclaw/workspace"
    },
    list: [
      {
        agent_id: "code-assistant",
        name: "代码助手",
        model: "gpt-4o",
        skills: ["git", "python"],
        tools: ["read", "write"]
      }
    ]
  },
  bindings: [
    {
      agent_id: "code-assistant",
      channel: "telegram"
    }
  ]
}"#;

        let config: OpenClawConfig = content.parse().unwrap();

        assert_eq!(config.agents.defaults.model, "claude-3-opus");
        assert_eq!(config.agents.list.len(), 1);
        assert_eq!(config.agents.list[0].agent_id, "code-assistant");
        assert_eq!(config.bindings.len(), 1);
    }
}
