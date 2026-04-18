use super::*;
use dashmap::DashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RollbackTrigger {
    Manual,
    PerformanceDrop,
    HighErrorRate,
    DriftDetected,
    ABTestWinner,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPolicy {
    pub id: String,
    pub name: String,
    pub model_id: String,
    pub triggers: Vec<RollbackTrigger>,
    pub performance_threshold: Option<f64>,
    pub error_rate_threshold: Option<f64>,
    pub drift_threshold: Option<f64>,
    pub cooldown_seconds: u64,
    pub target_version: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl RollbackPolicy {
    pub fn new(name: String, model_id: String, triggers: Vec<RollbackTrigger>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            model_id,
            triggers,
            performance_threshold: None,
            error_rate_threshold: None,
            drift_threshold: None,
            cooldown_seconds: 3600,
            target_version: None,
            enabled: true,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackEvent {
    pub id: String,
    pub model_id: String,
    pub from_version: String,
    pub to_version: String,
    pub trigger: RollbackTrigger,
    pub reason: String,
    pub policy_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct AutoRollbackManager {
    policies: DashMap<String, RollbackPolicy>,
    rollback_history: DashMap<String, Vec<RollbackEvent>>,
    last_rollback: DashMap<String, DateTime<Utc>>,
    version_manager: ModelVersionManager,
}

impl AutoRollbackManager {
    pub fn new(version_manager: ModelVersionManager) -> Self {
        Self {
            policies: DashMap::new(),
            rollback_history: DashMap::new(),
            last_rollback: DashMap::new(),
            version_manager,
        }
    }

    pub fn create_policy(&self, policy: RollbackPolicy) -> String {
        let policy_id = policy.id.clone();
        self.policies.insert(policy_id.clone(), policy);
        policy_id
    }

    pub fn get_policy(&self, policy_id: &str) -> Option<RollbackPolicy> {
        self.policies.get(policy_id).map(|p| p.clone())
    }

    pub fn list_policies(&self, model_id: &str) -> Vec<RollbackPolicy> {
        self.policies
            .iter()
            .filter(|p| p.model_id == model_id)
            .map(|p| p.clone())
            .collect()
    }

    pub fn update_policy(&self, policy_id: &str, policy: RollbackPolicy) -> bool {
        if self.policies.contains_key(policy_id) {
            self.policies.insert(policy_id.to_string(), policy);
            true
        } else {
            false
        }
    }

    pub fn delete_policy(&self, policy_id: &str) -> bool {
        self.policies.remove(policy_id).is_some()
    }

    pub fn enable_policy(&self, policy_id: &str) -> bool {
        if let Some(mut policy) = self.policies.get_mut(policy_id) {
            policy.enabled = true;
            true
        } else {
            false
        }
    }

    pub fn disable_policy(&self, policy_id: &str) -> bool {
        if let Some(mut policy) = self.policies.get_mut(policy_id) {
            policy.enabled = false;
            true
        } else {
            false
        }
    }

    pub fn check_and_rollback(
        &self,
        model_id: &str,
        metrics: &PerformanceMetrics,
        drift_result: Option<&DriftDetectionResult>,
    ) -> Option<RollbackEvent> {
        let policies = self.list_policies(model_id);

        for policy in policies {
            if !policy.enabled {
                continue;
            }

            if let Some(last_rollback) = self.last_rollback.get(model_id) {
                let cooldown_end =
                    *last_rollback + chrono::Duration::seconds(policy.cooldown_seconds as i64);
                if Utc::now() < cooldown_end {
                    continue;
                }
            }

            let mut should_rollback = false;
            let mut trigger = None;
            let mut reason = String::new();

            for t in &policy.triggers {
                match t {
                    RollbackTrigger::PerformanceDrop => {
                        if let (Some(threshold), Some(accuracy)) =
                            (policy.performance_threshold, metrics.accuracy)
                        {
                            if accuracy < threshold {
                                should_rollback = true;
                                trigger = Some(RollbackTrigger::PerformanceDrop);
                                reason = format!(
                                    "Performance dropped below threshold: {:.2} < {:.2}",
                                    accuracy, threshold
                                );
                                break;
                            }
                        }
                    }
                    RollbackTrigger::HighErrorRate => {
                        if let Some(threshold) = policy.error_rate_threshold {
                            if metrics.error_rate > threshold {
                                should_rollback = true;
                                trigger = Some(RollbackTrigger::HighErrorRate);
                                reason = format!(
                                    "Error rate exceeded threshold: {:.2} > {:.2}",
                                    metrics.error_rate, threshold
                                );
                                break;
                            }
                        }
                    }
                    RollbackTrigger::DriftDetected => {
                        if let (Some(threshold), Some(drift)) =
                            (policy.drift_threshold, drift_result)
                        {
                            if drift.has_drift && drift.drift_score > threshold {
                                should_rollback = true;
                                trigger = Some(RollbackTrigger::DriftDetected);
                                reason = format!(
                                    "Data drift detected: score = {:.2}",
                                    drift.drift_score
                                );
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }

            if should_rollback {
                let event = self.execute_rollback(
                    model_id,
                    policy.target_version.as_deref(),
                    trigger.unwrap(),
                    reason,
                    Some(policy.id.clone()),
                );
                return Some(event);
            }
        }

        None
    }

    pub fn execute_rollback(
        &self,
        model_id: &str,
        target_version: Option<&str>,
        trigger: RollbackTrigger,
        reason: String,
        policy_id: Option<String>,
    ) -> RollbackEvent {
        let current_version = self.version_manager.get_active_version(model_id);

        let to_version = target_version
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                let versions = self.version_manager.list_versions(model_id);
                versions.get(1)
                    .map(|v| v.version.clone())
                    .unwrap_or_else(|| "unknown".to_string())
            });

        let from_version = current_version
            .map(|v| v.version)
            .unwrap_or_else(|| "unknown".to_string());

        if let Some(version) = target_version {
            self.version_manager.rollback_to_version(model_id, version);
        }

        let event = RollbackEvent {
            id: uuid::Uuid::new_v4().to_string(),
            model_id: model_id.to_string(),
            from_version,
            to_version,
            trigger,
            reason,
            policy_id,
            created_at: Utc::now(),
        };

        self.rollback_history
            .entry(model_id.to_string())
            .or_default()
            .push(event.clone());

        self.last_rollback.insert(model_id.to_string(), Utc::now());

        event
    }

    pub fn manual_rollback(
        &self,
        model_id: &str,
        target_version: &str,
        reason: String,
    ) -> RollbackEvent {
        self.execute_rollback(
            model_id,
            Some(target_version),
            RollbackTrigger::Manual,
            reason,
            None,
        )
    }

    pub fn get_rollback_history(&self, model_id: &str, limit: Option<usize>) -> Vec<RollbackEvent> {
        self.rollback_history
            .get(model_id)
            .map(|history| {
                let mut events = history.clone();
                events.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                if let Some(limit) = limit {
                    events.truncate(limit);
                }
                events
            })
            .unwrap_or_default()
    }
}
