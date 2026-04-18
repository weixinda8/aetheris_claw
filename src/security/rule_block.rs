use crate::core::Task;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuleSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRule {
    pub rule_id: String,
    pub rule_name: String,
    pub description: String,
    pub severity: RuleSeverity,
    pub pattern: Option<String>,
    pub forbidden_keywords: Vec<String>,
    pub enabled: bool,
}

impl SecurityRule {
    pub fn new(
        rule_id: String,
        rule_name: String,
        description: String,
        severity: RuleSeverity,
    ) -> Self {
        Self {
            rule_id,
            rule_name,
            description,
            severity,
            pattern: None,
            forbidden_keywords: Vec::new(),
            enabled: true,
        }
    }

    pub fn with_pattern(mut self, pattern: String) -> Self {
        self.pattern = Some(pattern);
        self
    }

    pub fn with_forbidden_keywords(mut self, keywords: Vec<String>) -> Self {
        self.forbidden_keywords = keywords;
        self
    }

    pub fn matches(&self, task: &Task) -> bool {
        if !self.enabled {
            return false;
        }

        let content = format!("{} {}", task.description, task.id);

        if !self.forbidden_keywords.is_empty() {
            for keyword in &self.forbidden_keywords {
                if content.to_lowercase().contains(&keyword.to_lowercase()) {
                    return true;
                }
            }
        }

        if let Some(pattern) = &self.pattern {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(&content) {
                    return true;
                }
            }
        }

        false
    }
}

pub struct RuleEngine {
    rules: Vec<SecurityRule>,
    blocked_tasks: HashSet<String>,
}

impl RuleEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            rules: Vec::new(),
            blocked_tasks: HashSet::new(),
        };
        engine.load_default_rules();
        engine
    }

    fn load_default_rules(&mut self) {
        self.add_rule(
            SecurityRule::new(
                "rule-001".to_string(),
                "Dangerous System Commands".to_string(),
                "Blocks dangerous system commands".to_string(),
                RuleSeverity::Critical,
            )
            .with_forbidden_keywords(vec![
                "rm -rf".to_string(),
                "format".to_string(),
                "del /s".to_string(),
                "mkfs".to_string(),
                ":(){ :|:& };:".to_string(),
            ]),
        );

        self.add_rule(
            SecurityRule::new(
                "rule-002".to_string(),
                "Sensitive Data Access".to_string(),
                "Blocks access to sensitive data patterns".to_string(),
                RuleSeverity::High,
            )
            .with_forbidden_keywords(vec![
                "password".to_string(),
                "secret".to_string(),
                "api_key".to_string(),
                "private_key".to_string(),
                "credit_card".to_string(),
            ]),
        );

        self.add_rule(
            SecurityRule::new(
                "rule-003".to_string(),
                "Network Scanning".to_string(),
                "Blocks network scanning tools".to_string(),
                RuleSeverity::High,
            )
            .with_forbidden_keywords(vec![
                "nmap".to_string(),
                "port scan".to_string(),
                "network scan".to_string(),
            ]),
        );

        self.add_rule(
            SecurityRule::new(
                "rule-004".to_string(),
                "Privilege Escalation".to_string(),
                "Blocks privilege escalation attempts".to_string(),
                RuleSeverity::Critical,
            )
            .with_forbidden_keywords(vec![
                "sudo".to_string(),
                "su root".to_string(),
                "runas".to_string(),
                "elevate".to_string(),
            ]),
        );
    }

    pub fn add_rule(&mut self, rule: SecurityRule) {
        info!(
            "Adding security rule: {} - {}",
            rule.rule_id, rule.rule_name
        );
        self.rules.push(rule);
    }

    pub fn remove_rule(&mut self, rule_id: &str) {
        info!("Removing security rule: {}", rule_id);
        self.rules.retain(|r| r.rule_id != rule_id);
    }

    pub fn enable_rule(&mut self, rule_id: &str) {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.rule_id == rule_id) {
            info!("Enabling security rule: {}", rule_id);
            rule.enabled = true;
        }
    }

    pub fn disable_rule(&mut self, rule_id: &str) {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.rule_id == rule_id) {
            info!("Disabling security rule: {}", rule_id);
            rule.enabled = false;
        }
    }

    pub fn check(&self, task: &Task) -> bool {
        if self.blocked_tasks.contains(&task.id) {
            warn!("Task {} is in blocked list", task.id);
            return false;
        }

        for rule in &self.rules {
            if rule.matches(task) {
                warn!(
                    "Task {} blocked by rule {} - Severity: {:?}",
                    task.id, rule.rule_id, rule.severity
                );
                return false;
            }
        }

        info!("Task {} passed all security rules", task.id);
        true
    }

    pub fn get_violations(&self, task: &Task) -> Vec<&SecurityRule> {
        self.rules
            .iter()
            .filter(|rule| rule.matches(task))
            .collect()
    }

    pub fn block_task(&mut self, task_id: String) {
        info!("Blocking task: {}", task_id);
        self.blocked_tasks.insert(task_id);
    }

    pub fn unblock_task(&mut self, task_id: &str) {
        info!("Unblocking task: {}", task_id);
        self.blocked_tasks.remove(task_id);
    }

    pub fn list_rules(&self) -> Vec<&SecurityRule> {
        self.rules.iter().collect()
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}
