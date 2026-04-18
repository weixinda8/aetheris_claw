use dashmap::DashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertRuleType {
    Threshold,
    Trend,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Equal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdCondition {
    pub metric_name: String,
    pub operator: ComparisonOperator,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrendType {
    RateOfChange,
    MovingAverage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendCondition {
    pub metric_name: String,
    pub trend_type: TrendType,
    pub window_seconds: u64,
    pub operator: ComparisonOperator,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AlertCondition {
    Threshold(ThresholdCondition),
    Trend(TrendCondition),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationChannelType {
    Webhook,
    Email,
    Slack,
    Sms,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NotificationChannelConfig {
    Webhook(WebhookConfig),
    Email { recipients: Vec<String> },
    Slack { webhook_url: String },
    Sms { phone_numbers: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub id: String,
    pub name: String,
    pub channel_type: NotificationChannelType,
    pub config: NotificationChannelConfig,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationStep {
    pub delay_seconds: u64,
    pub channel_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationPolicy {
    pub id: String,
    pub name: String,
    pub steps: Vec<EscalationStep>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertRuleStatus {
    Enabled,
    Disabled,
    Muted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub rule_type: AlertRuleType,
    pub condition: AlertCondition,
    pub severity: super::AlertSeverity,
    pub status: AlertRuleStatus,
    pub channel_ids: Vec<String>,
    pub escalation_policy_id: Option<String>,
    pub evaluation_interval_seconds: u64,
    pub last_evaluated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertHistoryStatus {
    Triggered,
    Acknowledged,
    Resolved,
    Suppressed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertHistory {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub severity: super::AlertSeverity,
    pub status: AlertHistoryStatus,
    pub message: String,
    pub metric_value: Option<f64>,
    pub triggered_at: chrono::DateTime<chrono::Utc>,
    pub acknowledged_at: Option<chrono::DateTime<chrono::Utc>>,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub acknowledged_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuteConfig {
    pub rule_id: String,
    pub reason: String,
    pub muted_by: String,
    pub muted_at: chrono::DateTime<chrono::Utc>,
    pub unmute_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct AlertRuleEngine {
    rules: DashMap<String, AlertRule>,
    channels: DashMap<String, NotificationChannel>,
    escalation_policies: DashMap<String, EscalationPolicy>,
    history: DashMap<String, AlertHistory>,
    mute_configs: DashMap<String, MuteConfig>,
    metric_values: DashMap<String, Vec<(chrono::DateTime<chrono::Utc>, f64)>>,
}

impl AlertRuleEngine {
    pub fn new() -> Self {
        Self {
            rules: DashMap::new(),
            channels: DashMap::new(),
            escalation_policies: DashMap::new(),
            history: DashMap::new(),
            mute_configs: DashMap::new(),
            metric_values: DashMap::new(),
        }
    }

    pub fn create_rule(&self, rule: AlertRule) -> String {
        let rule_id = rule.id.clone();
        self.rules.insert(rule_id.clone(), rule);
        rule_id
    }

    pub fn get_rule(&self, rule_id: &str) -> Option<AlertRule> {
        self.rules.get(rule_id).map(|r| r.clone())
    }

    pub fn list_rules(&self) -> Vec<AlertRule> {
        self.rules.iter().map(|r| r.clone()).collect()
    }

    pub fn update_rule(&self, rule_id: &str, mut updated_rule: AlertRule) -> Option<AlertRule> {
        if let Some(mut rule) = self.rules.get_mut(rule_id) {
            updated_rule.id = rule_id.to_string();
            updated_rule.updated_at = chrono::Utc::now();
            *rule = updated_rule.clone();
            Some(updated_rule)
        } else {
            None
        }
    }

    pub fn delete_rule(&self, rule_id: &str) -> bool {
        self.rules.remove(rule_id).is_some()
    }

    pub fn create_channel(&self, channel: NotificationChannel) -> String {
        let channel_id = channel.id.clone();
        self.channels.insert(channel_id.clone(), channel);
        channel_id
    }

    pub fn get_channel(&self, channel_id: &str) -> Option<NotificationChannel> {
        self.channels.get(channel_id).map(|c| c.clone())
    }

    pub fn list_channels(&self) -> Vec<NotificationChannel> {
        self.channels.iter().map(|c| c.clone()).collect()
    }

    pub fn create_escalation_policy(&self, policy: EscalationPolicy) -> String {
        let policy_id = policy.id.clone();
        self.escalation_policies.insert(policy_id.clone(), policy);
        policy_id
    }

    pub fn get_escalation_policy(&self, policy_id: &str) -> Option<EscalationPolicy> {
        self.escalation_policies.get(policy_id).map(|p| p.clone())
    }

    pub fn list_escalation_policies(&self) -> Vec<EscalationPolicy> {
        self.escalation_policies.iter().map(|p| p.clone()).collect()
    }

    pub fn record_metric(&self, metric_name: String, value: f64) {
        let now = chrono::Utc::now();
        self.metric_values
            .entry(metric_name)
            .or_default()
            .push((now, value));
    }

    pub fn evaluate_threshold_condition(
        &self,
        condition: &ThresholdCondition,
    ) -> (bool, Option<f64>) {
        if let Some(values) = self.metric_values.get(&condition.metric_name) {
            if let Some((_, latest_value)) = values.last() {
                let triggered = match condition.operator {
                    ComparisonOperator::GreaterThan => *latest_value > condition.threshold,
                    ComparisonOperator::LessThan => *latest_value < condition.threshold,
                    ComparisonOperator::GreaterOrEqual => *latest_value >= condition.threshold,
                    ComparisonOperator::LessOrEqual => *latest_value <= condition.threshold,
                    ComparisonOperator::Equal => *latest_value == condition.threshold,
                };
                return (triggered, Some(*latest_value));
            }
        }
        (false, None)
    }

    pub fn evaluate_trend_condition(&self, condition: &TrendCondition) -> (bool, Option<f64>) {
        let now = chrono::Utc::now();
        let window_start = now - chrono::Duration::seconds(condition.window_seconds as i64);

        if let Some(values) = self.metric_values.get(&condition.metric_name) {
            let window_values: Vec<_> = values
                .iter()
                .filter(|(ts, _)| *ts >= window_start)
                .collect();

            if window_values.len() >= 2 {
                let metric_value = match condition.trend_type {
                    TrendType::RateOfChange => {
                        let first = window_values.first().unwrap();
                        let last = window_values.last().unwrap();
                        let time_diff = (last.0 - first.0).num_seconds() as f64;
                        if time_diff > 0.0 {
                            (last.1 - first.1) / time_diff
                        } else {
                            0.0
                        }
                    }
                    TrendType::MovingAverage => {
                        window_values.iter().map(|(_, v)| v).sum::<f64>()
                            / window_values.len() as f64
                    }
                };

                let triggered = match condition.operator {
                    ComparisonOperator::GreaterThan => metric_value > condition.threshold,
                    ComparisonOperator::LessThan => metric_value < condition.threshold,
                    ComparisonOperator::GreaterOrEqual => metric_value >= condition.threshold,
                    ComparisonOperator::LessOrEqual => metric_value <= condition.threshold,
                    ComparisonOperator::Equal => metric_value == condition.threshold,
                };
                return (triggered, Some(metric_value));
            }
        }
        (false, None)
    }

    pub fn evaluate_rule(&self, rule: &AlertRule) -> (bool, Option<f64>) {
        if rule.status == AlertRuleStatus::Disabled || rule.status == AlertRuleStatus::Muted {
            return (false, None);
        }

        if self.mute_configs.contains_key(&rule.id) {
            return (false, None);
        }

        match &rule.condition {
            AlertCondition::Threshold(condition) => self.evaluate_threshold_condition(condition),
            AlertCondition::Trend(condition) => self.evaluate_trend_condition(condition),
        }
    }

    pub fn evaluate_all_rules(&self) -> Vec<AlertHistory> {
        let mut triggered_alerts = Vec::new();
        let now = chrono::Utc::now();

        for mut rule_entry in self.rules.iter_mut() {
            let rule = rule_entry.value_mut();

            if let Some(last_eval) = rule.last_evaluated_at {
                let elapsed = (now - last_eval).num_seconds() as u64;
                if elapsed < rule.evaluation_interval_seconds {
                    continue;
                }
            }

            rule.last_evaluated_at = Some(now);

            let (triggered, metric_value) = self.evaluate_rule(rule);

            if triggered {
                let history = AlertHistory {
                    id: uuid::Uuid::new_v4().to_string(),
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    severity: rule.severity.clone(),
                    status: AlertHistoryStatus::Triggered,
                    message: format!("Alert rule '{}' triggered", rule.name),
                    metric_value,
                    triggered_at: now,
                    acknowledged_at: None,
                    resolved_at: None,
                    acknowledged_by: None,
                };

                self.history.insert(history.id.clone(), history.clone());
                triggered_alerts.push(history);
            }
        }

        triggered_alerts
    }

    pub fn get_alert_history(&self, limit: Option<usize>) -> Vec<AlertHistory> {
        let mut history: Vec<_> = self.history.iter().map(|h| h.clone()).collect();
        history.sort_by(|a, b| b.triggered_at.cmp(&a.triggered_at));
        if let Some(limit) = limit {
            history.truncate(limit);
        }
        history
    }

    pub fn acknowledge_alert(&self, history_id: &str, acknowledged_by: String) -> bool {
        if let Some(mut history) = self.history.get_mut(history_id) {
            history.status = AlertHistoryStatus::Acknowledged;
            history.acknowledged_at = Some(chrono::Utc::now());
            history.acknowledged_by = Some(acknowledged_by);
            true
        } else {
            false
        }
    }

    pub fn resolve_alert(&self, history_id: &str) -> bool {
        if let Some(mut history) = self.history.get_mut(history_id) {
            history.status = AlertHistoryStatus::Resolved;
            history.resolved_at = Some(chrono::Utc::now());
            true
        } else {
            false
        }
    }

    pub fn mute_rule(
        &self,
        rule_id: &str,
        reason: String,
        muted_by: String,
        duration_seconds: Option<u64>,
    ) -> bool {
        if self.rules.contains_key(rule_id) {
            let unmute_at = duration_seconds
                .map(|secs| chrono::Utc::now() + chrono::Duration::seconds(secs as i64));

            let config = MuteConfig {
                rule_id: rule_id.to_string(),
                reason,
                muted_by,
                muted_at: chrono::Utc::now(),
                unmute_at,
            };

            self.mute_configs.insert(rule_id.to_string(), config);

            if let Some(mut rule) = self.rules.get_mut(rule_id) {
                rule.status = AlertRuleStatus::Muted;
            }

            true
        } else {
            false
        }
    }

    pub fn unmute_rule(&self, rule_id: &str) -> bool {
        if self.mute_configs.remove(rule_id).is_some() {
            if let Some(mut rule) = self.rules.get_mut(rule_id) {
                rule.status = AlertRuleStatus::Enabled;
            }
            true
        } else {
            false
        }
    }

    pub async fn send_notification(&self, channel_id: &str, message: String) -> Result<(), String> {
        if let Some(channel) = self.channels.get(channel_id) {
            if !channel.enabled {
                return Ok(());
            }

            if let NotificationChannelConfig::Webhook(config) = &channel.config {
                let client = reqwest::Client::new();
                let mut request = client.post(&config.url).json(&serde_json::json!({
                    "message": message,
                    "timestamp": chrono::Utc::now()
                }));

                if let Some(headers) = &config.headers {
                    for (key, value) in headers {
                        request = request.header(key, value);
                    }
                }

                request.send().await.map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}

impl Default for AlertRuleEngine {
    fn default() -> Self {
        Self::new()
    }
}
