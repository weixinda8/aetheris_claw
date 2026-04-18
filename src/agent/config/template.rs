use crate::agent::config::config::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Template render error: {0}")]
    Render(String),
    #[error("Template parse error: {0}")]
    Parse(String),
    #[error("Template not found: {0}")]
    NotFound(String),
    #[error("Agent config error: {0}")]
    Config(#[from] AgentConfigError),
    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub agent_type: crate::agent::base::AgentType,
    pub template_content: String,
    pub variables: Vec<TemplateVariable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
    pub var_type: VariableType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VariableType {
    String,
    Number,
    Boolean,
    Path,
}

lazy_static::lazy_static! {
    static ref TEMPLATE_VAR_REGEX: Regex = Regex::new(r"\{\{vars\.([a-zA-Z0-9_]+)\}\}").unwrap();
}

pub struct AgentTemplateEngine {
    templates: HashMap<String, AgentTemplate>,
}

impl AgentTemplateEngine {
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        for template in create_default_templates() {
            templates.insert(template.id.clone(), template);
        }

        Self { templates }
    }

    pub fn register_template(&mut self, template: AgentTemplate) -> Result<(), TemplateError> {
        self.templates.insert(template.id.clone(), template);
        Ok(())
    }

    pub fn get_template(&self, template_id: &str) -> Option<&AgentTemplate> {
        self.templates.get(template_id)
    }

    pub fn list_templates(&self) -> Vec<&AgentTemplate> {
        self.templates.values().collect()
    }

    pub fn render(
        &self,
        template_id: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String, TemplateError> {
        let template = self
            .templates
            .get(template_id)
            .ok_or_else(|| TemplateError::NotFound(template_id.to_string()))?;

        for var in &template.variables {
            if var.required && !variables.contains_key(&var.name) {
                return Err(TemplateError::Render(format!(
                    "Required variable '{}' not provided",
                    var.name
                )));
            }
        }

        let mut result = template.template_content.clone();

        for cap in TEMPLATE_VAR_REGEX.captures_iter(&template.template_content) {
            if let Some(var_name) = cap.get(1) {
                let var_name_str = var_name.as_str();
                if let Some(var_value) = variables.get(var_name_str) {
                    result = result.replace(&cap[0], var_value);
                } else if let Some(default_value) = template
                    .variables
                    .iter()
                    .find(|v| v.name == var_name_str)
                    .and_then(|v| v.default.as_ref())
                {
                    result = result.replace(&cap[0], default_value);
                }
            }
        }

        Ok(result)
    }

    pub fn render_to_config(
        &self,
        template_id: &str,
        variables: &HashMap<String, String>,
        workspace: PathBuf,
    ) -> Result<AgentConfig, TemplateError> {
        let yaml_content = self.render(template_id, variables)?;
        let mut config: AgentConfig = serde_yaml::from_str(&yaml_content)?;
        config.meta.workspace = workspace;
        config.validate()?;
        Ok(config)
    }

    pub fn load_template_from_file<P: Into<PathBuf>>(
        &mut self,
        path: P,
    ) -> Result<AgentTemplate, TemplateError> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)?;
        let template: AgentTemplate = serde_yaml::from_str(&content)?;
        self.register_template(template.clone())?;
        Ok(template)
    }

    pub fn load_templates_from_directory<P: Into<PathBuf>>(
        &mut self,
        dir: P,
    ) -> Result<Vec<AgentTemplate>, TemplateError> {
        let dir = dir.into();
        let mut templates = Vec::new();

        if !dir.is_dir() {
            return Ok(templates);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "yaml" || ext == "yml" {
                        if let Ok(template) = self.load_template_from_file(&path) {
                            templates.push(template);
                        }
                    }
                }
            }
        }

        Ok(templates)
    }
}

impl Default for AgentTemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_default_templates() -> Vec<AgentTemplate> {
    vec![
        create_code_agent_template(),
        create_office_agent_template(),
        create_data_agent_template(),
        create_ops_agent_template(),
    ]
}

fn create_code_agent_template() -> AgentTemplate {
    AgentTemplate {
        id: "code_agent".to_string(),
        name: "Code Agent".to_string(),
        description: "A professional code development and review agent".to_string(),
        version: "1.0.0".to_string(),
        agent_type: crate::agent::base::AgentType::Code,
        template_content: r#"
meta:
  id: "{{vars.agent_id}}"
  name: "{{vars.agent_name}}"
  version: "1.0.0"
  type: code
  enabled: true
  hot_reload: true

persona:
  system_prompt: |
    You are a professional code development and review assistant.
    You help with writing, reviewing, and optimizing code.

model:
  primary: "{{vars.primary_model}}"
  fallback:
    - "gpt-4"
    - "gpt-3.5-turbo"
  params:
    temperature: 0.7
    max_tokens: 4096

skills:
  enabled:
    - "code_writer"
    - "code_reviewer"
    - "code_optimizer"
"#
        .to_string(),
        variables: vec![
            TemplateVariable {
                name: "agent_id".to_string(),
                description: "Unique identifier for the agent".to_string(),
                required: true,
                default: None,
                var_type: VariableType::String,
            },
            TemplateVariable {
                name: "agent_name".to_string(),
                description: "Display name of the agent".to_string(),
                required: true,
                default: Some("Code Assistant".to_string()),
                var_type: VariableType::String,
            },
            TemplateVariable {
                name: "primary_model".to_string(),
                description: "Primary LLM model to use".to_string(),
                required: true,
                default: Some("gpt-4o".to_string()),
                var_type: VariableType::String,
            },
        ],
    }
}

fn create_office_agent_template() -> AgentTemplate {
    AgentTemplate {
        id: "office_agent".to_string(),
        name: "Office Agent".to_string(),
        description: "An office automation and productivity agent".to_string(),
        version: "1.0.0".to_string(),
        agent_type: crate::agent::base::AgentType::Office,
        template_content: r#"
meta:
  id: "{{vars.agent_id}}"
  name: "{{vars.agent_name}}"
  version: "1.0.0"
  type: office
  enabled: true
  hot_reload: true

persona:
  system_prompt: |
    You are an office automation and productivity assistant.
    You help with document processing, scheduling, and administrative tasks.

model:
  primary: "{{vars.primary_model}}"
  fallback:
    - "gpt-4"
    - "gpt-3.5-turbo"
  params:
    temperature: 0.7
    max_tokens: 4096

skills:
  enabled:
    - "document_processor"
    - "scheduler"
    - "email_writer"
"#
        .to_string(),
        variables: vec![
            TemplateVariable {
                name: "agent_id".to_string(),
                description: "Unique identifier for the agent".to_string(),
                required: true,
                default: None,
                var_type: VariableType::String,
            },
            TemplateVariable {
                name: "agent_name".to_string(),
                description: "Display name of the agent".to_string(),
                required: true,
                default: Some("Office Assistant".to_string()),
                var_type: VariableType::String,
            },
            TemplateVariable {
                name: "primary_model".to_string(),
                description: "Primary LLM model to use".to_string(),
                required: true,
                default: Some("gpt-4o".to_string()),
                var_type: VariableType::String,
            },
        ],
    }
}

fn create_data_agent_template() -> AgentTemplate {
    AgentTemplate {
        id: "data_agent".to_string(),
        name: "Data Agent".to_string(),
        description: "A data analysis and processing agent".to_string(),
        version: "1.0.0".to_string(),
        agent_type: crate::agent::base::AgentType::Data,
        template_content: r#"
meta:
  id: "{{vars.agent_id}}"
  name: "{{vars.agent_name}}"
  version: "1.0.0"
  type: data
  enabled: true
  hot_reload: true

persona:
  system_prompt: |
    You are a data analysis and processing assistant.
    You help with data cleaning, analysis, visualization, and reporting.

model:
  primary: "{{vars.primary_model}}"
  fallback:
    - "gpt-4"
    - "gpt-3.5-turbo"
  params:
    temperature: 0.7
    max_tokens: 4096

skills:
  enabled:
    - "data_cleaner"
    - "data_analyzer"
    - "visualizer"
"#
        .to_string(),
        variables: vec![
            TemplateVariable {
                name: "agent_id".to_string(),
                description: "Unique identifier for the agent".to_string(),
                required: true,
                default: None,
                var_type: VariableType::String,
            },
            TemplateVariable {
                name: "agent_name".to_string(),
                description: "Display name of the agent".to_string(),
                required: true,
                default: Some("Data Assistant".to_string()),
                var_type: VariableType::String,
            },
            TemplateVariable {
                name: "primary_model".to_string(),
                description: "Primary LLM model to use".to_string(),
                required: true,
                default: Some("gpt-4o".to_string()),
                var_type: VariableType::String,
            },
        ],
    }
}

fn create_ops_agent_template() -> AgentTemplate {
    AgentTemplate {
        id: "ops_agent".to_string(),
        name: "Ops Agent".to_string(),
        description: "An operations and DevOps automation agent".to_string(),
        version: "1.0.0".to_string(),
        agent_type: crate::agent::base::AgentType::Ops,
        template_content: r#"
meta:
  id: "{{vars.agent_id}}"
  name: "{{vars.agent_name}}"
  version: "1.0.0"
  type: ops
  enabled: true
  hot_reload: true

persona:
  system_prompt: |
    You are an operations and DevOps automation assistant.
    You help with infrastructure management, deployment, monitoring, and troubleshooting.

model:
  primary: "{{vars.primary_model}}"
  fallback:
    - "gpt-4"
    - "gpt-3.5-turbo"
  params:
    temperature: 0.7
    max_tokens: 4096

skills:
  enabled:
    - "deployer"
    - "monitor"
    - "troubleshooter"
"#
        .to_string(),
        variables: vec![
            TemplateVariable {
                name: "agent_id".to_string(),
                description: "Unique identifier for the agent".to_string(),
                required: true,
                default: None,
                var_type: VariableType::String,
            },
            TemplateVariable {
                name: "agent_name".to_string(),
                description: "Display name of the agent".to_string(),
                required: true,
                default: Some("Ops Assistant".to_string()),
                var_type: VariableType::String,
            },
            TemplateVariable {
                name: "primary_model".to_string(),
                description: "Primary LLM model to use".to_string(),
                required: true,
                default: Some("gpt-4o".to_string()),
                var_type: VariableType::String,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_engine_creation() {
        let engine = AgentTemplateEngine::new();
        assert_eq!(engine.list_templates().len(), 4);
    }

    #[test]
    fn test_register_template() {
        let mut engine = AgentTemplateEngine::new();
        let template = AgentTemplate {
            id: "test_template".to_string(),
            name: "Test Template".to_string(),
            description: "A test template".to_string(),
            version: "1.0.0".to_string(),
            agent_type: crate::agent::base::AgentType::Generic,
            template_content: "name: {{vars.agent_name}}".to_string(),
            variables: vec![TemplateVariable {
                name: "agent_name".to_string(),
                description: "Agent name".to_string(),
                required: true,
                default: Some("Test Agent".to_string()),
                var_type: VariableType::String,
            }],
        };

        assert!(engine.register_template(template).is_ok());
        assert_eq!(engine.list_templates().len(), 5);
    }

    #[test]
    fn test_render_template() {
        let mut engine = AgentTemplateEngine::new();
        let template = AgentTemplate {
            id: "test_render".to_string(),
            name: "Test Render".to_string(),
            description: "Test render template".to_string(),
            version: "1.0.0".to_string(),
            agent_type: crate::agent::base::AgentType::Generic,
            template_content: "id: {{vars.agent_id}}\nname: {{vars.agent_name}}".to_string(),
            variables: vec![
                TemplateVariable {
                    name: "agent_id".to_string(),
                    description: "Agent ID".to_string(),
                    required: true,
                    default: None,
                    var_type: VariableType::String,
                },
                TemplateVariable {
                    name: "agent_name".to_string(),
                    description: "Agent name".to_string(),
                    required: true,
                    default: Some("Default".to_string()),
                    var_type: VariableType::String,
                },
            ],
        };

        engine.register_template(template).unwrap();

        let mut vars = HashMap::new();
        vars.insert("agent_id".to_string(), "test-001".to_string());
        vars.insert("agent_name".to_string(), "My Agent".to_string());

        let result = engine.render("test_render", &vars).unwrap();
        assert!(result.contains("test-001"));
        assert!(result.contains("My Agent"));
    }

    #[test]
    fn test_missing_required_variable() {
        let mut engine = AgentTemplateEngine::new();
        let template = AgentTemplate {
            id: "test_missing".to_string(),
            name: "Test Missing".to_string(),
            description: "Test missing variable".to_string(),
            version: "1.0.0".to_string(),
            agent_type: crate::agent::base::AgentType::Generic,
            template_content: "name: {{vars.agent_name}}".to_string(),
            variables: vec![TemplateVariable {
                name: "agent_name".to_string(),
                description: "Agent name".to_string(),
                required: true,
                default: None,
                var_type: VariableType::String,
            }],
        };

        engine.register_template(template).unwrap();

        let vars = HashMap::new();
        let result = engine.render("test_missing", &vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_default_templates() {
        let templates = create_default_templates();
        assert_eq!(templates.len(), 4);

        let template_ids: Vec<_> = templates.iter().map(|t| t.id.as_str()).collect();
        assert!(template_ids.contains(&"code_agent"));
        assert!(template_ids.contains(&"office_agent"));
        assert!(template_ids.contains(&"data_agent"));
        assert!(template_ids.contains(&"ops_agent"));
    }
}
