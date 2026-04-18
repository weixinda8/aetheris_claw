use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
    RealTime = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealTimeTaskConfig {
    pub task_id: String,
    pub name: String,
    pub priority: TaskPriority,
    pub deadline: Option<Duration>,
    pub period: Option<Duration>,
    pub cpu_affinity: Option<Vec<usize>>,
    pub stack_size: Option<usize>,
    pub max_execution_time: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionStats {
    pub task_id: String,
    pub total_executions: u64,
    pub total_success: u64,
    pub total_failures: u64,
    pub total_deadline_misses: u64,
    pub average_execution_time_ms: f64,
    pub max_execution_time_ms: f64,
    pub p99_execution_time_ms: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SchedulerStatus {
    Idle,
    Running,
    Paused,
    Overloaded,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub max_concurrent_tasks: usize,
    pub overload_threshold: f64,
    pub default_priority: TaskPriority,
    pub enable_cpu_affinity: bool,
    pub enable_deadline_monitoring: bool,
    pub metrics_interval: Duration,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: num_cpus::get() * 2,
            overload_threshold: 0.9,
            default_priority: TaskPriority::Normal,
            enable_cpu_affinity: true,
            enable_deadline_monitoring: true,
            metrics_interval: Duration::from_secs(1),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerMetrics {
    pub status: SchedulerStatus,
    pub current_tasks: usize,
    pub total_tasks: u64,
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub overload_count: u64,
    pub deadline_misses: u64,
    pub average_latency_ms: f64,
    pub p99_latency_ms: f64,
}
