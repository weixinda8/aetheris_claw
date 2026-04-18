pub mod enhanced_system;
pub mod marketplace;

use crate::utils::{AetherisError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SoulMetadata {
    pub name: String,
    pub description: String,
    pub personality: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Soul {
    pub metadata: SoulMetadata,
    pub content: String,
    pub path: PathBuf,
}

impl Default for Soul {
    fn default() -> Self {
        Self {
            metadata: SoulMetadata {
                name: "Default Soul".to_string(),
                description: "Default digital employee personality".to_string(),
                personality: "Friendly, professional, efficient".to_string(),
                version: None,
                author: None,
                tags: vec![],
            },
            content: String::new(),
            path: PathBuf::new(),
        }
    }
}

impl Soul {
    pub fn from_path(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Err(AetherisError::NotFound(format!(
                "Soul not found: {:?}",
                path
            )));
        }

        let content = std::fs::read_to_string(&path)?;
        let (metadata, content) = Self::parse_content(&content)?;

        Ok(Self {
            metadata,
            content,
            path,
        })
    }

    fn parse_content(content: &str) -> Result<(SoulMetadata, String)> {
        let lines = content.lines();
        let mut metadata_content = String::new();
        let mut body_content = String::new();
        let mut in_frontmatter = false;
        let mut frontmatter_found = false;

        for line in lines {
            if line.trim() == "---" {
                if !frontmatter_found {
                    in_frontmatter = true;
                    frontmatter_found = true;
                    continue;
                } else {
                    in_frontmatter = false;
                    continue;
                }
            }

            if in_frontmatter {
                metadata_content.push_str(line);
                metadata_content.push('\n');
            } else {
                body_content.push_str(line);
                body_content.push('\n');
            }
        }

        let metadata = if frontmatter_found {
            serde_yaml::from_str(&metadata_content)
                .map_err(|e| AetherisError::Soul(format!("Failed to parse frontmatter: {}", e)))?
        } else {
            SoulMetadata {
                name: "Default Soul".to_string(),
                description: "Default digital employee personality".to_string(),
                personality: "Friendly, professional, efficient".to_string(),
                version: None,
                author: None,
                tags: vec![],
            }
        };

        Soul::validate_metadata(&metadata)?;

        Ok((metadata, body_content.trim().to_string()))
    }

    fn validate_metadata(metadata: &SoulMetadata) -> Result<()> {
        if metadata.name.is_empty() {
            return Err(AetherisError::Soul("Soul name cannot be empty".to_string()));
        }
        if metadata.description.is_empty() {
            return Err(AetherisError::Soul(
                "Soul description cannot be empty".to_string(),
            ));
        }
        if metadata.personality.is_empty() {
            return Err(AetherisError::Soul(
                "Soul personality cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    pub fn description(&self) -> &str {
        &self.metadata.description
    }

    pub fn personality(&self) -> &str {
        &self.metadata.personality
    }

    pub fn system_prompt(&self) -> String {
        format!("{}\n\n{}", self.personality_section(), self.content)
    }

    fn personality_section(&self) -> String {
        format!(
            "# Personality: {}\n\nYou are {}.",
            self.metadata.name, self.metadata.personality
        )
    }

    pub fn save(&self) -> Result<()> {
        let content = self.to_string();
        std::fs::write(&self.path, content)?;
        Ok(())
    }
}

impl std::fmt::Display for Soul {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "---")?;
        writeln!(f, "name: {}", self.metadata.name)?;
        writeln!(f, "description: {}", self.metadata.description)?;
        writeln!(f, "personality: {}", self.metadata.personality)?;

        if let Some(version) = &self.metadata.version {
            writeln!(f, "version: {}", version)?;
        }
        if let Some(author) = &self.metadata.author {
            writeln!(f, "author: {}", author)?;
        }
        if !self.metadata.tags.is_empty() {
            writeln!(f, "tags:")?;
            for tag in &self.metadata.tags {
                writeln!(f, "  - {}", tag)?;
            }
        }

        writeln!(f, "---")?;
        writeln!(f)?;
        writeln!(f, "{}", self.content)
    }
}

#[derive(Debug, Clone)]
pub struct SoulRegistry {
    souls_dir: PathBuf,
    souls: Vec<Soul>,
}

impl SoulRegistry {
    pub fn new(souls_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&souls_dir)?;

        let mut registry = SoulRegistry {
            souls_dir,
            souls: Vec::new(),
        };

        registry.load_all()?;

        Ok(registry)
    }

    pub fn load_all(&mut self) -> Result<()> {
        self.souls.clear();

        if let Ok(entries) = std::fs::read_dir(&self.souls_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                    if let Ok(soul) = Soul::from_path(path) {
                        self.souls.push(soul);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Soul> {
        self.souls.iter().find(|s| s.name() == name)
    }

    pub fn get_by_path(&self, path: &PathBuf) -> Option<&Soul> {
        self.souls.iter().find(|s| &s.path == path)
    }

    pub fn list(&self) -> &[Soul] {
        &self.souls
    }

    pub fn add(&mut self, soul: Soul) -> Result<()> {
        if self.get(soul.name()).is_some() {
            return Err(AetherisError::Soul(format!(
                "Soul with name '{}' already exists",
                soul.name()
            )));
        }
        self.souls.push(soul);
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<()> {
        let index = self
            .souls
            .iter()
            .position(|s| s.name() == name)
            .ok_or_else(|| AetherisError::NotFound(format!("Soul not found: {}", name)))?;

        let soul = self.souls.remove(index);
        std::fs::remove_file(&soul.path)?;

        Ok(())
    }

    pub fn souls_dir(&self) -> &PathBuf {
        &self.souls_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_soul_parse() {
        let content = r#"---
name: Test Assistant
description: A test assistant
personality: Friendly and helpful
version: 1.0.0
author: Test Author
tags:
  - test
  - assistant
---

# Test Soul Content

This is the main content of the soul.
"#;

        let (metadata, body) = Soul::parse_content(content).unwrap();

        assert_eq!(metadata.name, "Test Assistant");
        assert_eq!(metadata.description, "A test assistant");
        assert_eq!(metadata.personality, "Friendly and helpful");
        assert_eq!(metadata.version, Some("1.0.0".to_string()));
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert_eq!(metadata.tags, vec!["test", "assistant"]);
        assert!(body.contains("This is the main content of the soul."));
    }

    #[test]
    fn test_soul_without_frontmatter() {
        let content = "# No Frontmatter\n\nJust content.";

        let (metadata, body) = Soul::parse_content(content).unwrap();

        assert_eq!(metadata.name, "Default Soul");
        assert!(body.contains("Just content."));
    }

    #[test]
    fn test_soul_registry() {
        let dir = tempdir().unwrap();
        let souls_dir = dir.path().join("souls");

        let mut registry = SoulRegistry::new(souls_dir.clone()).unwrap();

        let test_soul_content = r#"---
name: Test Soul
description: Test soul description
personality: Test personality
---

Test content.
"#;

        let soul_path = souls_dir.join("test-soul.md");
        std::fs::write(&soul_path, test_soul_content).unwrap();

        registry.load_all().unwrap();

        assert_eq!(registry.list().len(), 1);
        assert!(registry.get("Test Soul").is_some());
    }
}
