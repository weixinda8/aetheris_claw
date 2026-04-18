use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    pub task_id: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<u64>,
    pub status: String,
    pub agent_id: Option<String>,
    pub tokens_used: Option<u64>,
    pub cost_usd: Option<f64>,
}

impl Default for TaskMetrics {
    fn default() -> Self {
        Self {
            task_id: String::new(),
            start_time: chrono::Utc::now(),
            end_time: None,
            duration_ms: None,
            status: "unknown".to_string(),
            agent_id: None,
            tokens_used: None,
            cost_usd: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub total_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub active_tasks: u64,
    pub success_rate: f64,
    pub average_duration_ms: f64,
    pub total_tokens_used: u64,
    pub total_cost_usd: f64,
    pub active_agents: usize,
    pub uptime_seconds: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub alert_id: String,
    pub alert_type: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub task_id: Option<String>,
    pub agent_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub resolved: bool,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

pub struct MetricsCollector {
    total_tasks: AtomicU64,
    completed_tasks: AtomicU64,
    failed_tasks: AtomicU64,
    active_tasks: AtomicU64,
    total_tokens_used: AtomicU64,
    total_duration_ms: AtomicU64,
    task_metrics: DashMap<String, TaskMetrics>,
    alerts: DashMap<String, Alert>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            total_tasks: AtomicU64::new(0),
            completed_tasks: AtomicU64::new(0),
            failed_tasks: AtomicU64::new(0),
            active_tasks: AtomicU64::new(0),
            total_tokens_used: AtomicU64::new(0),
            total_duration_ms: AtomicU64::new(0),
            task_metrics: DashMap::new(),
            alerts: DashMap::new(),
        }
    }

    pub fn record_task_start(&self, task_id: String, agent_id: Option<String>) {
        self.total_tasks.fetch_add(1, Ordering::Relaxed);
        self.active_tasks.fetch_add(1, Ordering::Relaxed);

        let task_metrics = TaskMetrics {
            task_id: task_id.clone(),
            start_time: chrono::Utc::now(),
            end_time: None,
            duration_ms: None,
            status: "running".to_string(),
            agent_id,
            tokens_used: None,
            cost_usd: None,
        };

        self.task_metrics.insert(task_id, task_metrics);
    }

    pub fn record_task_completion(
        &self,
        task_id: &str,
        success: bool,
        tokens_used: Option<u64>,
        cost_usd: Option<f64>,
    ) {
        if let Some(mut task_metrics) = self.task_metrics.get_mut(task_id) {
            let end_time = chrono::Utc::now();
            let duration_ms = (end_time - task_metrics.start_time).num_milliseconds() as u64;

            task_metrics.end_time = Some(end_time);
            task_metrics.duration_ms = Some(duration_ms);
            task_metrics.status = if success {
                "completed".to_string()
            } else {
                "failed".to_string()
            };
            task_metrics.tokens_used = tokens_used;
            task_metrics.cost_usd = cost_usd;

            self.total_duration_ms
                .fetch_add(duration_ms, Ordering::Relaxed);

            if success {
                self.completed_tasks.fetch_add(1, Ordering::Relaxed);
            } else {
                self.failed_tasks.fetch_add(1, Ordering::Relaxed);
            }

            if let Some(tokens) = tokens_used {
                self.total_tokens_used.fetch_add(tokens, Ordering::Relaxed);
            }

            self.active_tasks.fetch_sub(1, Ordering::Relaxed);
        }
    }

    pub fn get_task_metrics(&self, task_id: &str) -> Option<TaskMetrics> {
        self.task_metrics.get(task_id).map(|m| m.clone())
    }

    pub fn get_all_task_metrics(&self) -> Vec<TaskMetrics> {
        self.task_metrics.iter().map(|m| m.clone()).collect()
    }

    pub fn get_system_metrics(&self, active_agents: usize, uptime_seconds: u64) -> SystemMetrics {
        let total = self.total_tasks.load(Ordering::Relaxed);
        let completed = self.completed_tasks.load(Ordering::Relaxed);
        let failed = self.failed_tasks.load(Ordering::Relaxed);
        let active = self.active_tasks.load(Ordering::Relaxed);
        let total_duration = self.total_duration_ms.load(Ordering::Relaxed);
        let total_tokens = self.total_tokens_used.load(Ordering::Relaxed);

        let success_rate = if total == 0 {
            0.0
        } else {
            completed as f64 / total as f64
        };

        let average_duration = if completed + failed == 0 {
            0.0
        } else {
            total_duration as f64 / (completed + failed) as f64
        };

        SystemMetrics {
            total_tasks: total,
            completed_tasks: completed,
            failed_tasks: failed,
            active_tasks: active,
            success_rate,
            average_duration_ms: average_duration,
            total_tokens_used: total_tokens,
            total_cost_usd: 0.0,
            active_agents,
            uptime_seconds,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn create_alert(
        &self,
        alert_type: String,
        severity: AlertSeverity,
        message: String,
        task_id: Option<String>,
        agent_id: Option<String>,
    ) -> String {
        let alert_id = uuid::Uuid::new_v4().to_string();
        let alert = Alert {
            alert_id: alert_id.clone(),
            alert_type,
            severity,
            message,
            task_id,
            agent_id,
            created_at: chrono::Utc::now(),
            resolved: false,
            resolved_at: None,
        };

        self.alerts.insert(alert_id.clone(), alert);
        alert_id
    }

    pub fn resolve_alert(&self, alert_id: &str) -> bool {
        if let Some(mut alert) = self.alerts.get_mut(alert_id) {
            alert.resolved = true;
            alert.resolved_at = Some(chrono::Utc::now());
            true
        } else {
            false
        }
    }

    pub fn get_alerts(&self, include_resolved: bool) -> Vec<Alert> {
        self.alerts
            .iter()
            .filter(|a| include_resolved || !a.resolved)
            .map(|a| a.clone())
            .collect()
    }

    pub fn get_alert(&self, alert_id: &str) -> Option<Alert> {
        self.alerts.get(alert_id).map(|a| a.clone())
    }
}

pub struct Telemetry {
    pub metrics: MetricsCollector,
    pub alert_rule_engine: crate::observability::AlertRuleEngine,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

impl Telemetry {
    pub fn new() -> Self {
        let alert_rule_engine = crate::observability::AlertRuleEngine::new();
        Self {
            metrics: MetricsCollector::new(),
            alert_rule_engine,
            started_at: chrono::Utc::now(),
        }
    }

    pub fn uptime_seconds(&self) -> u64 {
        (chrono::Utc::now() - self.started_at).num_seconds() as u64
    }
}

impl Default for Telemetry {
    fn default() -> Self {
        Self::new()
    }
}
