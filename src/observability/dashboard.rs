use crate::observability::Telemetry;
use crate::observability::telemetry::SystemMetrics;

pub struct Dashboard {
    telemetry: Telemetry,
}

impl Dashboard {
    pub fn new(telemetry: Telemetry) -> Self {
        Self { telemetry }
    }

    pub fn get_status(&self, active_agents: usize) -> DashboardStatus {
        let system_metrics = self
            .telemetry
            .metrics
            .get_system_metrics(active_agents, self.telemetry.uptime_seconds());

        DashboardStatus {
            total_tasks: system_metrics.total_tasks,
            completed_tasks: system_metrics.completed_tasks,
            failed_tasks: system_metrics.failed_tasks,
            success_rate: system_metrics.success_rate,
            uptime_seconds: system_metrics.uptime_seconds,
            active_agents: system_metrics.active_agents,
            timestamp: system_metrics.timestamp,
        }
    }

    pub fn get_system_metrics(&self, active_agents: usize) -> SystemMetrics {
        self.telemetry
            .metrics
            .get_system_metrics(active_agents, self.telemetry.uptime_seconds())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DashboardStatus {
    pub total_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub success_rate: f64,
    pub uptime_seconds: u64,
    pub active_agents: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
