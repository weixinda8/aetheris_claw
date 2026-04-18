use crate::edge_build::rules::AlertLevel;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAlert {
    pub id: String,
    pub level: AlertLevel,
    pub title: String,
    pub message: String,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
    pub resolved: bool,
    pub synced_to_cloud: bool,
}

pub struct LocalAlertManager {
    alerts: Arc<DashMap<String, LocalAlert>>,
    notification_queue: Arc<RwLock<Vec<LocalAlert>>>,
}

impl LocalAlertManager {
    pub fn new() -> Self {
        Self {
            alerts: Arc::new(DashMap::new()),
            notification_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn create_alert(
        &self,
        level: AlertLevel,
        title: String,
        message: String,
        source: String,
    ) -> String {
        let alert = LocalAlert {
            id: uuid::Uuid::new_v4().to_string(),
            level,
            title,
            message,
            source,
            timestamp: Utc::now(),
            acknowledged: false,
            resolved: false,
            synced_to_cloud: false,
        };
        let id = alert.id.clone();

        self.log_alert(&alert);
        self.alerts.insert(id.clone(), alert.clone());
        self.notification_queue.write().await.push(alert);

        id
    }

    fn log_alert(&self, alert: &LocalAlert) {
        match alert.level {
            AlertLevel::Info => tracing::info!("[Alert] {}: {}", alert.title, alert.message),
            AlertLevel::Warning => tracing::warn!("[Alert] {}: {}", alert.title, alert.message),
            AlertLevel::Error => tracing::error!("[Alert] {}: {}", alert.title, alert.message),
            AlertLevel::Critical => {
                tracing::error!("[CRITICAL Alert] {}: {}", alert.title, alert.message)
            }
        }
    }

    pub fn get_alert(&self, alert_id: &str) -> Option<LocalAlert> {
        self.alerts.get(alert_id).map(|a| a.value().clone())
    }

    pub fn list_alerts(&self, include_resolved: bool) -> Vec<LocalAlert> {
        self.alerts
            .iter()
            .map(|a| a.value().clone())
            .filter(|a| include_resolved || !a.resolved)
            .collect()
    }

    pub async fn acknowledge_alert(&self, alert_id: &str) -> bool {
        if let Some(mut alert) = self.alerts.get_mut(alert_id) {
            alert.acknowledged = true;
            true
        } else {
            false
        }
    }

    pub async fn resolve_alert(&self, alert_id: &str) -> bool {
        if let Some(mut alert) = self.alerts.get_mut(alert_id) {
            alert.resolved = true;
            true
        } else {
            false
        }
    }

    pub async fn mark_synced(&self, alert_id: &str) -> bool {
        if let Some(mut alert) = self.alerts.get_mut(alert_id) {
            alert.synced_to_cloud = true;
            true
        } else {
            false
        }
    }

    pub async fn get_unsynced_alerts(&self) -> Vec<LocalAlert> {
        self.alerts
            .iter()
            .map(|a| a.value().clone())
            .filter(|a| !a.synced_to_cloud)
            .collect()
    }

    pub async fn get_pending_notifications(&self) -> Vec<LocalAlert> {
        self.notification_queue.read().await.clone()
    }

    pub async fn clear_notifications(&self) {
        self.notification_queue.write().await.clear();
    }

    pub async fn get_stats(&self) -> AlertStats {
        let alerts: Vec<_> = self.alerts.iter().map(|a| a.value().clone()).collect();
        AlertStats {
            total: alerts.len(),
            unresolved: alerts.iter().filter(|a| !a.resolved).count(),
            acknowledged: alerts.iter().filter(|a| a.acknowledged).count(),
            critical: alerts
                .iter()
                .filter(|a| a.level == AlertLevel::Critical && !a.resolved)
                .count(),
            error: alerts
                .iter()
                .filter(|a| a.level == AlertLevel::Error && !a.resolved)
                .count(),
            warning: alerts
                .iter()
                .filter(|a| a.level == AlertLevel::Warning && !a.resolved)
                .count(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertStats {
    pub total: usize,
    pub unresolved: usize,
    pub acknowledged: usize,
    pub critical: usize,
    pub error: usize,
    pub warning: usize,
}

impl Default for LocalAlertManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge_build::rules::AlertLevel;

    #[test]
    fn test_local_alert_manager_new() {
        let manager = LocalAlertManager::new();
        assert!(manager.list_alerts(false).is_empty());
    }

    #[test]
    fn test_local_alert_manager_default() {
        let manager = LocalAlertManager::default();
        assert!(manager.list_alerts(false).is_empty());
    }

    #[tokio::test]
    async fn test_create_alert() {
        let manager = LocalAlertManager::new();

        let alert_id = manager
            .create_alert(
                AlertLevel::Warning,
                "Test Alert".to_string(),
                "Test Message".to_string(),
                "test-source".to_string(),
            )
            .await;

        assert!(!alert_id.is_empty());

        let alert = manager.get_alert(&alert_id);
        assert!(alert.is_some());
        let alert = alert.unwrap();
        assert_eq!(alert.title, "Test Alert");
        assert_eq!(alert.message, "Test Message");
        assert_eq!(alert.source, "test-source");
        assert_eq!(alert.level, AlertLevel::Warning);
        assert!(!alert.acknowledged);
        assert!(!alert.resolved);
        assert!(!alert.synced_to_cloud);
    }

    #[tokio::test]
    async fn test_list_alerts() {
        let manager = LocalAlertManager::new();

        let id1 = manager
            .create_alert(
                AlertLevel::Info,
                "Alert 1".to_string(),
                "Message 1".to_string(),
                "source-1".to_string(),
            )
            .await;

        let id2 = manager
            .create_alert(
                AlertLevel::Error,
                "Alert 2".to_string(),
                "Message 2".to_string(),
                "source-2".to_string(),
            )
            .await;

        manager.resolve_alert(&id2).await;

        let all_alerts = manager.list_alerts(true);
        assert_eq!(all_alerts.len(), 2);

        let unresolved_alerts = manager.list_alerts(false);
        assert_eq!(unresolved_alerts.len(), 1);
        assert_eq!(unresolved_alerts[0].id, id1);
    }

    #[tokio::test]
    async fn test_acknowledge_alert() {
        let manager = LocalAlertManager::new();

        let alert_id = manager
            .create_alert(
                AlertLevel::Critical,
                "Critical Alert".to_string(),
                "Critical Message".to_string(),
                "test".to_string(),
            )
            .await;

        let alert = manager.get_alert(&alert_id).unwrap();
        assert!(!alert.acknowledged);

        let result = manager.acknowledge_alert(&alert_id).await;
        assert!(result);

        let alert = manager.get_alert(&alert_id).unwrap();
        assert!(alert.acknowledged);
    }

    #[tokio::test]
    async fn test_resolve_alert() {
        let manager = LocalAlertManager::new();

        let alert_id = manager
            .create_alert(
                AlertLevel::Error,
                "Error Alert".to_string(),
                "Error Message".to_string(),
                "test".to_string(),
            )
            .await;

        let alert = manager.get_alert(&alert_id).unwrap();
        assert!(!alert.resolved);

        let result = manager.resolve_alert(&alert_id).await;
        assert!(result);

        let alert = manager.get_alert(&alert_id).unwrap();
        assert!(alert.resolved);
    }

    #[tokio::test]
    async fn test_mark_synced() {
        let manager = LocalAlertManager::new();

        let alert_id = manager
            .create_alert(
                AlertLevel::Warning,
                "Warning Alert".to_string(),
                "Warning Message".to_string(),
                "test".to_string(),
            )
            .await;

        let alert = manager.get_alert(&alert_id).unwrap();
        assert!(!alert.synced_to_cloud);

        let result = manager.mark_synced(&alert_id).await;
        assert!(result);

        let alert = manager.get_alert(&alert_id).unwrap();
        assert!(alert.synced_to_cloud);
    }

    #[tokio::test]
    async fn test_get_unsynced_alerts() {
        let manager = LocalAlertManager::new();

        let id1 = manager
            .create_alert(
                AlertLevel::Info,
                "Alert 1".to_string(),
                "Message 1".to_string(),
                "source-1".to_string(),
            )
            .await;

        let id2 = manager
            .create_alert(
                AlertLevel::Warning,
                "Alert 2".to_string(),
                "Message 2".to_string(),
                "source-2".to_string(),
            )
            .await;

        manager.mark_synced(&id1).await;

        let unsynced = manager.get_unsynced_alerts().await;
        assert_eq!(unsynced.len(), 1);
        assert_eq!(unsynced[0].id, id2);
    }

    #[tokio::test]
    async fn test_pending_notifications() {
        let manager = LocalAlertManager::new();

        let id1 = manager
            .create_alert(
                AlertLevel::Info,
                "Alert 1".to_string(),
                "Message 1".to_string(),
                "source-1".to_string(),
            )
            .await;

        let id2 = manager
            .create_alert(
                AlertLevel::Warning,
                "Alert 2".to_string(),
                "Message 2".to_string(),
                "source-2".to_string(),
            )
            .await;

        let pending = manager.get_pending_notifications().await;
        assert_eq!(pending.len(), 2);

        manager.clear_notifications().await;
        let pending = manager.get_pending_notifications().await;
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let manager = LocalAlertManager::new();

        manager
            .create_alert(
                AlertLevel::Critical,
                "Critical Alert".to_string(),
                "Critical Message".to_string(),
                "test".to_string(),
            )
            .await;

        manager
            .create_alert(
                AlertLevel::Error,
                "Error Alert".to_string(),
                "Error Message".to_string(),
                "test".to_string(),
            )
            .await;

        let warning_id = manager
            .create_alert(
                AlertLevel::Warning,
                "Warning Alert".to_string(),
                "Warning Message".to_string(),
                "test".to_string(),
            )
            .await;

        manager
            .create_alert(
                AlertLevel::Info,
                "Info Alert".to_string(),
                "Info Message".to_string(),
                "test".to_string(),
            )
            .await;

        manager.acknowledge_alert(&warning_id).await;
        manager.resolve_alert(&warning_id).await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.total, 4);
        assert_eq!(stats.unresolved, 3);
        assert_eq!(stats.acknowledged, 1);
        assert_eq!(stats.critical, 1);
        assert_eq!(stats.error, 1);
        assert_eq!(stats.warning, 0);
    }

    #[tokio::test]
    async fn test_acknowledge_nonexistent_alert() {
        let manager = LocalAlertManager::new();
        let result = manager.acknowledge_alert("nonexistent-id").await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_resolve_nonexistent_alert() {
        let manager = LocalAlertManager::new();
        let result = manager.resolve_alert("nonexistent-id").await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_mark_synced_nonexistent_alert() {
        let manager = LocalAlertManager::new();
        let result = manager.mark_synced("nonexistent-id").await;
        assert!(!result);
    }
}
