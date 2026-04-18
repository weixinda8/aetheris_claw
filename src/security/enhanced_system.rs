#![deprecated(note = "This enhanced security system is not used. Use SecurityManager from security/mod.rs instead.")]

use crate::security::capability::CapabilitySecurityModel;
use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SecurityLevel {
    Restricted,
    Low,
    Medium,
    High,
    Critical,
    Maximum,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AuditEventType {
    Authentication,
    Authorization,
    CapabilityGrant,
    CapabilityRevoke,
    ResourceAccess,
    ResourceModify,
    PluginLoad,
    PluginUnload,
    ConfigChange,
    SecurityPolicyUpdate,
    SuspiciousActivity,
    ThreatDetected,
    Incident,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub event_type: AuditEventType,
    pub user_id: Option<String>,
    pub component_id: Option<String>,
    pub resource_id: Option<String>,
    pub action: String,
    pub success: bool,
    pub details: Option<serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source_ip: Option<String>,
    pub user_agent: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub policy_id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub enabled: bool,
    pub rules: Vec<SecurityRule>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRule {
    pub rule_id: String,
    pub name: String,
    pub description: String,
    pub rule_type: RuleType,
    pub conditions: Vec<RuleCondition>,
    pub actions: Vec<RuleAction>,
    pub enabled: bool,
    pub severity: SecurityLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RuleType {
    Authentication,
    Authorization,
    RateLimit,
    ResourceAccess,
    ThreatDetection,
    BehaviorAnomaly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    pub condition_id: String,
    pub field: String,
    pub operator: ConditionOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    In,
    NotIn,
    Matches,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleAction {
    pub action_id: String,
    pub action_type: ActionType,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ActionType {
    Allow,
    Deny,
    Log,
    Alert,
    Block,
    Quarantine,
    RevokeSession,
    NotifyAdmin,
    Throttle,
    Challenge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub limit_id: String,
    pub resource: String,
    pub max_requests: u32,
    pub window_seconds: u64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitState {
    pub resource: String,
    pub user_id: Option<String>,
    pub requests: u32,
    pub window_start: Instant,
    pub blocked: bool,
    pub blocked_until: Option<Instant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatIndicator {
    pub indicator_id: String,
    pub indicator_type: ThreatType,
    pub value: String,
    pub severity: SecurityLevel,
    pub source: String,
    pub confidence: f64,
    pub first_seen: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ThreatType {
    MaliciousIP,
    MaliciousUserAgent,
    KnownExploit,
    SuspiciousPattern,
    CredentialStuffing,
    BruteForce,
    SqlInjection,
    XssAttack,
    CsrfAttack,
    PathTraversal,
    CommandInjection,
    FileInclusion,
    UnusualBehavior,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIncident {
    pub incident_id: String,
    pub title: String,
    pub description: String,
    pub severity: SecurityLevel,
    pub status: IncidentStatus,
    pub threat_indicators: Vec<String>,
    pub affected_resources: Vec<String>,
    pub events: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub assigned_to: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IncidentStatus {
    Open,
    Investigating,
    Mitigated,
    Resolved,
    Closed,
    FalsePositive,
}

pub struct EnhancedSecuritySystem {
    capability_model: Arc<CapabilitySecurityModel>,
    policies: Arc<DashMap<String, SecurityPolicy>>,
    audit_events: Arc<DashMap<String, AuditEvent>>,
    rate_limits: Arc<DashMap<String, RateLimitConfig>>,
    rate_limit_states: Arc<DashMap<String, RateLimitState>>,
    threat_indicators: Arc<DashMap<String, ThreatIndicator>>,
    incidents: Arc<DashMap<String, SecurityIncident>>,
    trusted_ips: Arc<DashSet<String>>,
    blocked_ips: Arc<DashSet<String>>,
    storage_path: PathBuf,
}

use dashmap::DashSet;

impl EnhancedSecuritySystem {
    pub fn new(
        capability_model: Arc<CapabilitySecurityModel>,
        storage_path: PathBuf,
    ) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;

        Ok(Self {
            capability_model,
            policies: Arc::new(DashMap::new()),
            audit_events: Arc::new(DashMap::new()),
            rate_limits: Arc::new(DashMap::new()),
            rate_limit_states: Arc::new(DashMap::new()),
            threat_indicators: Arc::new(DashMap::new()),
            incidents: Arc::new(DashMap::new()),
            trusted_ips: Arc::new(DashSet::new()),
            blocked_ips: Arc::new(DashSet::new()),
            storage_path,
        })
    }

    pub fn add_policy(&self, policy: SecurityPolicy) -> Result<()> {
        if self.policies.contains_key(&policy.policy_id) {
            return Err(AetherisError::Validation(format!(
                "Policy with ID '{}' already exists",
                policy.policy_id
            )));
        }

        info!("Adding security policy: {}", policy.name);
        self.policies.insert(policy.policy_id.clone(), policy);

        Ok(())
    }

    pub fn get_policy(&self, policy_id: &str) -> Option<SecurityPolicy> {
        self.policies.get(policy_id).map(|p| p.value().clone())
    }

    pub fn list_policies(&self, enabled_only: bool) -> Vec<SecurityPolicy> {
        self.policies
            .iter()
            .filter(|entry| !enabled_only || entry.value().enabled)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn evaluate_access(
        &self,
        user_id: &str,
        resource_id: &str,
        action: &str,
        context: Option<serde_json::Value>,
    ) -> Result<bool> {
        let mut allowed = self
            .capability_model
            .check_capability(user_id, &format!("{}:{}", resource_id, action))
            .unwrap_or(false);

        if allowed {
            for policy in self.list_policies(true) {
                for rule in &policy.rules {
                    if rule.enabled && rule.rule_type == RuleType::Authorization {
                        allowed = allowed && self.evaluate_rule(rule, user_id, resource_id, action, &context);
                    }
                }
            }
        }

        self.log_audit_event(AuditEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: AuditEventType::Authorization,
            user_id: Some(user_id.to_string()),
            component_id: None,
            resource_id: Some(resource_id.to_string()),
            action: action.to_string(),
            success: allowed,
            details: context,
            timestamp: chrono::Utc::now(),
            source_ip: None,
            user_agent: None,
            session_id: None,
        })?;

        Ok(allowed)
    }

    fn evaluate_rule(
        &self,
        rule: &SecurityRule,
        user_id: &str,
        resource_id: &str,
        action: &str,
        context: &Option<serde_json::Value>,
    ) -> bool {
        let mut all_conditions_met = true;

        for condition in &rule.conditions {
            if !self.evaluate_condition(condition, user_id, resource_id, action, context) {
                all_conditions_met = false;
                break;
            }
        }

        all_conditions_met
    }

    fn evaluate_condition(
        &self,
        condition: &RuleCondition,
        user_id: &str,
        resource_id: &str,
        action: &str,
        context: &Option<serde_json::Value>,
    ) -> bool {
        let field_value = match condition.field.as_str() {
            "user_id" => serde_json::json!(user_id),
            "resource_id" => serde_json::json!(resource_id),
            "action" => serde_json::json!(action),
            _ => {
                if let Some(ctx) = context {
                    ctx.get(&condition.field).cloned().unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                }
            }
        };

        match condition.operator {
            ConditionOperator::Equals => field_value == condition.value,
            ConditionOperator::NotEquals => field_value != condition.value,
            _ => true,
        }
    }

    pub fn log_audit_event(&self, event: AuditEvent) -> Result<()> {
        self.audit_events
            .insert(event.event_id.clone(), event.clone());

        if !event.success {
            warn!(
                "Security audit event failed: {:?} - {}",
                event.event_type, event.action
            );
        }

        Ok(())
    }

    pub fn query_audit_events(
        &self,
        event_type: Option<AuditEventType>,
        user_id: Option<&str>,
        start_time: Option<chrono::DateTime<chrono::Utc>>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<usize>,
    ) -> Vec<AuditEvent> {
        let mut events: Vec<AuditEvent> = self
            .audit_events
            .iter()
            .filter(|entry| {
                let event = entry.value();

                if let Some(et) = &event_type {
                    if event.event_type != *et {
                        return false;
                    }
                }

                if let Some(uid) = user_id {
                    if event.user_id.as_deref() != Some(uid) {
                        return false;
                    }
                }

                if let Some(start) = start_time {
                    if event.timestamp < start {
                        return false;
                    }
                }

                if let Some(end) = end_time {
                    if event.timestamp > end {
                        return false;
                    }
                }

                true
            })
            .map(|entry| entry.value().clone())
            .collect();

        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        if let Some(limit) = limit {
            events.truncate(limit);
        }

        events
    }

    pub fn check_rate_limit(&self, resource: &str, user_id: Option<&str>) -> Result<bool> {
        let key = if let Some(uid) = user_id {
            format!("{}:{}", resource, uid)
        } else {
            resource.to_string()
        };

        let config = self.rate_limits.get(resource).ok_or_else(|| {
            AetherisError::NotFound(format!("Rate limit config not found for: {}", resource))
        })?;

        let now = Instant::now();
        let window = Duration::from_secs(config.window_seconds);

        let mut state = self
            .rate_limit_states
            .entry(key.clone())
            .or_insert_with(|| RateLimitState {
                resource: resource.to_string(),
                user_id: user_id.map(|s| s.to_string()),
                requests: 0,
                window_start: now,
                blocked: false,
                blocked_until: None,
            });

        if state.blocked {
            if let Some(blocked_until) = state.blocked_until {
                if now < blocked_until {
                    return Ok(false);
                } else {
                    state.blocked = false;
                    state.blocked_until = None;
                }
            }
        }

        if now.duration_since(state.window_start) > window {
            state.requests = 0;
            state.window_start = now;
        }

        if state.requests >= config.max_requests {
            state.blocked = true;
            state.blocked_until = Some(now + window);
            return Ok(false);
        }

        state.requests += 1;
        Ok(true)
    }

    pub fn add_threat_indicator(&self, indicator: ThreatIndicator) -> Result<()> {
        self.threat_indicators
            .insert(indicator.indicator_id.clone(), indicator.clone());

        warn!("Added threat indicator: {:?} - {}", indicator.threat_type, indicator.value);

        Ok(())
    }

    pub fn check_threat_indicator(&self, threat_type: ThreatType, value: &str) -> bool {
        self.threat_indicators
            .iter()
            .any(|entry| {
                let indicator = entry.value();
                indicator.active && indicator.threat_type == threat_type && indicator.value == value
            })
    }

    pub fn create_incident(&self, incident: SecurityIncident) -> Result<()> {
        self.incidents
            .insert(incident.incident_id.clone(), incident.clone());

        warn!(
            "Created security incident: {} (severity: {:?})",
            incident.title, incident.severity
        );

        Ok(())
    }

    pub fn get_incident(&self, incident_id: &str) -> Option<SecurityIncident> {
        self.incidents.get(incident_id).map(|i| i.value().clone())
    }

    pub fn list_incidents(&self, status: Option<IncidentStatus>) -> Vec<SecurityIncident> {
        self.incidents
            .iter()
            .filter(|entry| {
                if let Some(s) = &status {
                    entry.value().status == *s
                } else {
                    true
                }
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn add_trusted_ip(&self, ip: &str) {
        self.trusted_ips.insert(ip.to_string());
        info!("Added trusted IP: {}", ip);
    }

    pub fn remove_trusted_ip(&self, ip: &str) {
        self.trusted_ips.remove(ip);
        info!("Removed trusted IP: {}", ip);
    }

    pub fn is_trusted_ip(&self, ip: &str) -> bool {
        self.trusted_ips.contains(ip)
    }

    pub fn block_ip(&self, ip: &str, duration: Option<Duration>) {
        self.blocked_ips.insert(ip.to_string());
        warn!("Blocked IP: {}", ip);
    }

    pub fn unblock_ip(&self, ip: &str) {
        self.blocked_ips.remove(ip);
        info!("Unblocked IP: {}", ip);
    }

    pub fn is_blocked_ip(&self, ip: &str) -> bool {
        self.blocked_ips.contains(ip)
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    pub fn audit_event_count(&self) -> usize {
        self.audit_events.len()
    }

    pub fn threat_indicator_count(&self) -> usize {
        self.threat_indicators.len()
    }

    pub fn incident_count(&self) -> usize {
        self.incidents.len()
    }
}

impl Default for EnhancedSecuritySystem {
    fn default() -> Self {
        let storage_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".aetheris")
            .join("security");

        let cap_model = Arc::new(CapabilitySecurityModel::default());

        Self::new(cap_model, storage_path).unwrap_or_else(|_| {
            let temp_dir = tempfile::tempdir().unwrap();
            let cap_model = Arc::new(CapabilitySecurityModel::default());
            Self::new(cap_model, temp_dir.path().to_path_buf()).unwrap()
        })
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            policy_id: uuid::Uuid::new_v4().to_string(),
            name: "Default Policy".to_string(),
            description: "Default security policy".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            rules: Vec::new(),
            created_at: now,
            updated_at: now,
            priority: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_security_system_new() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cap_model = Arc::new(CapabilitySecurityModel::default());
        let system = EnhancedSecuritySystem::new(cap_model, temp_dir.path().to_path_buf());
        assert!(system.is_ok());
    }
}
