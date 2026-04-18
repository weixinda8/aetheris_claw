use crate::config::openclaw::OpenClawConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OnboardError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Config error: {0}")]
    Config(#[from] crate::config::openclaw::OpenClawConfigError),
    #[error("Soul error: {0}")]
    Soul(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("User cancelled")]
    Cancelled,
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardStep {
    pub id: String,
    pub title: String,
    pub description: String,
    pub required: bool,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardProgress {
    pub current_step: String,
    pub steps: Vec<OnboardStep>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for OnboardProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl OnboardProgress {
    pub fn new() -> Self {
        Self {
            current_step: "welcome".to_string(),
            steps: vec![
                OnboardStep {
                    id: "welcome".to_string(),
                    title: "欢迎使用 Aetheris".to_string(),
                    description: "欢迎使用 Aetheris 2.0 - 统一数字员工与 Agent/Skill 生态系统"
                        .to_string(),
                    required: true,
                    completed: false,
                },
                OnboardStep {
                    id: "detect_openclaw".to_string(),
                    title: "检测 OpenClaw 配置".to_string(),
                    description: "检测是否存在 OpenClaw 配置，支持平滑迁移".to_string(),
                    required: false,
                    completed: false,
                },
                OnboardStep {
                    id: "create_config".to_string(),
                    title: "创建配置文件".to_string(),
                    description: "创建 Aetheris 配置文件".to_string(),
                    required: true,
                    completed: false,
                },
                OnboardStep {
                    id: "setup_soul".to_string(),
                    title: "设置数字员工".to_string(),
                    description: "设置你的数字员工人格（SOUL.md）".to_string(),
                    required: false,
                    completed: false,
                },
                OnboardStep {
                    id: "complete".to_string(),
                    title: "完成设置".to_string(),
                    description: "恭喜！你已完成 Aetheris 的初始设置".to_string(),
                    required: true,
                    completed: false,
                },
            ],
            started_at: chrono::Utc::now(),
            completed_at: None,
        }
    }

    pub fn get_step(&self, id: &str) -> Option<&OnboardStep> {
        self.steps.iter().find(|s| s.id == id)
    }

    pub fn get_step_mut(&mut self, id: &str) -> Option<&mut OnboardStep> {
        self.steps.iter_mut().find(|s| s.id == id)
    }

    pub fn complete_step(&mut self, id: &str) -> Result<(), OnboardError> {
        let step = self
            .get_step_mut(id)
            .ok_or_else(|| OnboardError::Validation(format!("Step not found: {}", id)))?;

        if step.required && !step.completed {
            step.completed = true;
        }

        if let Some(index) = self.steps.iter().position(|s| s.id == id) {
            if let Some(next_step) = self.steps.get(index + 1) {
                self.current_step = next_step.id.clone();
            } else {
                self.completed_at = Some(chrono::Utc::now());
            }
        }

        Ok(())
    }

    pub fn is_complete(&self) -> bool {
        self.completed_at.is_some() || self.steps.iter().all(|s| !s.required || s.completed)
    }

    pub fn progress_percentage(&self) -> f64 {
        let total_required = self.steps.iter().filter(|s| s.required).count();
        if total_required == 0 {
            return 100.0;
        }

        let completed_required = self
            .steps
            .iter()
            .filter(|s| s.required && s.completed)
            .count();

        (completed_required as f64 / total_required as f64) * 100.0
    }

    pub fn save(&self, path: PathBuf) -> std::result::Result<(), OnboardError> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn load(path: PathBuf) -> std::result::Result<Self, OnboardError> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(&path)?;
        let progress: Self = serde_json::from_str(&content)?;
        Ok(progress)
    }
}

pub struct OnboardWizard {
    config_dir: PathBuf,
    progress: OnboardProgress,
}

impl OnboardWizard {
    pub fn new(config_dir: PathBuf) -> std::result::Result<Self, OnboardError> {
        std::fs::create_dir_all(&config_dir)?;

        let progress_path = config_dir.join("onboard-progress.json");
        let progress = OnboardProgress::load(progress_path)?;

        Ok(Self {
            config_dir,
            progress,
        })
    }

    pub fn new_default() -> std::result::Result<Self, OnboardError> {
        let config_dir = dirs::home_dir()
            .ok_or_else(|| OnboardError::Validation("Could not find home directory".to_string()))?
            .join(".aetheris");

        Self::new(config_dir)
    }

    pub fn progress(&self) -> &OnboardProgress {
        &self.progress
    }

    pub fn progress_mut(&mut self) -> &mut OnboardProgress {
        &mut self.progress
    }

    pub fn detect_openclaw_config(&self) -> Option<PathBuf> {
        let home_dir = dirs::home_dir()?;
        let openclaw_path = home_dir.join(".openclaw").join("openclaw.json");
        if openclaw_path.exists() {
            Some(openclaw_path)
        } else {
            None
        }
    }

    pub async fn migrate_openclaw_config(
        &mut self,
        openclaw_path: PathBuf,
    ) -> Result<PathBuf, OnboardError> {
        let openclaw_config = OpenClawConfig::from_path(openclaw_path)?;

        let aetheris_config_path = self.config_dir.join("aetheris.json5");
        openclaw_config.save_aetheris_config(aetheris_config_path.clone())?;

        self.progress.complete_step("detect_openclaw")?;
        self.save_progress()?;

        Ok(aetheris_config_path)
    }

    pub fn create_default_config(&mut self) -> Result<PathBuf, OnboardError> {
        let config_path = self.config_dir.join("aetheris.json5");

        let default_config = r#"// Aetheris Default Config
{
  openclaw: {
    compatible: true,
    migrated_from: null,
  },
  server: {
    host: "127.0.0.1",
    port: 3000,
  },
  llm: {
    provider: "mock",
    model: "gpt-4",
    temperature: 0.7,
    max_tokens: 2000,
    timeout_seconds: 30,
  },
}"#;

        std::fs::write(&config_path, default_config)?;

        self.progress.complete_step("create_config")?;
        self.save_progress()?;

        Ok(config_path)
    }

    pub fn setup_default_soul(&mut self) -> Result<PathBuf, OnboardError> {
        let souls_dir = self.config_dir.join("souls");
        std::fs::create_dir_all(&souls_dir)?;

        let default_soul_path = souls_dir.join("default.md");

        let default_soul_content = r#"---
name: Aetheris Assistant
description: Your personal AI assistant
personality: Friendly, professional, efficient
version: 1.0.0
author: Aetheris
tags:
  - assistant
  - default
---

# Aetheris Assistant

## Personality

You are a friendly, professional, and efficient AI assistant. You always:
- Answer questions in clear, concise language
- Proactively offer useful suggestions
- Respect the user's time and privacy

## Core Instructions

1. **Understand intent**: Carefully analyze the user's question to ensure you understand their real needs
2. **Use appropriate tools**: Choose the most suitable tools for the task
3. **Provide clear results**: Present results in a structured manner
4. **Continuous learning**: Learn from interactions and continuously improve

## Skills Usage Guide

- Prioritize built-in tools
- Actively request user authorization when needed
- Record important decisions for user review
"#;

        std::fs::write(&default_soul_path, default_soul_content)?;

        self.progress.complete_step("setup_soul")?;
        self.save_progress()?;

        Ok(default_soul_path)
    }

    pub fn complete_onboard(&mut self) -> Result<(), OnboardError> {
        self.progress.complete_step("complete")?;
        self.save_progress()?;
        Ok(())
    }

    fn save_progress(&mut self) -> Result<(), OnboardError> {
        let progress_path = self.config_dir.join("onboard-progress.json");
        self.progress.save(progress_path)?;
        Ok(())
    }

    pub fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_onboard_progress_new() {
        let progress = OnboardProgress::new();
        assert_eq!(progress.current_step, "welcome");
        assert_eq!(progress.steps.len(), 5);
        assert!(!progress.is_complete());
    }

    #[test]
    fn test_onboard_progress_complete_step() {
        let mut progress = OnboardProgress::new();
        progress.complete_step("welcome").unwrap();
        assert_eq!(progress.current_step, "detect_openclaw");
    }

    #[test]
    fn test_onboard_wizard_creation() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join(".aetheris");

        let wizard = OnboardWizard::new(config_dir);
        assert!(wizard.is_ok());
    }
}
