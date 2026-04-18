use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub timestamp: DateTime<Utc>,
    pub memory_mb: f64,
    pub cpu_percent: f64,
    pub disk_usage_percent: Option<f64>,
    pub network_in_bytes: Option<u64>,
    pub network_out_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DegradationLevel {
    None,
    Light,
    Medium,
    Severe,
}

pub struct ResourceMonitor {
    usage_history: Arc<RwLock<Vec<ResourceUsage>>>,
    max_history: usize,
    max_memory_mb: f64,
    max_cpu_percent: f64,
    current_degradation: Arc<RwLock<DegradationLevel>>,
    auto_downgrade_enabled: bool,
}

impl ResourceMonitor {
    pub fn new(max_memory_mb: f64, max_cpu_percent: f64, auto_downgrade_enabled: bool) -> Self {
        Self {
            usage_history: Arc::new(RwLock::new(Vec::new())),
            max_history: 100,
            max_memory_mb,
            max_cpu_percent,
            current_degradation: Arc::new(RwLock::new(DegradationLevel::None)),
            auto_downgrade_enabled,
        }
    }

    pub async fn record_usage(&self, usage: ResourceUsage) {
        let mut history = self.usage_history.write().await;
        history.push(usage);

        if history.len() > self.max_history {
            history.remove(0);
        }

        if self.auto_downgrade_enabled {
            self.check_and_degrade().await;
        }
    }

    pub async fn get_current_usage(&self) -> Option<ResourceUsage> {
        let history = self.usage_history.read().await;
        history.last().cloned()
    }

    pub async fn get_average_usage(&self, window_seconds: u64) -> Option<ResourceUsage> {
        let history = self.usage_history.read().await;
        let now = Utc::now();
        let window_start = now - chrono::Duration::seconds(window_seconds as i64);

        let recent: Vec<_> = history
            .iter()
            .filter(|u| u.timestamp >= window_start)
            .collect();

        if recent.is_empty() {
            return None;
        }

        let count = recent.len() as f64;
        Some(ResourceUsage {
            timestamp: now,
            memory_mb: recent.iter().map(|u| u.memory_mb).sum::<f64>() / count,
            cpu_percent: recent.iter().map(|u| u.cpu_percent).sum::<f64>() / count,
            disk_usage_percent: None,
            network_in_bytes: None,
            network_out_bytes: None,
        })
    }

    pub async fn get_degradation_level(&self) -> DegradationLevel {
        self.current_degradation.read().await.clone()
    }

    async fn check_and_degrade(&self) {
        let Some(usage) = self.get_average_usage(60).await else {
            return;
        };

        let memory_ratio = usage.memory_mb / self.max_memory_mb;
        let cpu_ratio = usage.cpu_percent / self.max_cpu_percent;

        let new_level = if memory_ratio > 0.95 || cpu_ratio > 0.95 {
            DegradationLevel::Severe
        } else if memory_ratio > 0.85 || cpu_ratio > 0.85 {
            DegradationLevel::Medium
        } else if memory_ratio > 0.7 || cpu_ratio > 0.7 {
            DegradationLevel::Light
        } else {
            DegradationLevel::None
        };

        let mut current = self.current_degradation.write().await;
        if *current != new_level {
            tracing::warn!(
                "Resource degradation level changed: {:?} -> {:?}",
                current,
                new_level
            );
            *current = new_level;
        }
    }

    pub async fn get_usage_history(&self) -> Vec<ResourceUsage> {
        self.usage_history.read().await.clone()
    }

    pub async fn get_resource_alerts(&self) -> Vec<ResourceAlert> {
        let mut alerts = Vec::new();
        let Some(usage) = self.get_current_usage().await else {
            return alerts;
        };

        if usage.memory_mb > self.max_memory_mb * 0.9 {
            alerts.push(ResourceAlert {
                severity: ResourceAlertSeverity::Critical,
                message: format!(
                    "Memory usage critical: {:.1}MB / {:.1}MB",
                    usage.memory_mb, self.max_memory_mb
                ),
            });
        } else if usage.memory_mb > self.max_memory_mb * 0.75 {
            alerts.push(ResourceAlert {
                severity: ResourceAlertSeverity::Warning,
                message: format!(
                    "Memory usage high: {:.1}MB / {:.1}MB",
                    usage.memory_mb, self.max_memory_mb
                ),
            });
        }

        if usage.cpu_percent > self.max_cpu_percent * 0.9 {
            alerts.push(ResourceAlert {
                severity: ResourceAlertSeverity::Critical,
                message: format!(
                    "CPU usage critical: {:.1}% / {:.1}%",
                    usage.cpu_percent, self.max_cpu_percent
                ),
            });
        } else if usage.cpu_percent > self.max_cpu_percent * 0.75 {
            alerts.push(ResourceAlert {
                severity: ResourceAlertSeverity::Warning,
                message: format!(
                    "CPU usage high: {:.1}% / {:.1}%",
                    usage.cpu_percent, self.max_cpu_percent
                ),
            });
        }

        alerts
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAlert {
    pub severity: ResourceAlertSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceAlertSeverity {
    Info,
    Warning,
    Critical,
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new(256.0, 100.0, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_monitor_new() {
        let monitor = ResourceMonitor::new(512.0, 80.0, false);
        assert_eq!(monitor.get_degradation_level(), DegradationLevel::None);
    }

    #[test]
    fn test_resource_monitor_default() {
        let monitor = ResourceMonitor::default();
        assert_eq!(monitor.get_degradation_level(), DegradationLevel::None);
    }

    #[tokio::test]
    async fn test_record_usage() {
        let monitor = ResourceMonitor::new(256.0, 100.0, false);

        let usage = ResourceUsage {
            timestamp: Utc::now(),
            memory_mb: 100.0,
            cpu_percent: 30.0,
            disk_usage_percent: Some(50.0),
            network_in_bytes: Some(1000),
            network_out_bytes: Some(2000),
        };

        monitor.record_usage(usage).await;

        let current = monitor.get_current_usage().await;
        assert!(current.is_some());
        let current = current.unwrap();
        assert_eq!(current.memory_mb, 100.0);
        assert_eq!(current.cpu_percent, 30.0);
    }

    #[tokio::test]
    async fn test_get_average_usage() {
        let monitor = ResourceMonitor::new(256.0, 100.0, false);

        let now = Utc::now();
        let usage1 = ResourceUsage {
            timestamp: now - chrono::Duration::seconds(30),
            memory_mb: 100.0,
            cpu_percent: 40.0,
            disk_usage_percent: None,
            network_in_bytes: None,
            network_out_bytes: None,
        };

        let usage2 = ResourceUsage {
            timestamp: now - chrono::Duration::seconds(10),
            memory_mb: 200.0,
            cpu_percent: 60.0,
            disk_usage_percent: None,
            network_in_bytes: None,
            network_out_bytes: None,
        };

        monitor.record_usage(usage1).await;
        monitor.record_usage(usage2).await;

        let average = monitor.get_average_usage(60).await;
        assert!(average.is_some());
        let average = average.unwrap();
        assert!((average.memory_mb - 150.0).abs() < 0.001);
        assert!((average.cpu_percent - 50.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_get_average_usage_empty() {
        let monitor = ResourceMonitor::new(256.0, 100.0, false);
        let average = monitor.get_average_usage(60).await;
        assert!(average.is_none());
    }

    #[tokio::test]
    async fn test_degradation_none() {
        let monitor = ResourceMonitor::new(256.0, 100.0, true);

        let usage = ResourceUsage {
            timestamp: Utc::now(),
            memory_mb: 100.0,
            cpu_percent: 50.0,
            disk_usage_percent: None,
            network_in_bytes: None,
            network_out_bytes: None,
        };

        monitor.record_usage(usage.clone()).await;
        monitor.record_usage(usage.clone()).await;
        monitor.record_usage(usage).await;

        let level = monitor.get_degradation_level().await;
        assert_eq!(level, DegradationLevel::None);
    }

    #[tokio::test]
    async fn test_degradation_light() {
        let monitor = ResourceMonitor::new(256.0, 100.0, true);

        let usage = ResourceUsage {
            timestamp: Utc::now(),
            memory_mb: 200.0,
            cpu_percent: 75.0,
            disk_usage_percent: None,
            network_in_bytes: None,
            network_out_bytes: None,
        };

        monitor.record_usage(usage.clone()).await;
        monitor.record_usage(usage.clone()).await;
        monitor.record_usage(usage).await;

        let level = monitor.get_degradation_level().await;
        assert_eq!(level, DegradationLevel::Light);
    }

    #[tokio::test]
    async fn test_degradation_medium() {
        let monitor = ResourceMonitor::new(256.0, 100.0, true);

        let usage = ResourceUsage {
            timestamp: Utc::now(),
            memory_mb: 225.0,
            cpu_percent: 90.0,
            disk_usage_percent: None,
            network_in_bytes: None,
            network_out_bytes: None,
        };

        monitor.record_usage(usage.clone()).await;
        monitor.record_usage(usage.clone()).await;
        monitor.record_usage(usage).await;

        let level = monitor.get_degradation_level().await;
        assert_eq!(level, DegradationLevel::Medium);
    }

    #[tokio::test]
    async fn test_degradation_severe() {
        let monitor = ResourceMonitor::new(256.0, 100.0, true);

        let usage = ResourceUsage {
            timestamp: Utc::now(),
            memory_mb: 250.0,
            cpu_percent: 98.0,
            disk_usage_percent: None,
            network_in_bytes: None,
            network_out_bytes: None,
        };

        monitor.record_usage(usage.clone()).await;
        monitor.record_usage(usage.clone()).await;
        monitor.record_usage(usage).await;

        let level = monitor.get_degradation_level().await;
        assert_eq!(level, DegradationLevel::Severe);
    }

    #[tokio::test]
    async fn test_get_usage_history() {
        let monitor = ResourceMonitor::new(256.0, 100.0, false);

        for i in 0..5 {
            let usage = ResourceUsage {
                timestamp: Utc::now(),
                memory_mb: 100.0 + i as f64 * 10.0,
                cpu_percent: 30.0 + i as f64 * 5.0,
                disk_usage_percent: None,
                network_in_bytes: None,
                network_out_bytes: None,
            };
            monitor.record_usage(usage).await;
        }

        let history = monitor.get_usage_history().await;
        assert_eq!(history.len(), 5);
    }

    #[tokio::test]
    async fn test_get_resource_alerts_warning() {
        let monitor = ResourceMonitor::new(256.0, 100.0, false);

        let usage = ResourceUsage {
            timestamp: Utc::now(),
            memory_mb: 200.0,
            cpu_percent: 80.0,
            disk_usage_percent: None,
            network_in_bytes: None,
            network_out_bytes: None,
        };

        monitor.record_usage(usage).await;

        let alerts = monitor.get_resource_alerts().await;
        assert!(!alerts.is_empty());
        assert!(
            alerts
                .iter()
                .any(|a| a.severity == ResourceAlertSeverity::Warning)
        );
    }

    #[tokio::test]
    async fn test_get_resource_alerts_critical() {
        let monitor = ResourceMonitor::new(256.0, 100.0, false);

        let usage = ResourceUsage {
            timestamp: Utc::now(),
            memory_mb: 240.0,
            cpu_percent: 95.0,
            disk_usage_percent: None,
            network_in_bytes: None,
            network_out_bytes: None,
        };

        monitor.record_usage(usage).await;

        let alerts = monitor.get_resource_alerts().await;
        assert!(!alerts.is_empty());
        assert!(
            alerts
                .iter()
                .any(|a| a.severity == ResourceAlertSeverity::Critical)
        );
    }

    #[tokio::test]
    async fn test_get_resource_alerts_no_alerts() {
        let monitor = ResourceMonitor::new(256.0, 100.0, false);

        let usage = ResourceUsage {
            timestamp: Utc::now(),
            memory_mb: 100.0,
            cpu_percent: 50.0,
            disk_usage_percent: None,
            network_in_bytes: None,
            network_out_bytes: None,
        };

        monitor.record_usage(usage).await;

        let alerts = monitor.get_resource_alerts().await;
        assert!(alerts.is_empty());
    }

    #[tokio::test]
    async fn test_get_resource_alerts_empty() {
        let monitor = ResourceMonitor::new(256.0, 100.0, false);
        let alerts = monitor.get_resource_alerts().await;
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_degradation_level_equality() {
        assert_eq!(DegradationLevel::None, DegradationLevel::None);
        assert_eq!(DegradationLevel::Light, DegradationLevel::Light);
        assert_eq!(DegradationLevel::Medium, DegradationLevel::Medium);
        assert_eq!(DegradationLevel::Severe, DegradationLevel::Severe);
    }

    #[test]
    fn test_resource_alert_severity_equality() {
        assert_eq!(ResourceAlertSeverity::Info, ResourceAlertSeverity::Info);
        assert_eq!(
            ResourceAlertSeverity::Warning,
            ResourceAlertSeverity::Warning
        );
        assert_eq!(
            ResourceAlertSeverity::Critical,
            ResourceAlertSeverity::Critical
        );
    }
}
