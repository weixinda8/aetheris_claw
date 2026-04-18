use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub condition: RuleCondition,
    pub action: RuleAction,
    pub enabled: bool,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCondition {
    Threshold {
        metric: String,
        operator: ThresholdOperator,
        value: f64,
    },
    PatternMatch {
        field: String,
        pattern: String,
    },
    And(Vec<RuleCondition>),
    Or(Vec<RuleCondition>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThresholdOperator {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    Alert { level: AlertLevel, message: String },
    Log { level: LogLevel, message: String },
    Webhook { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

pub struct LocalRuleEngine {
    rules: Arc<DashMap<String, Rule>>,
}

impl LocalRuleEngine {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(DashMap::new()),
        }
    }

    pub fn add_rule(&self, rule: Rule) {
        self.rules.insert(rule.id.clone(), rule);
    }

    pub fn remove_rule(&self, rule_id: &str) {
        self.rules.remove(rule_id);
    }

    pub fn get_rule(&self, rule_id: &str) -> Option<Rule> {
        self.rules.get(rule_id).map(|r| r.value().clone())
    }

    pub fn list_rules(&self) -> Vec<Rule> {
        self.rules.iter().map(|r| r.value().clone()).collect()
    }

    pub fn update_rule(&self, rule: Rule) {
        self.rules.insert(rule.id.clone(), rule);
    }

    pub async fn evaluate(&self, data: &serde_json::Value) -> Vec<RuleAction> {
        let mut actions = Vec::new();
        let mut rules: Vec<_> = self.rules.iter().map(|r| r.value().clone()).collect();
        rules.sort_by_key(|r| r.priority);

        for rule in rules {
            if !rule.enabled {
                continue;
            }
            if self.evaluate_condition(&rule.condition, data) {
                actions.push(rule.action.clone());
            }
        }
        actions
    }

    fn evaluate_condition(&self, condition: &RuleCondition, data: &serde_json::Value) -> bool {
        match condition {
            RuleCondition::Threshold {
                metric,
                operator,
                value,
            } => {
                if let Some(metric_value) = data.get(metric).and_then(|v| v.as_f64()) {
                    match operator {
                        ThresholdOperator::GreaterThan => metric_value > *value,
                        ThresholdOperator::GreaterThanOrEqual => metric_value >= *value,
                        ThresholdOperator::LessThan => metric_value < *value,
                        ThresholdOperator::LessThanOrEqual => metric_value <= *value,
                        ThresholdOperator::Equal => metric_value == *value,
                        ThresholdOperator::NotEqual => metric_value != *value,
                    }
                } else {
                    false
                }
            }
            RuleCondition::PatternMatch { field, pattern } => {
                if let Some(field_value) = data.get(field).and_then(|v| v.as_str()) {
                    regex::Regex::new(pattern)
                        .map(|re| re.is_match(field_value))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            RuleCondition::And(conditions) => {
                conditions.iter().all(|c| self.evaluate_condition(c, data))
            }
            RuleCondition::Or(conditions) => {
                conditions.iter().any(|c| self.evaluate_condition(c, data))
            }
        }
    }
}

impl Default for LocalRuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_local_rule_engine_new() {
        let engine = LocalRuleEngine::new();
        assert!(engine.list_rules().is_empty());
    }

    #[test]
    fn test_local_rule_engine_default() {
        let engine = LocalRuleEngine::default();
        assert!(engine.list_rules().is_empty());
    }

    #[test]
    fn test_add_and_get_rule() {
        let engine = LocalRuleEngine::new();

        let rule = Rule {
            id: "rule-1".to_string(),
            name: "Test Rule".to_string(),
            description: "Test Description".to_string(),
            condition: RuleCondition::Threshold {
                metric: "temperature".to_string(),
                operator: ThresholdOperator::GreaterThan,
                value: 100.0,
            },
            action: RuleAction::Alert {
                level: AlertLevel::Warning,
                message: "Temperature too high!".to_string(),
            },
            enabled: true,
            priority: 100,
        };

        engine.add_rule(rule.clone());

        let retrieved = engine.get_rule("rule-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Rule");
    }

    #[test]
    fn test_list_rules() {
        let engine = LocalRuleEngine::new();

        let rule1 = Rule {
            id: "rule-1".to_string(),
            name: "Rule 1".to_string(),
            description: "Description 1".to_string(),
            condition: RuleCondition::Threshold {
                metric: "metric1".to_string(),
                operator: ThresholdOperator::GreaterThan,
                value: 10.0,
            },
            action: RuleAction::Alert {
                level: AlertLevel::Info,
                message: "Alert 1".to_string(),
            },
            enabled: true,
            priority: 10,
        };

        let rule2 = Rule {
            id: "rule-2".to_string(),
            name: "Rule 2".to_string(),
            description: "Description 2".to_string(),
            condition: RuleCondition::Threshold {
                metric: "metric2".to_string(),
                operator: ThresholdOperator::LessThan,
                value: 5.0,
            },
            action: RuleAction::Log {
                level: LogLevel::Info,
                message: "Log 1".to_string(),
            },
            enabled: true,
            priority: 20,
        };

        engine.add_rule(rule1);
        engine.add_rule(rule2);

        let rules = engine.list_rules();
        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn test_remove_rule() {
        let engine = LocalRuleEngine::new();

        let rule = Rule {
            id: "rule-1".to_string(),
            name: "Test Rule".to_string(),
            description: "Test Description".to_string(),
            condition: RuleCondition::Threshold {
                metric: "temp".to_string(),
                operator: ThresholdOperator::GreaterThan,
                value: 100.0,
            },
            action: RuleAction::Alert {
                level: AlertLevel::Error,
                message: "Error!".to_string(),
            },
            enabled: true,
            priority: 50,
        };

        engine.add_rule(rule);
        assert!(engine.get_rule("rule-1").is_some());

        engine.remove_rule("rule-1");
        assert!(engine.get_rule("rule-1").is_none());
    }

    #[test]
    fn test_update_rule() {
        let engine = LocalRuleEngine::new();

        let rule = Rule {
            id: "rule-1".to_string(),
            name: "Old Name".to_string(),
            description: "Old Description".to_string(),
            condition: RuleCondition::Threshold {
                metric: "temp".to_string(),
                operator: ThresholdOperator::GreaterThan,
                value: 100.0,
            },
            action: RuleAction::Alert {
                level: AlertLevel::Warning,
                message: "Old message".to_string(),
            },
            enabled: true,
            priority: 50,
        };

        engine.add_rule(rule);

        let updated_rule = Rule {
            id: "rule-1".to_string(),
            name: "New Name".to_string(),
            description: "New Description".to_string(),
            condition: RuleCondition::Threshold {
                metric: "temp".to_string(),
                operator: ThresholdOperator::GreaterThan,
                value: 150.0,
            },
            action: RuleAction::Alert {
                level: AlertLevel::Warning,
                message: "New message".to_string(),
            },
            enabled: true,
            priority: 50,
        };

        engine.update_rule(updated_rule);

        let retrieved = engine.get_rule("rule-1").unwrap();
        assert_eq!(retrieved.name, "New Name");
    }

    #[tokio::test]
    async fn test_evaluate_threshold_greater_than() {
        let engine = LocalRuleEngine::new();

        let rule = Rule {
            id: "rule-1".to_string(),
            name: "Temperature Rule".to_string(),
            description: "Check temperature".to_string(),
            condition: RuleCondition::Threshold {
                metric: "temperature".to_string(),
                operator: ThresholdOperator::GreaterThan,
                value: 100.0,
            },
            action: RuleAction::Alert {
                level: AlertLevel::Warning,
                message: "Too hot!".to_string(),
            },
            enabled: true,
            priority: 100,
        };

        engine.add_rule(rule);

        let data = json!({"temperature": 120.0});
        let actions = engine.evaluate(&data).await;
        assert_eq!(actions.len(), 1);

        let data = json!({"temperature": 80.0});
        let actions = engine.evaluate(&data).await;
        assert!(actions.is_empty());
    }

    #[tokio::test]
    async fn test_evaluate_threshold_less_than_or_equal() {
        let engine = LocalRuleEngine::new();

        let rule = Rule {
            id: "rule-1".to_string(),
            name: "Pressure Rule".to_string(),
            description: "Check pressure".to_string(),
            condition: RuleCondition::Threshold {
                metric: "pressure".to_string(),
                operator: ThresholdOperator::LessThanOrEqual,
                value: 50.0,
            },
            action: RuleAction::Log {
                level: LogLevel::Info,
                message: "Low pressure".to_string(),
            },
            enabled: true,
            priority: 50,
        };

        engine.add_rule(rule);

        let data = json!({"pressure": 40.0});
        let actions = engine.evaluate(&data).await;
        assert_eq!(actions.len(), 1);

        let data = json!({"pressure": 60.0});
        let actions = engine.evaluate(&data).await;
        assert!(actions.is_empty());
    }

    #[tokio::test]
    async fn test_evaluate_disabled_rule() {
        let engine = LocalRuleEngine::new();

        let rule = Rule {
            id: "rule-1".to_string(),
            name: "Disabled Rule".to_string(),
            description: "Should not execute".to_string(),
            condition: RuleCondition::Threshold {
                metric: "value".to_string(),
                operator: ThresholdOperator::GreaterThan,
                value: 0.0,
            },
            action: RuleAction::Alert {
                level: AlertLevel::Info,
                message: "Should not see this".to_string(),
            },
            enabled: false,
            priority: 100,
        };

        engine.add_rule(rule);

        let data = json!({"value": 100.0});
        let actions = engine.evaluate(&data).await;
        assert!(actions.is_empty());
    }

    #[tokio::test]
    async fn test_evaluate_and_condition() {
        let engine = LocalRuleEngine::new();

        let rule = Rule {
            id: "rule-1".to_string(),
            name: "Combined Rule".to_string(),
            description: "Both conditions must be true".to_string(),
            condition: RuleCondition::And(vec![
                RuleCondition::Threshold {
                    metric: "temp".to_string(),
                    operator: ThresholdOperator::GreaterThan,
                    value: 100.0,
                },
                RuleCondition::Threshold {
                    metric: "pressure".to_string(),
                    operator: ThresholdOperator::GreaterThan,
                    value: 50.0,
                },
            ]),
            action: RuleAction::Alert {
                level: AlertLevel::Critical,
                message: "Critical!".to_string(),
            },
            enabled: true,
            priority: 100,
        };

        engine.add_rule(rule);

        let data = json!({"temp": 120.0, "pressure": 60.0});
        let actions = engine.evaluate(&data).await;
        assert_eq!(actions.len(), 1);

        let data = json!({"temp": 120.0, "pressure": 40.0});
        let actions = engine.evaluate(&data).await;
        assert!(actions.is_empty());
    }

    #[tokio::test]
    async fn test_evaluate_or_condition() {
        let engine = LocalRuleEngine::new();

        let rule = Rule {
            id: "rule-1".to_string(),
            name: "Either condition".to_string(),
            description: "Either condition can be true".to_string(),
            condition: RuleCondition::Or(vec![
                RuleCondition::Threshold {
                    metric: "temp".to_string(),
                    operator: ThresholdOperator::GreaterThan,
                    value: 100.0,
                },
                RuleCondition::Threshold {
                    metric: "pressure".to_string(),
                    operator: ThresholdOperator::GreaterThan,
                    value: 100.0,
                },
            ]),
            action: RuleAction::Alert {
                level: AlertLevel::Warning,
                message: "Warning!".to_string(),
            },
            enabled: true,
            priority: 50,
        };

        engine.add_rule(rule);

        let data = json!({"temp": 120.0, "pressure": 50.0});
        let actions = engine.evaluate(&data).await;
        assert_eq!(actions.len(), 1);

        let data = json!({"temp": 50.0, "pressure": 120.0});
        let actions = engine.evaluate(&data).await;
        assert_eq!(actions.len(), 1);
    }

    #[tokio::test]
    async fn test_rule_priority_ordering() {
        let engine = LocalRuleEngine::new();

        let rule_low = Rule {
            id: "rule-low".to_string(),
            name: "Low Priority".to_string(),
            description: "Runs later".to_string(),
            condition: RuleCondition::Threshold {
                metric: "value".to_string(),
                operator: ThresholdOperator::GreaterThan,
                value: 0.0,
            },
            action: RuleAction::Log {
                level: LogLevel::Info,
                message: "Low priority".to_string(),
            },
            enabled: true,
            priority: 200,
        };

        let rule_high = Rule {
            id: "rule-high".to_string(),
            name: "High Priority".to_string(),
            description: "Runs first".to_string(),
            condition: RuleCondition::Threshold {
                metric: "value".to_string(),
                operator: ThresholdOperator::GreaterThan,
                value: 0.0,
            },
            action: RuleAction::Log {
                level: LogLevel::Info,
                message: "High priority".to_string(),
            },
            enabled: true,
            priority: 50,
        };

        engine.add_rule(rule_low);
        engine.add_rule(rule_high);

        let data = json!({"value": 100.0});
        let actions = engine.evaluate(&data).await;
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn test_threshold_operator_equality() {
        let op1 = ThresholdOperator::GreaterThan;
        let op2 = ThresholdOperator::GreaterThan;
        let op3 = ThresholdOperator::LessThan;

        assert_eq!(op1, op2);
        assert_ne!(op1, op3);
    }

    #[test]
    fn test_alert_level_equality() {
        let level1 = AlertLevel::Warning;
        let level2 = AlertLevel::Warning;
        let level3 = AlertLevel::Error;

        assert_eq!(level1, level2);
        assert_ne!(level1, level3);
    }

    #[test]
    fn test_log_level_equality() {
        let level1 = LogLevel::Info;
        let level2 = LogLevel::Info;
        let level3 = LogLevel::Error;

        assert_eq!(level1, level2);
        assert_ne!(level1, level3);
    }
}
