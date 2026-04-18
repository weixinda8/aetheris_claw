use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TemplateType {
    AetherisConfig,
    Soul,
    Skill,
    Agent,
    SecurityPolicy,
    Plugin,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigTemplate {
    pub template_id: String,
    pub name: String,
    pub description: String,
    pub template_type: TemplateType,
    pub version: String,
    pub author: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub content: serde_json::Value,
    pub variables: Vec<TemplateVariable>,
    pub examples: Vec<TemplateExample>,
    pub is_official: bool,
    pub is_published: bool,
    pub download_count: u64,
    pub rating: f32,
    pub rating_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    pub variable_type: VariableType,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub validation_rules: Vec<ValidationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VariableType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub rule_type: ValidationRuleType,
    pub parameters: HashMap<String, serde_json::Value>,
    pub error_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValidationRuleType {
    MinLength,
    MaxLength,
    MinValue,
    MaxValue,
    Pattern,
    Enum,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateExample {
    pub name: String,
    pub description: String,
    pub variables: HashMap<String, serde_json::Value>,
    pub expected_output: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateRating {
    pub rating_id: String,
    pub template_id: String,
    pub user_id: String,
    pub rating: u8,
    pub comment: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct ConfigTemplateLibrary {
    templates: Arc<DashMap<String, ConfigTemplate>>,
    ratings: Arc<DashMap<String, Vec<TemplateRating>>>,
    template_type_index: Arc<DashMap<TemplateType, Vec<String>>>,
    tag_index: Arc<DashMap<String, Vec<String>>>,
    category_index: Arc<DashMap<String, Vec<String>>>,
    author_index: Arc<DashMap<String, Vec<String>>>,
    storage_path: PathBuf,
}

impl ConfigTemplateLibrary {
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;

        let instance = Self {
            templates: Arc::new(DashMap::new()),
            ratings: Arc::new(DashMap::new()),
            template_type_index: Arc::new(DashMap::new()),
            tag_index: Arc::new(DashMap::new()),
            category_index: Arc::new(DashMap::new()),
            author_index: Arc::new(DashMap::new()),
            storage_path,
        };

        instance.load()?;

        Ok(instance)
    }

    pub fn save(&self) -> Result<()> {
        let templates_path = self.storage_path.join("templates.json");
        let ratings_path = self.storage_path.join("ratings.json");
        let template_type_index_path = self.storage_path.join("template_type_index.json");
        let tag_index_path = self.storage_path.join("tag_index.json");
        let category_index_path = self.storage_path.join("category_index.json");
        let author_index_path = self.storage_path.join("author_index.json");

        let templates: Vec<ConfigTemplate> =
            self.templates.iter().map(|t| t.value().clone()).collect();
        let ratings: Vec<(String, Vec<TemplateRating>)> = self
            .ratings
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        let template_type_index: Vec<(TemplateType, Vec<String>)> = self
            .template_type_index
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        let tag_index: Vec<(String, Vec<String>)> = self
            .tag_index
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        let category_index: Vec<(String, Vec<String>)> = self
            .category_index
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        let author_index: Vec<(String, Vec<String>)> = self
            .author_index
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        std::fs::write(templates_path, serde_json::to_string_pretty(&templates)?)?;
        std::fs::write(ratings_path, serde_json::to_string_pretty(&ratings)?)?;
        std::fs::write(
            template_type_index_path,
            serde_json::to_string_pretty(&template_type_index)?,
        )?;
        std::fs::write(tag_index_path, serde_json::to_string_pretty(&tag_index)?)?;
        std::fs::write(
            category_index_path,
            serde_json::to_string_pretty(&category_index)?,
        )?;
        std::fs::write(
            author_index_path,
            serde_json::to_string_pretty(&author_index)?,
        )?;

        info!("ConfigTemplateLibrary saved to: {:?}", self.storage_path);

        Ok(())
    }

    fn load(&self) -> Result<()> {
        let templates_path = self.storage_path.join("templates.json");
        let ratings_path = self.storage_path.join("ratings.json");
        let template_type_index_path = self.storage_path.join("template_type_index.json");
        let tag_index_path = self.storage_path.join("tag_index.json");
        let category_index_path = self.storage_path.join("category_index.json");
        let author_index_path = self.storage_path.join("author_index.json");

        if templates_path.exists() {
            let content = std::fs::read_to_string(templates_path)?;
            let templates: Vec<ConfigTemplate> = serde_json::from_str(&content)?;
            for template in templates {
                self.templates
                    .insert(template.template_id.clone(), template);
            }
        }

        if ratings_path.exists() {
            let content = std::fs::read_to_string(ratings_path)?;
            let ratings: Vec<(String, Vec<TemplateRating>)> = serde_json::from_str(&content)?;
            for (template_id, template_ratings) in ratings {
                self.ratings.insert(template_id, template_ratings);
            }
        }

        if template_type_index_path.exists() {
            let content = std::fs::read_to_string(template_type_index_path)?;
            let template_type_index: Vec<(TemplateType, Vec<String>)> =
                serde_json::from_str(&content)?;
            for (template_type, template_ids) in template_type_index {
                self.template_type_index.insert(template_type, template_ids);
            }
        }

        if tag_index_path.exists() {
            let content = std::fs::read_to_string(tag_index_path)?;
            let tag_index: Vec<(String, Vec<String>)> = serde_json::from_str(&content)?;
            for (tag, template_ids) in tag_index {
                self.tag_index.insert(tag, template_ids);
            }
        }

        if category_index_path.exists() {
            let content = std::fs::read_to_string(category_index_path)?;
            let category_index: Vec<(String, Vec<String>)> = serde_json::from_str(&content)?;
            for (category, template_ids) in category_index {
                self.category_index.insert(category, template_ids);
            }
        }

        if author_index_path.exists() {
            let content = std::fs::read_to_string(author_index_path)?;
            let author_index: Vec<(String, Vec<String>)> = serde_json::from_str(&content)?;
            for (author, template_ids) in author_index {
                self.author_index.insert(author, template_ids);
            }
        }

        info!("ConfigTemplateLibrary loaded from: {:?}", self.storage_path);

        Ok(())
    }

    pub fn register_template(&self, template: ConfigTemplate) -> Result<()> {
        info!(
            "Registering template: {} ({:?}) by {}",
            template.name, template.template_type, template.author
        );

        let template_id = template.template_id.clone();

        if self.templates.contains_key(&template_id) {
            return Err(AetherisError::Validation(format!(
                "Template with ID '{}' already exists",
                template_id
            )));
        }

        self.templates.insert(template_id.clone(), template.clone());

        self.update_indices(&template_id, &template);

        self.save()?;

        Ok(())
    }

    fn update_indices(&self, template_id: &str, template: &ConfigTemplate) {
        self.template_type_index
            .entry(template.template_type.clone())
            .or_default()
            .push(template_id.to_string());

        for tag in &template.tags {
            self.tag_index
                .entry(tag.clone())
                .or_default()
                .push(template_id.to_string());
        }

        for category in &template.categories {
            self.category_index
                .entry(category.clone())
                .or_default()
                .push(template_id.to_string());
        }

        self.author_index
            .entry(template.author.clone())
            .or_default()
            .push(template_id.to_string());
    }

    pub fn get_template(&self, template_id: &str) -> Option<ConfigTemplate> {
        self.templates.get(template_id).map(|t| t.value().clone())
    }

    pub fn list_templates(&self) -> Vec<ConfigTemplate> {
        self.templates
            .iter()
            .filter(|entry| entry.value().is_published)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_templates_by_type(&self, template_type: &TemplateType) -> Vec<ConfigTemplate> {
        self.template_type_index
            .get(template_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_template(id))
                    .filter(|t| t.is_published)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_templates_by_tag(&self, tag: &str) -> Vec<ConfigTemplate> {
        self.tag_index
            .get(tag)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_template(id))
                    .filter(|t| t.is_published)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_templates_by_category(&self, category: &str) -> Vec<ConfigTemplate> {
        self.category_index
            .get(category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_template(id))
                    .filter(|t| t.is_published)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_templates_by_author(&self, author: &str) -> Vec<ConfigTemplate> {
        self.author_index
            .get(author)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_template(id))
                    .filter(|t| t.is_published)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn search_templates(&self, query: &str) -> Vec<ConfigTemplate> {
        let query_lower = query.to_lowercase();
        self.templates
            .iter()
            .filter(|entry| {
                let template = entry.value();
                if !template.is_published {
                    return false;
                }

                template.name.to_lowercase().contains(&query_lower)
                    || template.description.to_lowercase().contains(&query_lower)
                    || template.template_id.to_lowercase().contains(&query_lower)
                    || template.author.to_lowercase().contains(&query_lower)
                    || template
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
                    || template
                        .categories
                        .iter()
                        .any(|c| c.to_lowercase().contains(&query_lower))
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn instantiate_template(
        &self,
        template_id: &str,
        variables: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let template = self.get_template(template_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Template not found: {}", template_id))
        })?;

        info!("Instantiating template: {}", template.name);

        self.validate_variables(&template, &variables)?;

        let mut content = template.content.clone();
        self.substitute_variables(&mut content, &variables)?;

        if let Some(mut template) = self.templates.get_mut(template_id) {
            template.download_count += 1;
        }

        self.save()?;

        Ok(content)
    }

    fn validate_variables(
        &self,
        template: &ConfigTemplate,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        for var in &template.variables {
            if var.required && !variables.contains_key(&var.name) {
                return Err(AetherisError::Validation(format!(
                    "Required variable '{}' is missing",
                    var.name
                )));
            }

            if let Some(value) = variables.get(&var.name) {
                self.validate_variable_value(var, value)?;
            }
        }

        Ok(())
    }

    fn validate_variable_value(
        &self,
        var: &TemplateVariable,
        value: &serde_json::Value,
    ) -> Result<()> {
        let type_valid = matches!((&var.variable_type, value),
            (VariableType::String, serde_json::Value::String(_)) |
            (VariableType::Number, serde_json::Value::Number(_)) |
            (VariableType::Boolean, serde_json::Value::Bool(_)) |
            (VariableType::Array, serde_json::Value::Array(_)) |
            (VariableType::Object, serde_json::Value::Object(_))
        );

        if !type_valid {
            return Err(AetherisError::Validation(format!(
                "Variable '{}' has incorrect type",
                var.name
            )));
        }

        for rule in &var.validation_rules {
            self.apply_validation_rule(var, value, rule)?;
        }

        Ok(())
    }

    fn apply_validation_rule(
        &self,
        _var: &TemplateVariable,
        value: &serde_json::Value,
        rule: &ValidationRule,
    ) -> Result<()> {
        match rule.rule_type {
            ValidationRuleType::MinLength => {
                if let Some(min) = rule.parameters.get("min").and_then(|p| p.as_u64()) {
                    if let Some(s) = value.as_str() {
                        if s.len() < min as usize {
                            return Err(AetherisError::Validation(rule.error_message.clone()));
                        }
                    } else if let Some(arr) = value.as_array() {
                        if arr.len() < min as usize {
                            return Err(AetherisError::Validation(rule.error_message.clone()));
                        }
                    }
                }
            }
            ValidationRuleType::MaxLength => {
                if let Some(max) = rule.parameters.get("max").and_then(|p| p.as_u64()) {
                    if let Some(s) = value.as_str() {
                        if s.len() > max as usize {
                            return Err(AetherisError::Validation(rule.error_message.clone()));
                        }
                    } else if let Some(arr) = value.as_array() {
                        if arr.len() > max as usize {
                            return Err(AetherisError::Validation(rule.error_message.clone()));
                        }
                    }
                }
            }
            ValidationRuleType::MinValue => {
                if let Some(min) = rule.parameters.get("min").and_then(|p| p.as_f64()) {
                    if let Some(n) = value.as_f64() {
                        if n < min {
                            return Err(AetherisError::Validation(rule.error_message.clone()));
                        }
                    }
                }
            }
            ValidationRuleType::MaxValue => {
                if let Some(max) = rule.parameters.get("max").and_then(|p| p.as_f64()) {
                    if let Some(n) = value.as_f64() {
                        if n > max {
                            return Err(AetherisError::Validation(rule.error_message.clone()));
                        }
                    }
                }
            }
            ValidationRuleType::Pattern => {
                if let Some(pattern) = rule.parameters.get("pattern").and_then(|p| p.as_str()) {
                    if let Some(s) = value.as_str() {
                        let regex = regex::Regex::new(pattern).map_err(|e| {
                            AetherisError::Validation(format!("Invalid pattern: {}", e))
                        })?;
                        if !regex.is_match(s) {
                            return Err(AetherisError::Validation(rule.error_message.clone()));
                        }
                    }
                }
            }
            ValidationRuleType::Enum => {
                if let Some(options) = rule.parameters.get("options").and_then(|p| p.as_array()) {
                    let mut found = false;
                    for option in options {
                        if option == value {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Err(AetherisError::Validation(rule.error_message.clone()));
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn substitute_variables(
        &self,
        content: &mut serde_json::Value,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        match content {
            serde_json::Value::String(s) => {
                for (var_name, var_value) in variables {
                    let placeholder = format!("{{{{{}}}}}", var_name);
                    if s.contains(&placeholder) {
                        if let Some(replacement) = var_value.as_str() {
                            *s = s.replace(&placeholder, replacement);
                        }
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    self.substitute_variables(item, variables)?;
                }
            }
            serde_json::Value::Object(obj) => {
                for (_key, value) in obj {
                    self.substitute_variables(value, variables)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub fn rate_template(
        &self,
        template_id: &str,
        user_id: String,
        rating: u8,
        comment: Option<String>,
    ) -> Result<()> {
        if !(1..=5).contains(&rating) {
            return Err(AetherisError::Validation(
                "Rating must be between 1 and 5".to_string(),
            ));
        }

        info!(
            "Rating template: {} by user: {} with rating: {}",
            template_id, user_id, rating
        );

        let template_rating = TemplateRating {
            rating_id: uuid::Uuid::new_v4().to_string(),
            template_id: template_id.to_string(),
            user_id,
            rating,
            comment,
            created_at: chrono::Utc::now(),
        };

        self.ratings
            .entry(template_id.to_string())
            .or_default()
            .push(template_rating);

        self.update_template_rating(template_id)?;

        self.save()?;

        Ok(())
    }

    fn update_template_rating(&self, template_id: &str) -> Result<()> {
        if let Some(ratings) = self.ratings.get(template_id) {
            let rating_count = ratings.len() as u32;
            if rating_count > 0 {
                let total_rating: u32 = ratings.iter().map(|r| r.rating as u32).sum();
                let average_rating = total_rating as f32 / rating_count as f32;

                if let Some(mut template) = self.templates.get_mut(template_id) {
                    template.rating = average_rating;
                    template.rating_count = rating_count;
                }
            }
        }

        Ok(())
    }

    pub fn get_template_ratings(&self, template_id: &str) -> Vec<TemplateRating> {
        self.ratings
            .get(template_id)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }

    pub fn get_popular_templates(&self, limit: usize) -> Vec<ConfigTemplate> {
        let mut templates = self.list_templates();
        templates.sort_by(|a, b| {
            b.download_count.cmp(&a.download_count).then_with(|| {
                b.rating
                    .partial_cmp(&a.rating)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });
        templates.truncate(limit);
        templates
    }

    pub fn get_top_rated_templates(&self, limit: usize) -> Vec<ConfigTemplate> {
        let mut templates = self.list_templates();
        templates.sort_by(|a, b| {
            b.rating
                .partial_cmp(&a.rating)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.rating_count.cmp(&a.rating_count))
        });
        templates.truncate(limit);
        templates
    }

    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    pub fn published_template_count(&self) -> usize {
        self.list_templates().len()
    }
}

impl Default for ConfigTemplateLibrary {
    fn default() -> Self {
        let storage_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".aetheris")
            .join("config-templates");

        Self::new(storage_path).unwrap_or_else(|_| {
            let temp_dir = tempfile::tempdir().unwrap();
            Self::new(temp_dir.path().to_path_buf()).unwrap()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_library_new() {
        let temp_dir = tempfile::tempdir().unwrap();
        let library = ConfigTemplateLibrary::new(temp_dir.path().to_path_buf());
        assert!(library.is_ok());
    }

    #[test]
    fn test_register_template() {
        let temp_dir = tempfile::tempdir().unwrap();
        let library = ConfigTemplateLibrary::new(temp_dir.path().to_path_buf()).unwrap();

        let template = ConfigTemplate {
            template_id: "test-template".to_string(),
            name: "Test Template".to_string(),
            description: "A test template".to_string(),
            template_type: TemplateType::AetherisConfig,
            version: "1.0.0".to_string(),
            author: "test-author".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec!["test".to_string()],
            categories: vec!["example".to_string()],
            content: serde_json::json!({"key": "{{value}}"}),
            variables: vec![],
            examples: vec![],
            is_official: false,
            is_published: true,
            download_count: 0,
            rating: 0.0,
            rating_count: 0,
        };

        let result = library.register_template(template);
        assert!(result.is_ok());
        assert_eq!(library.published_template_count(), 1);
    }

    #[test]
    fn test_search_templates() {
        let temp_dir = tempfile::tempdir().unwrap();
        let library = ConfigTemplateLibrary::new(temp_dir.path().to_path_buf()).unwrap();

        let template = ConfigTemplate {
            template_id: "test-template".to_string(),
            name: "Test Template".to_string(),
            description: "A test template".to_string(),
            template_type: TemplateType::AetherisConfig,
            version: "1.0.0".to_string(),
            author: "test-author".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec!["test".to_string()],
            categories: vec!["example".to_string()],
            content: serde_json::json!({}),
            variables: vec![],
            examples: vec![],
            is_official: false,
            is_published: true,
            download_count: 0,
            rating: 0.0,
            rating_count: 0,
        };

        library.register_template(template).unwrap();

        let results = library.search_templates("test");
        assert!(!results.is_empty());
    }
}
