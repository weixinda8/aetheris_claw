use async_trait::async_trait;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MaskingAlgorithm {
    Replace,
    Mask,
    Generalize,
    Delete,
    Encrypt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskingRule {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub field_pattern: String,
    pub algorithm: MaskingAlgorithm,
    pub config: MaskingConfig,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskingConfig {
    pub replace_with: Option<String>,
    pub mask_char: Option<char>,
    pub mask_prefix: Option<usize>,
    pub mask_suffix: Option<usize>,
    pub generalize_level: Option<u32>,
    pub encryption_key: Option<String>,
}

impl Default for MaskingConfig {
    fn default() -> Self {
        Self {
            replace_with: Some("***".to_string()),
            mask_char: Some('*'),
            mask_prefix: Some(0),
            mask_suffix: Some(0),
            generalize_level: Some(1),
            encryption_key: None,
        }
    }
}

impl MaskingRule {
    pub fn new(
        name: String,
        description: String,
        field_pattern: String,
        algorithm: MaskingAlgorithm,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            field_pattern,
            algorithm,
            config: MaskingConfig::default(),
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_config(mut self, config: MaskingConfig) -> Self {
        self.config = config;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskingException {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub data_id: String,
    pub reason: String,
    pub granted_by: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl MaskingException {
    pub fn new(rule_id: Uuid, data_id: String, reason: String, granted_by: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            rule_id,
            data_id,
            reason,
            granted_by,
            expires_at: None,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskingResult {
    pub original_data: String,
    pub masked_data: String,
    pub rule_id: Option<Uuid>,
    pub algorithm: MaskingAlgorithm,
    pub masked_at: DateTime<Utc>,
}

#[async_trait]
pub trait DataMasker: Send + Sync {
    async fn mask(
        &self,
        data: &str,
        rule: Option<&MaskingRule>,
    ) -> crate::utils::Result<MaskingResult>;
    fn name(&self) -> &str;
}

pub struct StaticMasker {
    rules: HashMap<Uuid, MaskingRule>,
    exceptions: HashMap<Uuid, MaskingException>,
}

impl StaticMasker {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            exceptions: HashMap::new(),
        }
    }

    pub fn add_rule(&mut self, rule: MaskingRule) {
        self.rules.insert(rule.id, rule);
    }

    pub fn add_exception(&mut self, exception: MaskingException) {
        self.exceptions.insert(exception.id, exception);
    }

    fn apply_mask(&self, data: &str, rule: &MaskingRule) -> String {
        match rule.algorithm {
            MaskingAlgorithm::Replace => rule
                .config
                .replace_with
                .clone()
                .unwrap_or_else(|| "***".to_string()),
            MaskingAlgorithm::Mask => {
                let mask_char = rule.config.mask_char.unwrap_or('*');
                let prefix = rule.config.mask_prefix.unwrap_or(0);
                let suffix = rule.config.mask_suffix.unwrap_or(0);
                let len = data.len();

                if len <= prefix + suffix {
                    mask_char.to_string().repeat(len)
                } else {
                    let masked_part = mask_char.to_string().repeat(len - prefix - suffix);
                    format!(
                        "{}{}{}",
                        &data[..prefix],
                        masked_part,
                        &data[len - suffix..]
                    )
                }
            }
            MaskingAlgorithm::Generalize => {
                let level = rule.config.generalize_level.unwrap_or(1);
                match level {
                    1 => data.chars().take(2).collect::<String>() + "***",
                    2 => data.chars().next().map_or(String::new(), |c| c.to_string()) + "***",
                    _ => "***".to_string(),
                }
            }
            MaskingAlgorithm::Delete => String::new(),
            MaskingAlgorithm::Encrypt => {
                let mut hasher = Sha256::new();
                hasher.update(data.as_bytes());
                let result = hasher.finalize();
                format!("{:x}", result)
            }
        }
    }

    fn find_matching_rule(&self, field_name: &str) -> Option<&MaskingRule> {
        self.rules.values().find(|rule| {
            if !rule.enabled {
                return false;
            }
            if let Ok(re) = Regex::new(&rule.field_pattern) {
                re.is_match(field_name)
            } else {
                rule.field_pattern == field_name
            }
        })
    }

    fn is_exception(&self, rule_id: &Uuid, data_id: &str) -> bool {
        self.exceptions.values().any(|exc| {
            if exc.rule_id != *rule_id || exc.data_id != data_id {
                return false;
            }
            if let Some(expires) = exc.expires_at {
                expires > Utc::now()
            } else {
                true
            }
        })
    }
}

impl Default for StaticMasker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataMasker for StaticMasker {
    async fn mask(
        &self,
        data: &str,
        rule: Option<&MaskingRule>,
    ) -> crate::utils::Result<MaskingResult> {
        let (used_rule, algorithm) = if let Some(r) = rule {
            (Some(r.clone()), r.algorithm.clone())
        } else {
            let default_rule = MaskingRule::new(
                "default".to_string(),
                "Default masking rule".to_string(),
                ".*".to_string(),
                MaskingAlgorithm::Mask,
            )
            .with_config(MaskingConfig {
                mask_prefix: Some(1),
                mask_suffix: Some(1),
                ..Default::default()
            });
            let algorithm = default_rule.algorithm.clone();
            (Some(default_rule), algorithm)
        };

        let masked_data = if let Some(r) = &used_rule {
            self.apply_mask(data, r)
        } else {
            data.to_string()
        };

        Ok(MaskingResult {
            original_data: data.to_string(),
            masked_data,
            rule_id: used_rule.map(|r| r.id),
            algorithm,
            masked_at: Utc::now(),
        })
    }

    fn name(&self) -> &str {
        "StaticMasker"
    }
}

pub struct DynamicMasker {
    rules: HashMap<Uuid, MaskingRule>,
    user_permissions: HashMap<String, HashSet<Uuid>>,
}

impl DynamicMasker {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            user_permissions: HashMap::new(),
        }
    }

    pub fn add_rule(&mut self, rule: MaskingRule) {
        self.rules.insert(rule.id, rule);
    }

    pub fn grant_permission(&mut self, user_id: String, rule_id: Uuid) {
        self.user_permissions
            .entry(user_id)
            .or_default()
            .insert(rule_id);
    }

    pub fn revoke_permission(&mut self, user_id: &str, rule_id: &Uuid) {
        if let Some(perms) = self.user_permissions.get_mut(user_id) {
            perms.remove(rule_id);
        }
    }

    fn apply_dynamic_mask(&self, data: &str, rule: &MaskingRule, user_id: Option<&str>) -> String {
        if let Some(uid) = user_id {
            if let Some(perms) = self.user_permissions.get(uid) {
                if perms.contains(&rule.id) {
                    return data.to_string();
                }
            }
        }

        match rule.algorithm {
            MaskingAlgorithm::Replace => rule
                .config
                .replace_with
                .clone()
                .unwrap_or_else(|| "***".to_string()),
            MaskingAlgorithm::Mask => {
                let mask_char = rule.config.mask_char.unwrap_or('*');
                let prefix = rule.config.mask_prefix.unwrap_or(0);
                let suffix = rule.config.mask_suffix.unwrap_or(0);
                let len = data.len();

                if len <= prefix + suffix {
                    mask_char.to_string().repeat(len)
                } else {
                    let masked_part = mask_char.to_string().repeat(len - prefix - suffix);
                    format!(
                        "{}{}{}",
                        &data[..prefix],
                        masked_part,
                        &data[len - suffix..]
                    )
                }
            }
            MaskingAlgorithm::Generalize => {
                let level = rule.config.generalize_level.unwrap_or(1);
                match level {
                    1 => data.chars().take(3).collect::<String>() + "***",
                    2 => data.chars().take(1).collect::<String>() + "***",
                    _ => "***".to_string(),
                }
            }
            MaskingAlgorithm::Delete => String::new(),
            MaskingAlgorithm::Encrypt => {
                let mut hasher = Sha256::new();
                hasher.update(data.as_bytes());
                let result = hasher.finalize();
                format!("{:x}", result)
            }
        }
    }

    pub async fn mask_for_user(
        &self,
        data: &str,
        rule: &MaskingRule,
        user_id: Option<&str>,
    ) -> crate::utils::Result<MaskingResult> {
        let masked_data = self.apply_dynamic_mask(data, rule, user_id);

        Ok(MaskingResult {
            original_data: data.to_string(),
            masked_data,
            rule_id: Some(rule.id),
            algorithm: rule.algorithm.clone(),
            masked_at: Utc::now(),
        })
    }
}

impl Default for DynamicMasker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataMasker for DynamicMasker {
    async fn mask(
        &self,
        data: &str,
        rule: Option<&MaskingRule>,
    ) -> crate::utils::Result<MaskingResult> {
        let used_rule = rule.cloned().unwrap_or_else(|| {
            MaskingRule::new(
                "default".to_string(),
                "Default dynamic masking rule".to_string(),
                ".*".to_string(),
                MaskingAlgorithm::Mask,
            )
        });

        let masked_data = self.apply_dynamic_mask(data, &used_rule, None);

        Ok(MaskingResult {
            original_data: data.to_string(),
            masked_data,
            rule_id: Some(used_rule.id),
            algorithm: used_rule.algorithm.clone(),
            masked_at: Utc::now(),
        })
    }

    fn name(&self) -> &str {
        "DynamicMasker"
    }
}

pub struct MaskingManager {
    static_masker: StaticMasker,
    dynamic_masker: DynamicMasker,
    rules: HashMap<Uuid, MaskingRule>,
    exceptions: HashMap<Uuid, MaskingException>,
    masking_history: Vec<MaskingResult>,
}

impl MaskingManager {
    pub fn new() -> Self {
        Self {
            static_masker: StaticMasker::new(),
            dynamic_masker: DynamicMasker::new(),
            rules: HashMap::new(),
            exceptions: HashMap::new(),
            masking_history: Vec::new(),
        }
    }

    pub fn add_rule(&mut self, rule: MaskingRule) {
        self.rules.insert(rule.id, rule.clone());
        self.static_masker.add_rule(rule.clone());
        self.dynamic_masker.add_rule(rule);
    }

    pub fn get_rule(&self, rule_id: &Uuid) -> Option<&MaskingRule> {
        self.rules.get(rule_id)
    }

    pub fn list_rules(&self) -> Vec<&MaskingRule> {
        self.rules.values().collect()
    }

    pub fn add_exception(&mut self, exception: MaskingException) {
        self.exceptions.insert(exception.id, exception.clone());
        self.static_masker.add_exception(exception);
    }

    pub fn get_exception(&self, exception_id: &Uuid) -> Option<&MaskingException> {
        self.exceptions.get(exception_id)
    }

    pub fn list_exceptions(&self) -> Vec<&MaskingException> {
        self.exceptions.values().collect()
    }

    pub async fn mask_static(
        &self,
        data: &str,
        rule: Option<&MaskingRule>,
    ) -> crate::utils::Result<MaskingResult> {
        self.static_masker.mask(data, rule).await
    }

    pub async fn mask_dynamic(
        &self,
        data: &str,
        rule: &MaskingRule,
        user_id: Option<&str>,
    ) -> crate::utils::Result<MaskingResult> {
        self.dynamic_masker.mask_for_user(data, rule, user_id).await
    }

    pub fn grant_user_permission(&mut self, user_id: String, rule_id: Uuid) {
        self.dynamic_masker.grant_permission(user_id, rule_id);
    }

    pub fn revoke_user_permission(&mut self, user_id: &str, rule_id: &Uuid) {
        self.dynamic_masker.revoke_permission(user_id, rule_id);
    }
}

impl Default for MaskingManager {
    fn default() -> Self {
        Self::new()
    }
}
