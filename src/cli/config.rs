use crate::agent::config::config::AgentConfigError;
use crate::agent::config::loader::AgentConfigLoader;
use crate::agent::config::template::{AgentTemplateEngine, create_default_templates};
use crate::config::onboard::OnboardWizard;
use crate::soul::Soul;
use crate::soul::enhanced_system::{
    EnhancedSoulSystem, EvolutionChangeType, EvolutionTrigger, PersonalityProfile, PersonalityType,
};
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Onboard error: {0}")]
    Onboard(#[from] crate::config::onboard::OnboardError),
    #[error("OpenClaw error: {0}")]
    OpenClaw(#[from] crate::config::openclaw::OpenClawConfigError),
    #[error("Soul error: {0}")]
    Soul(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Config not found")]
    ConfigNotFound,
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Agent config error: {0}")]
    AgentConfig(#[from] AgentConfigError),
    #[error("Agent template error: {0}")]
    AgentTemplate(String),
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("Template not found: {0}")]
    TemplateNotFound(String),
}

#[derive(Debug, Parser)]
#[command(
    name = "aetheris",
    about = "Aetheris 2.0 - Unified Digital Employee & Agent/Skill Ecosystem"
)]
pub struct AetherisCli {
    #[command(subcommand)]
    pub command: Option<AetherisCommand>,
}

#[derive(Debug, Subcommand)]
pub enum AetherisCommand {
    #[command(about = "Run the initial setup wizard")]
    Onboard,
    #[command(about = "Configure Aetheris settings")]
    Configure(ConfigureArgs),
    #[command(about = "Manage configuration")]
    Config {
        #[command(subcommand)]
        subcommand: ConfigSubcommand,
    },
    #[command(about = "Manage digital employee personalities")]
    Soul {
        #[command(subcommand)]
        subcommand: SoulSubcommand,
    },
    #[command(about = "Manage agents")]
    Agent {
        #[command(subcommand)]
        subcommand: AgentSubcommand,
    },
    #[command(about = "Run health check")]
    Doctor,
    #[command(about = "Run security audit")]
    SecurityAudit(SecurityAuditArgs),
}

#[derive(Debug, Clone, Parser)]
pub struct ConfigureArgs {
    #[arg(long, short = 'H')]
    pub server_host: Option<String>,
    #[arg(long, short = 'P')]
    pub server_port: Option<u16>,
    #[arg(long)]
    pub llm_provider: Option<String>,
    #[arg(long)]
    pub llm_model: Option<String>,
    #[arg(long)]
    pub llm_api_key: Option<String>,
    #[arg(long)]
    pub llm_temperature: Option<f32>,
    #[arg(long)]
    pub llm_max_tokens: Option<u32>,
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommand {
    #[command(about = "Get a configuration value")]
    Get(GetArgs),
    #[command(about = "Set a configuration value")]
    Set(SetArgs),
    #[command(about = "Unset a configuration value")]
    Unset(UnsetArgs),
    #[command(about = "List all configuration values")]
    List,
    #[command(about = "Import configuration from a file")]
    Import(ImportArgs),
    #[command(about = "Export configuration to a file")]
    Export(ExportArgs),
}

#[derive(Debug, Clone, Parser)]
pub struct GetArgs {
    pub key: String,
}

#[derive(Debug, Clone, Parser)]
pub struct SetArgs {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Parser)]
pub struct UnsetArgs {
    pub key: String,
}

#[derive(Debug, Clone, Parser)]
pub struct ImportArgs {
    pub path: PathBuf,
    #[arg(long, short)]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Parser)]
pub struct ExportArgs {
    pub path: PathBuf,
    #[arg(long, short)]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Parser)]
pub struct SecurityAuditArgs {
    #[arg(long, short)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub full: bool,
}

#[derive(Debug, Subcommand)]
pub enum SoulSubcommand {
    #[command(about = "Switch current personality")]
    Switch(SwitchArgs),
    #[command(about = "Show current personality")]
    Current(CurrentArgs),
    #[command(about = "List all available personalities")]
    List(ListArgs),
    #[command(about = "Create a new personality")]
    Create(CreateArgs),
    #[command(about = "Edit an existing personality")]
    Edit(EditArgs),
    #[command(about = "Import a personality")]
    Import(ImportSoulArgs),
    #[command(about = "Export a personality")]
    Export(ExportSoulArgs),
    #[command(about = "Optimize a personality based on usage")]
    Optimize(OptimizeArgs),
    #[command(about = "Show personality evolution history")]
    History(HistoryArgs),
    #[command(about = "Rate a personality")]
    Rate(RateArgs),
    #[command(about = "Publish personality to marketplace")]
    Publish(PublishArgs),
}

#[derive(Debug, Clone, Parser)]
pub struct SwitchArgs {
    pub personality_id: String,
    #[arg(long, short)]
    pub user: Option<String>,
}

#[derive(Debug, Clone, Parser)]
pub struct CurrentArgs {
    #[arg(long, short)]
    pub user: Option<String>,
}

#[derive(Debug, Clone, Parser)]
pub struct ListArgs {
    #[arg(long)]
    pub personality_type: Option<String>,
    #[arg(long)]
    pub tag: Option<String>,
    #[arg(long)]
    pub author: Option<String>,
    #[arg(long)]
    pub popular: bool,
    #[arg(long)]
    pub top_rated: bool,
    #[arg(long, short)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Parser)]
pub struct CreateArgs {
    #[arg(long, short)]
    pub name: String,
    #[arg(long, short)]
    pub description: String,
    #[arg(long)]
    pub personality_type: Option<String>,
    #[arg(long)]
    pub author: Option<String>,
    #[arg(long, short)]
    pub tag: Vec<String>,
    #[arg(long)]
    pub from_template: Option<String>,
}

#[derive(Debug, Clone, Parser)]
pub struct EditArgs {
    pub personality_id: String,
    #[arg(long, short)]
    pub name: Option<String>,
    #[arg(long, short)]
    pub description: Option<String>,
    #[arg(long, short)]
    pub tag: Option<Vec<String>>,
}

#[derive(Debug, Clone, Parser)]
pub struct ImportSoulArgs {
    pub path: PathBuf,
    #[arg(long, short)]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Parser)]
pub struct ExportSoulArgs {
    pub personality_id: String,
    pub path: PathBuf,
    #[arg(long, short)]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Parser)]
pub struct OptimizeArgs {
    pub personality_id: String,
    #[arg(long)]
    pub user: Option<String>,
    #[arg(long)]
    pub auto_apply: bool,
}

#[derive(Debug, Clone, Parser)]
pub struct HistoryArgs {
    pub personality_id: String,
    #[arg(long, short)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Parser)]
pub struct RateArgs {
    pub personality_id: String,
    pub rating: u8,
    #[arg(long, short)]
    pub comment: Option<String>,
    #[arg(long, short)]
    pub tag: Vec<String>,
}

#[derive(Debug, Clone, Parser)]
pub struct PublishArgs {
    pub personality_id: String,
    #[arg(long)]
    pub marketplace: Option<String>,
    #[arg(long)]
    pub price: Option<f64>,
    #[arg(long)]
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Parser)]
pub struct AgentListArgs {
    #[arg(long, short)]
    pub dir: Option<PathBuf>,
    #[arg(long, short)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Parser)]
pub struct AgentCreateArgs {
    pub config_path: PathBuf,
    #[arg(long, short)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, Parser)]
pub struct AgentTemplateArgs {
    pub template_id: String,
    #[arg(long, short)]
    pub name: Option<String>,
    #[arg(long, short)]
    pub description: Option<String>,
    #[arg(long, short)]
    pub var: Vec<String>,
    #[arg(long, short)]
    pub workspace: Option<PathBuf>,
    #[arg(long, short)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, Parser)]
pub struct AgentShowArgs {
    pub config_path: PathBuf,
    #[arg(long, short)]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Parser)]
pub struct AgentValidateArgs {
    pub config_path: PathBuf,
}

#[derive(Debug, Clone, Parser)]
pub struct AgentTemplatesArgs {
    #[arg(long, short)]
    pub details: bool,
}

#[derive(Debug, Clone, Parser)]
pub struct AgentExportArgs {
    pub config_path: PathBuf,
    pub output_path: PathBuf,
    #[arg(long, short)]
    pub format: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum AgentSubcommand {
    #[command(about = "List all available agents")]
    List(AgentListArgs),
    #[command(about = "Create a new agent from config file")]
    Create(AgentCreateArgs),
    #[command(about = "Create a new agent from template")]
    Template(AgentTemplateArgs),
    #[command(about = "Show agent details")]
    Show(AgentShowArgs),
    #[command(about = "Validate agent configuration")]
    Validate(AgentValidateArgs),
    #[command(about = "List available agent templates")]
    Templates(AgentTemplatesArgs),
    #[command(about = "Export agent configuration")]
    Export(AgentExportArgs),
}

pub struct ConfigManager {
    config_dir: PathBuf,
}

impl ConfigManager {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    pub fn new_default() -> Result<Self, CliConfigError> {
        let config_dir = dirs::home_dir()
            .ok_or_else(|| CliConfigError::Validation("Could not find home directory".to_string()))?
            .join(".aetheris");

        Ok(Self::new(config_dir))
    }

    pub async fn run_onboard(&self) -> Result<(), CliConfigError> {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║           Welcome to Aetheris 2.0 Setup Wizard                ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();

        let mut wizard = OnboardWizard::new(self.config_dir.clone())?;

        println!(
            "Step 1: {}",
            wizard.progress().get_step("welcome").unwrap().title
        );
        println!(
            "{}",
            wizard.progress().get_step("welcome").unwrap().description
        );
        println!();

        wizard.progress_mut().complete_step("welcome")?;

        if let Some(openclaw_path) = wizard.detect_openclaw_config() {
            println!(
                "Step 2: {}",
                wizard.progress().get_step("detect_openclaw").unwrap().title
            );
            println!(
                "{}",
                wizard
                    .progress()
                    .get_step("detect_openclaw")
                    .unwrap()
                    .description
            );
            println!();
            println!("OpenClaw configuration detected at: {:?}", openclaw_path);
            println!();

            println!("Would you like to migrate your OpenClaw configuration? [Y/n]");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();

            if input.is_empty() || input == "y" || input == "yes" {
                println!("Migrating OpenClaw configuration...");
                let aetheris_path = wizard.migrate_openclaw_config(openclaw_path).await?;
                println!("Configuration migrated to: {:?}", aetheris_path);
            } else {
                println!("Skipping OpenClaw migration.");
            }
        }

        wizard.progress_mut().complete_step("detect_openclaw")?;

        let config_path = self.config_dir.join("aetheris.json5");
        if !config_path.exists() {
            println!();
            println!(
                "Step 3: {}",
                wizard.progress().get_step("create_config").unwrap().title
            );
            println!(
                "{}",
                wizard
                    .progress()
                    .get_step("create_config")
                    .unwrap()
                    .description
            );
            println!();

            println!("Creating default configuration...");
            let config_path = wizard.create_default_config()?;
            println!("Default configuration created at: {:?}", config_path);
        }

        wizard.progress_mut().complete_step("create_config")?;

        println!();
        println!(
            "Step 4: {}",
            wizard.progress().get_step("setup_soul").unwrap().title
        );
        println!(
            "{}",
            wizard
                .progress()
                .get_step("setup_soul")
                .unwrap()
                .description
        );
        println!();

        println!("Would you like to set up a default digital employee? [Y/n]");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input.is_empty() || input == "y" || input == "yes" {
            println!("Setting up default digital employee...");
            let soul_path = wizard.setup_default_soul()?;
            println!("Default soul created at: {:?}", soul_path);
        } else {
            println!("Skipping default soul setup.");
        }

        wizard.progress_mut().complete_step("setup_soul")?;

        println!();
        println!(
            "Step 5: {}",
            wizard.progress().get_step("complete").unwrap().title
        );
        println!(
            "{}",
            wizard.progress().get_step("complete").unwrap().description
        );
        println!();

        wizard.complete_onboard()?;

        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║             Setup Complete! Enjoy Aetheris 2.0!               ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
        println!("Next steps:");
        println!("  1. Run 'aetheris doctor' to check your setup");
        println!("  2. Run 'aetheris configure' to customize your settings");
        println!("  3. Check out the documentation at https://docs.aetheris.io");
        println!();

        Ok(())
    }

    pub fn configure(&self, args: ConfigureArgs) -> Result<(), CliConfigError> {
        println!("Configuring Aetheris...");
        let config_path = self.config_dir.join("aetheris.json5");

        if !config_path.exists() {
            return Err(CliConfigError::ConfigNotFound);
        }

        let mut config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            json5::from_str::<serde_json::Value>(&content)
                .map_err(|e| CliConfigError::Validation(format!("Invalid config: {}", e)))?
        } else {
            serde_json::json!({})
        };

        if let Some(host) = args.server_host {
            config["server"]["host"] = serde_json::Value::String(host);
        }
        if let Some(port) = args.server_port {
            config["server"]["port"] = serde_json::Value::Number(port.into());
        }
        if let Some(provider) = args.llm_provider {
            config["llm"]["provider"] = serde_json::Value::String(provider);
        }
        if let Some(model) = args.llm_model {
            config["llm"]["model"] = serde_json::Value::String(model);
        }
        if let Some(api_key) = args.llm_api_key {
            config["llm"]["api_key"] = serde_json::Value::String(api_key);
        }
        if let Some(temp) = args.llm_temperature {
            config["llm"]["temperature"] =
                serde_json::Value::Number(serde_json::Number::from_f64(temp as f64).unwrap());
        }
        if let Some(max_tokens) = args.llm_max_tokens {
            config["llm"]["max_tokens"] = serde_json::Value::Number(max_tokens.into());
        }

        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(&config_path, content)?;

        println!("Configuration updated successfully!");
        Ok(())
    }

    pub fn config_get(&self, args: GetArgs) -> Result<(), CliConfigError> {
        let config_path = self.config_dir.join("aetheris.json5");

        if !config_path.exists() {
            return Err(CliConfigError::ConfigNotFound);
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: serde_json::Value = json5::from_str(&content)
            .map_err(|e| CliConfigError::Validation(format!("Invalid config: {}", e)))?;

        let keys: Vec<&str> = args.key.split('.').collect();
        let mut current = &config;

        for key in keys {
            current = current.get(key).ok_or_else(|| {
                CliConfigError::Validation(format!("Key not found: {}", args.key))
            })?;
        }

        println!("{}", current);
        Ok(())
    }

    pub fn config_set(&self, args: SetArgs) -> Result<(), CliConfigError> {
        let config_path = self.config_dir.join("aetheris.json5");

        if !config_path.exists() {
            return Err(CliConfigError::ConfigNotFound);
        }

        let content = std::fs::read_to_string(&config_path)?;
        let mut config: serde_json::Value = json5::from_str(&content)
            .map_err(|e| CliConfigError::Validation(format!("Invalid config: {}", e)))?;

        let keys: Vec<&str> = args.key.split('.').collect();
        let mut current = &mut config;

        for (i, key) in keys.iter().enumerate() {
            if i == keys.len() - 1 {
                current[key] = serde_json::Value::String(args.value.clone());
            } else {
                if current.get(key).is_none() {
                    current[key] = serde_json::json!({});
                }
                current = current.get_mut(key).unwrap();
            }
        }

        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(&config_path, content)?;

        println!("Set {} = {}", args.key, args.value);
        Ok(())
    }

    pub fn config_unset(&self, args: UnsetArgs) -> Result<(), CliConfigError> {
        let config_path = self.config_dir.join("aetheris.json5");

        if !config_path.exists() {
            return Err(CliConfigError::ConfigNotFound);
        }

        let content = std::fs::read_to_string(&config_path)?;
        let mut config: serde_json::Value = json5::from_str(&content)
            .map_err(|e| CliConfigError::Validation(format!("Invalid config: {}", e)))?;

        let keys: Vec<&str> = args.key.split('.').collect();
        let mut current = &mut config;

        for (i, key) in keys.iter().enumerate() {
            if i == keys.len() - 1 {
                if let Some(obj) = current.as_object_mut() {
                    obj.remove(*key);
                }
            } else {
                current = current.get_mut(key).ok_or_else(|| {
                    CliConfigError::Validation(format!("Key not found: {}", args.key))
                })?;
            }
        }

        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(&config_path, content)?;

        println!("Unset {}", args.key);
        Ok(())
    }

    pub fn config_list(&self) -> Result<(), CliConfigError> {
        let config_path = self.config_dir.join("aetheris.json5");

        if !config_path.exists() {
            return Err(CliConfigError::ConfigNotFound);
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: serde_json::Value = json5::from_str(&content)
            .map_err(|e| CliConfigError::Validation(format!("Invalid config: {}", e)))?;

        println!("{}", serde_json::to_string_pretty(&config)?);
        Ok(())
    }

    pub fn doctor(&self) -> Result<(), CliConfigError> {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║                    Aetheris Health Check                        ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();

        let mut all_ok = true;

        println!("[1/5] Checking config directory...");
        if self.config_dir.exists() {
            println!("  ✓ Config directory exists: {:?}", self.config_dir);
        } else {
            println!("  ✗ Config directory not found");
            all_ok = false;
        }

        println!();
        println!("[2/5] Checking configuration file...");
        let config_path = self.config_dir.join("aetheris.json5");
        if config_path.exists() {
            println!("  ✓ Configuration file exists");
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if json5::from_str::<serde_json::Value>(&content).is_ok() {
                    println!("  ✓ Configuration file is valid");
                } else {
                    println!("  ✗ Configuration file is invalid");
                    all_ok = false;
                }
            }
        } else {
            println!("  ✗ Configuration file not found");
            all_ok = false;
        }

        println!();
        println!("[3/5] Checking souls directory...");
        let souls_dir = self.config_dir.join("souls");
        if souls_dir.exists() {
            println!("  ✓ Souls directory exists");
        } else {
            println!("  ✗ Souls directory not found");
            all_ok = false;
        }

        println!();
        println!("[4/5] Checking onboard progress...");
        let onboard_path = self.config_dir.join("onboard-progress.json");
        if onboard_path.exists() {
            if let Ok(progress) = crate::config::onboard::OnboardProgress::load(onboard_path) {
                if progress.is_complete() {
                    println!("  ✓ Onboard complete");
                } else {
                    println!(
                        "  ⚠ Onboard in progress: {:.0}%",
                        progress.progress_percentage()
                    );
                }
            }
        } else {
            println!("  ⚠ Onboard not started");
        }

        println!();
        println!("[5/5] Summary...");
        if all_ok {
            println!("  ✓ All checks passed!");
        } else {
            println!("  ✗ Some checks failed");
            println!();
            println!("Next steps:");
            println!("  1. Run 'aetheris onboard' to complete setup");
            println!("  2. Check the documentation at https://docs.aetheris.io");
        }

        println!();
        Ok(())
    }

    pub fn security_audit(&self, args: SecurityAuditArgs) -> Result<(), CliConfigError> {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║                  Aetheris Security Audit                        ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();

        println!("Running security audit...");
        println!();

        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        let config_path = self.config_dir.join("aetheris.json5");
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if content.contains("aetheris-secret-key-change-in-production") {
                    issues.push("Default JWT secret detected - change in production!".to_string());
                }
                if content.contains("\"mock\"") {
                    warnings
                        .push("Mock LLM provider detected - not for production use".to_string());
                }
            }
        }

        println!("Issues found:");
        if issues.is_empty() {
            println!("  ✓ No critical issues found");
        } else {
            for issue in &issues {
                println!("  ✗ {}", issue);
            }
        }

        println!();
        println!("Warnings:");
        if warnings.is_empty() {
            println!("  ✓ No warnings found");
        } else {
            for warning in &warnings {
                println!("  ⚠ {}", warning);
            }
        }

        println!();
        if let Some(output_path) = args.output {
            let report = serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "issues": issues,
                "warnings": warnings,
                "full": args.full,
            });
            std::fs::write(&output_path, serde_json::to_string_pretty(&report)?)?;
            println!("Audit report saved to: {:?}", output_path);
        }

        Ok(())
    }

    pub fn config_import(&self, args: ImportArgs) -> Result<(), CliConfigError> {
        println!("Importing configuration from {:?}...", args.path);

        let content = std::fs::read_to_string(&args.path)?;
        let config: serde_json::Value = match args.format.as_deref() {
            Some("json5") | None => json5::from_str(&content)
                .map_err(|e| CliConfigError::Validation(format!("Invalid JSON5: {}", e)))?,
            Some("json") => serde_json::from_str(&content)
                .map_err(|e| CliConfigError::Validation(format!("Invalid JSON: {}", e)))?,
            Some("yaml") | Some("yml") => serde_yaml::from_str(&content)
                .map_err(|e| CliConfigError::Validation(format!("Invalid YAML: {}", e)))?,
            _ => {
                return Err(CliConfigError::Validation(
                    "Unsupported format. Use json5, json, or yaml.".to_string(),
                ));
            }
        };

        let output_path = self.config_dir.join("aetheris.json5");
        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(&output_path, content)?;

        println!("Configuration imported successfully!");
        Ok(())
    }

    pub fn config_export(&self, args: ExportArgs) -> Result<(), CliConfigError> {
        println!("Exporting configuration to {:?}...", args.path);

        let config_path = self.config_dir.join("aetheris.json5");
        if !config_path.exists() {
            return Err(CliConfigError::ConfigNotFound);
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: serde_json::Value = json5::from_str(&content)
            .map_err(|e| CliConfigError::Validation(format!("Invalid config: {}", e)))?;

        let output_content = match args.format.as_deref() {
            Some("json5") | None => content,
            Some("json") => serde_json::to_string_pretty(&config)?,
            Some("yaml") | Some("yml") => serde_yaml::to_string(&config)?,
            _ => {
                return Err(CliConfigError::Validation(
                    "Unsupported format. Use json5, json, or yaml.".to_string(),
                ));
            }
        };

        std::fs::write(&args.path, output_content)?;

        println!("Configuration exported successfully!");
        Ok(())
    }

    pub fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }

    fn get_enhanced_soul_system(&self) -> Result<EnhancedSoulSystem, CliConfigError> {
        let personalities_dir = self.config_dir.join("soul-personalities");
        EnhancedSoulSystem::new(personalities_dir)
            .map_err(|e| CliConfigError::Soul(format!("Failed to initialize soul system: {}", e)))
    }

    fn get_default_user_id() -> String {
        "default-user".to_string()
    }

    fn parse_personality_type(s: &str) -> PersonalityType {
        match s.to_lowercase().as_str() {
            "assistant" => PersonalityType::Assistant,
            "developer" => PersonalityType::Developer,
            "designer" => PersonalityType::Designer,
            "analyst" => PersonalityType::Analyst,
            "manager" => PersonalityType::Manager,
            "teacher" => PersonalityType::Teacher,
            "creative" => PersonalityType::Creative,
            "analytical" => PersonalityType::Analytical,
            s => PersonalityType::Custom(s.to_string()),
        }
    }

    pub fn soul_switch(&self, args: SwitchArgs) -> Result<(), CliConfigError> {
        let system = self.get_enhanced_soul_system()?;
        let user_id = args.user.unwrap_or_else(Self::get_default_user_id);

        system
            .switch_personality(&user_id, &args.personality_id)
            .map_err(|e| CliConfigError::Soul(format!("Failed to switch personality: {}", e)))?;

        println!("Switched to personality: {}", args.personality_id);
        Ok(())
    }

    pub fn soul_current(&self, args: CurrentArgs) -> Result<(), CliConfigError> {
        let system = self.get_enhanced_soul_system()?;
        let user_id = args.user.unwrap_or_else(Self::get_default_user_id);

        if let Some(personality) = system.get_active_personality(&user_id) {
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                    Current Personality                            ║");
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();
            println!("ID:          {}", personality.personality_id);
            println!("Name:        {}", personality.name);
            println!("Description: {}", personality.description);
            println!("Type:        {:?}", personality.personality_type);
            println!("Version:     {}", personality.version);
            println!("Author:      {}", personality.author);
            println!(
                "Rating:      {:.1}/5 ({} ratings)",
                personality.rating, personality.rating_count
            );
            println!("Downloads:   {}", personality.download_count);
        } else {
            println!("No active personality set.");
        }
        Ok(())
    }

    pub fn soul_list(&self, args: ListArgs) -> Result<(), CliConfigError> {
        let system = self.get_enhanced_soul_system()?;

        let personalities: Vec<PersonalityProfile> = if args.popular {
            system.get_popular_personalities(args.limit.unwrap_or(10))
        } else if args.top_rated {
            system.get_top_rated_personalities(args.limit.unwrap_or(10))
        } else if let Some(personality_type) = args.personality_type {
            let pt = Self::parse_personality_type(&personality_type);
            system.get_personalities_by_type(&pt)
        } else if let Some(tag) = args.tag {
            system.get_personalities_by_tag(&tag)
        } else if let Some(author) = args.author {
            system.get_personalities_by_author(&author)
        } else {
            system.list_personalities()
        };

        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║                   Available Personalities                        ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();

        if personalities.is_empty() {
            println!("No personalities found.");
        } else {
            for (i, p) in personalities.iter().enumerate() {
                println!("{}. {}", i + 1, p.name);
                println!("   ID:          {}", p.personality_id);
                println!("   Description: {}", p.description);
                println!("   Type:        {:?}", p.personality_type);
                println!(
                    "   Rating:      {:.1}/5 ({} ratings)",
                    p.rating, p.rating_count
                );
                println!("   Downloads:   {}", p.download_count);
                if !p.tags.is_empty() {
                    println!("   Tags:        {}", p.tags.join(", "));
                }
                println!();
            }
        }
        Ok(())
    }

    pub fn soul_create(&self, args: CreateArgs) -> Result<(), CliConfigError> {
        let system = self.get_enhanced_soul_system()?;
        let personality_id = format!(
            "{}-{}",
            args.name.to_lowercase().replace(" ", "-"),
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
        );

        let soul = Soul::default();
        let personality = PersonalityProfile {
            personality_id: personality_id.clone(),
            name: args.name,
            description: args.description,
            personality_type: args
                .personality_type
                .map(|s| Self::parse_personality_type(&s))
                .unwrap_or(PersonalityType::Assistant),
            version: "1.0.0".to_string(),
            author: args.author.unwrap_or_else(|| "anonymous".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            is_active: false,
            is_official: false,
            is_published: false,
            tags: args.tag,
            soul,
            personality_traits: std::collections::HashMap::new(),
            behavioral_patterns: std::collections::HashMap::new(),
            conversation_style: std::collections::HashMap::new(),
            knowledge_base: vec![],
            skill_preferences: std::collections::HashMap::new(),
            evolution_history: vec![],
            rating: 0.0,
            rating_count: 0,
            download_count: 0,
        };

        system
            .register_personality(personality)
            .map_err(|e| CliConfigError::Soul(format!("Failed to create personality: {}", e)))?;

        println!("Created personality: {}", personality_id);
        Ok(())
    }

    pub fn soul_edit(&self, args: EditArgs) -> Result<(), CliConfigError> {
        let system = self.get_enhanced_soul_system()?;

        if let Some(mut personality) = system.personalities.get_mut(&args.personality_id) {
            if let Some(name) = args.name {
                personality.name = name;
            }
            if let Some(description) = args.description {
                personality.description = description;
            }
            if let Some(tags) = args.tag {
                personality.tags = tags;
            }
            personality.updated_at = chrono::Utc::now();
            println!("Updated personality: {}", args.personality_id);
        } else {
            return Err(CliConfigError::Soul(format!(
                "Personality not found: {}",
                args.personality_id
            )));
        }
        Ok(())
    }

    pub fn soul_import(&self, args: ImportSoulArgs) -> Result<(), CliConfigError> {
        println!("Importing personality from {:?}...", args.path);

        let content = std::fs::read_to_string(&args.path)?;
        let personality: PersonalityProfile = match args.format.as_deref() {
            Some("json") | None => serde_json::from_str(&content)
                .map_err(|e| CliConfigError::Validation(format!("Invalid JSON: {}", e)))?,
            Some("yaml") | Some("yml") => serde_yaml::from_str(&content)
                .map_err(|e| CliConfigError::Validation(format!("Invalid YAML: {}", e)))?,
            _ => {
                return Err(CliConfigError::Validation(
                    "Unsupported format. Use json or yaml.".to_string(),
                ));
            }
        };

        let system = self.get_enhanced_soul_system()?;
        system
            .register_personality(personality)
            .map_err(|e| CliConfigError::Soul(format!("Failed to import personality: {}", e)))?;

        println!("Personality imported successfully!");
        Ok(())
    }

    pub fn soul_export(&self, args: ExportSoulArgs) -> Result<(), CliConfigError> {
        println!("Exporting personality to {:?}...", args.path);

        let system = self.get_enhanced_soul_system()?;
        let personality = system
            .get_personality(&args.personality_id)
            .ok_or_else(|| {
                CliConfigError::Soul(format!("Personality not found: {}", args.personality_id))
            })?;

        let content = match args.format.as_deref() {
            Some("json") | None => serde_json::to_string_pretty(&personality)?,
            Some("yaml") | Some("yml") => serde_yaml::to_string(&personality)?,
            _ => {
                return Err(CliConfigError::Validation(
                    "Unsupported format. Use json or yaml.".to_string(),
                ));
            }
        };

        std::fs::write(&args.path, content)?;
        println!("Personality exported successfully!");
        Ok(())
    }

    pub fn soul_optimize(&self, args: OptimizeArgs) -> Result<(), CliConfigError> {
        let system = self.get_enhanced_soul_system()?;

        println!("Optimizing personality: {}", args.personality_id);
        println!("Analyzing usage patterns...");

        if args.auto_apply {
            system
                .evolve_personality(
                    &args.personality_id,
                    EvolutionChangeType::ManualAdjustment,
                    "Auto-optimization based on usage patterns".to_string(),
                    None,
                    None,
                    EvolutionTrigger::PerformanceMetrics,
                )
                .map_err(|e| {
                    CliConfigError::Soul(format!("Failed to optimize personality: {}", e))
                })?;

            println!("Optimization applied successfully!");
        } else {
            println!("Optimization suggestions:");
            println!("  - Use --auto-apply to apply optimization automatically");
        }
        Ok(())
    }

    pub fn soul_history(&self, args: HistoryArgs) -> Result<(), CliConfigError> {
        let system = self.get_enhanced_soul_system()?;
        let history = system.get_evolution_history(&args.personality_id);

        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║                Personality Evolution History                     ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();

        let limit = args.limit.unwrap_or(history.len());
        let display_history = history.iter().rev().take(limit);

        for record in display_history {
            println!("Date:        {}", record.timestamp);
            println!("Change Type: {:?}", record.change_type);
            println!("Triggered:   {:?}", record.triggered_by);
            println!("Description: {}", record.description);
            if let Some(score) = record.effectiveness_score {
                println!("Effectiveness: {:.1}", score);
            }
            println!();
        }
        Ok(())
    }

    pub fn soul_rate(&self, args: RateArgs) -> Result<(), CliConfigError> {
        let system = self.get_enhanced_soul_system()?;
        let user_id = Self::get_default_user_id();

        system
            .rate_personality(
                &args.personality_id,
                user_id,
                args.rating,
                args.comment,
                args.tag,
            )
            .map_err(|e| CliConfigError::Soul(format!("Failed to rate personality: {}", e)))?;

        println!("Thank you for your rating!");
        Ok(())
    }

    pub fn soul_publish(&self, args: PublishArgs) -> Result<(), CliConfigError> {
        let system = self.get_enhanced_soul_system()?;
        let marketplace_id = args.marketplace.unwrap_or_else(|| "default".to_string());

        system
            .publish_to_market(
                &args.personality_id,
                &marketplace_id,
                args.price,
                args.currency,
            )
            .map_err(|e| CliConfigError::Soul(format!("Failed to publish personality: {}", e)))?;

        println!("Personality published to marketplace: {}", marketplace_id);
        Ok(())
    }

    fn get_agent_config_loader() -> AgentConfigLoader {
        AgentConfigLoader::new()
    }

    fn get_agent_template_engine() -> AgentTemplateEngine {
        let mut engine = AgentTemplateEngine::new();
        for template in create_default_templates() {
            let _ = engine.register_template(template);
        }
        engine
    }

    pub fn agent_list(&self, args: AgentListArgs) -> Result<(), CliConfigError> {
        let dir = args.dir.unwrap_or_else(|| self.config_dir.join("agents"));

        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║                      Available Agents                            ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();

        if !dir.exists() {
            println!("No agents directory found at: {:?}", dir);
            println!("Use 'aetheris agent create' to create your first agent.");
            return Ok(());
        }

        let loader = Self::get_agent_config_loader();
        match loader.load_all_from_directory(&dir) {
            Ok(configs) => {
                if configs.is_empty() {
                    println!("No agents found in directory.");
                } else {
                    let filtered: Vec<_> = if args.enabled {
                        configs.into_iter().filter(|c| c.meta.enabled).collect()
                    } else {
                        configs
                    };

                    for (i, config) in filtered.iter().enumerate() {
                        println!("{}. {}", i + 1, config.meta.name);
                        println!("   ID:          {}", config.meta.id);
                        println!("   Version:     {}", config.meta.version);
                        println!("   Type:        {:?}", config.meta.agent_type);
                        println!(
                            "   Enabled:     {}",
                            if config.meta.enabled { "✓" } else { "✗" }
                        );
                        println!(
                            "   Hot Reload:  {}",
                            if config.meta.hot_reload { "✓" } else { "✗" }
                        );
                        if let Some(desc) = &config.meta.description {
                            println!("   Description: {}", desc);
                        }
                        println!();
                    }
                }
            }
            Err(e) => {
                println!("Failed to load agents: {}", e);
            }
        }

        Ok(())
    }

    pub fn agent_create(&self, args: AgentCreateArgs) -> Result<(), CliConfigError> {
        let loader = Self::get_agent_config_loader();
        let path = args.config_path.clone();

        println!("Creating agent from: {:?}", path);

        let config = if let Some(ext) = path.extension() {
            match ext.to_str() {
                Some("yaml") | Some("yml") => loader.load_from_yaml_file(&path)?,
                Some("json5") => loader.load_from_json5_file(&path)?,
                _ => {
                    return Err(CliConfigError::Validation(format!(
                        "Unsupported config format: {:?}",
                        ext
                    )));
                }
            }
        } else {
            loader.load_from_yaml_file(&path)?
        };

        config.validate()?;

        if let Some(output) = args.output {
            let agents_dir = output.parent().unwrap_or(&output);
            std::fs::create_dir_all(agents_dir)?;

            let content = serde_yaml::to_string(&config)?;
            std::fs::write(&output, content)?;
            println!("Agent configuration saved to: {:?}", output);
        }

        println!();
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║                    Agent Created Successfully!                   ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
        println!("Agent ID:      {}", config.meta.id);
        println!("Name:          {}", config.meta.name);
        println!("Version:       {}", config.meta.version);
        println!("Type:          {:?}", config.meta.agent_type);

        Ok(())
    }

    pub fn agent_template(&self, args: AgentTemplateArgs) -> Result<(), CliConfigError> {
        let engine = Self::get_agent_template_engine();

        println!("Creating agent from template: {}", args.template_id);

        let mut variables = HashMap::new();
        for var in args.var {
            let parts: Vec<_> = var.splitn(2, '=').collect();
            if parts.len() == 2 {
                variables.insert(parts[0].to_string(), parts[1].to_string());
            }
        }

        if let Some(name) = args.name {
            variables.insert("agent_name".to_string(), name);
        }
        if let Some(desc) = args.description {
            variables.insert("agent_description".to_string(), desc);
        }

        let workspace = args.workspace.unwrap_or_else(|| {
            dirs::home_dir()
                .map(|p| p.join(".aetheris").join("workspace"))
                .unwrap_or_else(|| PathBuf::from("./workspace"))
        });

        let config = engine
            .render_to_config(&args.template_id, &variables, workspace)
            .map_err(|e| {
                CliConfigError::AgentTemplate(format!("Failed to render template: {}", e))
            })?;

        if let Some(output) = args.output {
            let output_dir = output.parent().unwrap_or(&output);
            std::fs::create_dir_all(output_dir)?;

            let content = serde_yaml::to_string(&config)?;
            std::fs::write(&output, content)?;
            println!("Agent configuration saved to: {:?}", output);
        }

        println!();
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║              Agent Created from Template!                       ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
        println!("Template:      {}", args.template_id);
        println!("Agent ID:      {}", config.meta.id);
        println!("Name:          {}", config.meta.name);
        println!("Version:       {}", config.meta.version);

        Ok(())
    }

    pub fn agent_show(&self, args: AgentShowArgs) -> Result<(), CliConfigError> {
        let loader = Self::get_agent_config_loader();
        let path = args.config_path;

        let config = if let Some(ext) = path.extension() {
            match ext.to_str() {
                Some("yaml") | Some("yml") => loader.load_from_yaml_file(&path)?,
                Some("json5") => loader.load_from_json5_file(&path)?,
                _ => {
                    return Err(CliConfigError::Validation(format!(
                        "Unsupported config format: {:?}",
                        ext
                    )));
                }
            }
        } else {
            loader.load_from_yaml_file(&path)?
        };

        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║                      Agent Configuration                         ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();

        match args.format.as_deref() {
            Some("json") => {
                println!("{}", serde_json::to_string_pretty(&config)?);
            }
            Some("yaml") | Some("yml") | None => {
                println!("{}", serde_yaml::to_string(&config)?);
            }
            _ => {
                return Err(CliConfigError::Validation(
                    "Unsupported format. Use json or yaml.".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn agent_validate(&self, args: AgentValidateArgs) -> Result<(), CliConfigError> {
        let loader = Self::get_agent_config_loader();
        let path = args.config_path;

        println!("Validating agent configuration: {:?}", path);
        println!();

        let config = if let Some(ext) = path.extension() {
            match ext.to_str() {
                Some("yaml") | Some("yml") => loader.load_from_yaml_file(&path)?,
                Some("json5") => loader.load_from_json5_file(&path)?,
                _ => {
                    return Err(CliConfigError::Validation(format!(
                        "Unsupported config format: {:?}",
                        ext
                    )));
                }
            }
        } else {
            loader.load_from_yaml_file(&path)?
        };

        match config.validate() {
            Ok(_) => {
                println!("✓ Configuration is valid!");
                println!();
                println!("Agent ID:      {}", config.meta.id);
                println!("Name:          {}", config.meta.name);
                println!("Version:       {}", config.meta.version);
            }
            Err(e) => {
                println!("✗ Configuration validation failed:");
                println!("  {}", e);
                return Err(CliConfigError::AgentConfig(e));
            }
        }

        Ok(())
    }

    pub fn agent_templates(&self, args: AgentTemplatesArgs) -> Result<(), CliConfigError> {
        let engine = Self::get_agent_template_engine();
        let templates = engine.list_templates();

        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║                   Available Agent Templates                      ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();

        if templates.is_empty() {
            println!("No templates available.");
        } else {
            for (i, template) in templates.iter().enumerate() {
                println!("{}. {}", i + 1, template.name);
                println!("   ID:          {}", template.id);
                println!("   Version:     {}", template.version);
                println!("   Type:        {:?}", template.agent_type);
                if args.details {
                    println!("   Description: {}", template.description);
                    if !template.variables.is_empty() {
                        println!("   Variables:");
                        for var in &template.variables {
                            let req = if var.required {
                                "(required)"
                            } else {
                                "(optional)"
                            };
                            let default = if let Some(d) = &var.default {
                                format!(" [default: {}]", d)
                            } else {
                                String::new()
                            };
                            println!(
                                "      - {}: {} {}{}",
                                var.name, var.description, req, default
                            );
                        }
                    }
                }
                println!();
            }
        }

        Ok(())
    }

    pub fn agent_export(&self, args: AgentExportArgs) -> Result<(), CliConfigError> {
        let loader = Self::get_agent_config_loader();
        let input_path = args.config_path;
        let output_path = args.output_path;

        println!("Exporting agent configuration...");
        println!("  From: {:?}", input_path);
        println!("  To:   {:?}", output_path);

        let config = if let Some(ext) = input_path.extension() {
            match ext.to_str() {
                Some("yaml") | Some("yml") => loader.load_from_yaml_file(&input_path)?,
                Some("json5") => loader.load_from_json5_file(&input_path)?,
                _ => {
                    return Err(CliConfigError::Validation(format!(
                        "Unsupported config format: {:?}",
                        ext
                    )));
                }
            }
        } else {
            loader.load_from_yaml_file(&input_path)?
        };

        config.validate()?;

        let output_dir = output_path.parent().unwrap_or(&output_path);
        std::fs::create_dir_all(output_dir)?;

        let content = match args.format.as_deref() {
            Some("json") => serde_json::to_string_pretty(&config)?,
            Some("yaml") | Some("yml") | None => serde_yaml::to_string(&config)?,
            _ => {
                return Err(CliConfigError::Validation(
                    "Unsupported format. Use json or yaml.".to_string(),
                ));
            }
        };

        std::fs::write(&output_path, content)?;

        println!();
        println!("✓ Agent configuration exported successfully!");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_manager_creation() {
        let dir = tempdir().unwrap();
        let manager = ConfigManager::new(dir.path().to_path_buf());
        assert_eq!(manager.config_dir(), dir.path());
    }
}
