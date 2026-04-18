use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectionStatus {
    Online,
    Offline,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineDataRecord {
    pub id: String,
    pub data_type: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub synced: bool,
}

pub struct OfflineModeManager {
    status: Arc<RwLock<ConnectionStatus>>,
    pending_sync: Arc<DashMap<String, OfflineDataRecord>>,
    last_online_time: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl OfflineModeManager {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(ConnectionStatus::Online)),
            pending_sync: Arc::new(DashMap::new()),
            last_online_time: Arc::new(RwLock::new(Some(Utc::now()))),
        }
    }

    pub async fn get_status(&self) -> ConnectionStatus {
        self.status.read().await.clone()
    }

    pub async fn set_status(&self, status: ConnectionStatus) {
        let mut current = self.status.write().await;
        if *current == ConnectionStatus::Online && status != ConnectionStatus::Online {
            tracing::warn!("Connection going offline");
        } else if *current != ConnectionStatus::Online && status == ConnectionStatus::Online {
            tracing::info!("Connection restored to online");
            *self.last_online_time.write().await = Some(Utc::now());
        }
        *current = status;
    }

    pub async fn is_offline(&self) -> bool {
        matches!(self.get_status().await, ConnectionStatus::Offline)
    }

    pub async fn queue_for_sync(&self, data_type: String, payload: serde_json::Value) -> String {
        let record = OfflineDataRecord {
            id: uuid::Uuid::new_v4().to_string(),
            data_type,
            payload,
            timestamp: Utc::now(),
            synced: false,
        };
        let id = record.id.clone();
        self.pending_sync.insert(id.clone(), record);
        id
    }

    pub async fn get_pending_sync(&self) -> Vec<OfflineDataRecord> {
        self.pending_sync
            .iter()
            .map(|r| r.value().clone())
            .filter(|r| !r.synced)
            .collect()
    }

    pub async fn mark_synced(&self, id: &str) {
        if let Some(mut record) = self.pending_sync.get_mut(id) {
            record.synced = true;
        }
    }

    pub async fn clear_synced(&self) {
        self.pending_sync.retain(|_, r| !r.synced);
    }

    pub async fn get_last_online_time(&self) -> Option<DateTime<Utc>> {
        *self.last_online_time.read().await
    }
}

impl Default for OfflineModeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_offline_mode_manager_new() {
        let manager = OfflineModeManager::new();
        assert_eq!(manager.get_status().await, ConnectionStatus::Online);
        assert!(manager.get_pending_sync().await.is_empty());
    }

    #[tokio::test]
    async fn test_offline_mode_manager_default() {
        let manager = OfflineModeManager::default();
        assert_eq!(manager.get_status().await, ConnectionStatus::Online);
    }

    #[tokio::test]
    async fn test_set_status_online_to_offline() {
        let manager = OfflineModeManager::new();
        manager.set_status(ConnectionStatus::Offline).await;
        assert_eq!(manager.get_status().await, ConnectionStatus::Offline);
        assert!(manager.is_offline().await);
    }

    #[tokio::test]
    async fn test_set_status_offline_to_online() {
        let manager = OfflineModeManager::new();
        manager.set_status(ConnectionStatus::Offline).await;
        let before_time = manager.get_last_online_time().await;

        manager.set_status(ConnectionStatus::Online).await;
        let after_time = manager.get_last_online_time().await;

        assert_eq!(manager.get_status().await, ConnectionStatus::Online);
        assert!(!manager.is_offline().await);
        assert_ne!(before_time, after_time);
    }

    #[tokio::test]
    async fn test_set_status_degraded() {
        let manager = OfflineModeManager::new();
        manager.set_status(ConnectionStatus::Degraded).await;
        assert_eq!(manager.get_status().await, ConnectionStatus::Degraded);
    }

    #[tokio::test]
    async fn test_queue_for_sync() {
        let manager = OfflineModeManager::new();
        let payload = json!({"key": "value"});

        let id = manager
            .queue_for_sync("test-type".to_string(), payload.clone())
            .await;

        assert!(!id.is_empty());
        let pending = manager.get_pending_sync().await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].data_type, "test-type");
        assert_eq!(pending[0].payload, payload);
    }

    #[tokio::test]
    async fn test_multiple_queue_for_sync() {
        let manager = OfflineModeManager::new();

        let id1 = manager
            .queue_for_sync("type1".to_string(), json!({"data": 1}))
            .await;
        let id2 = manager
            .queue_for_sync("type2".to_string(), json!({"data": 2}))
            .await;

        let pending = manager.get_pending_sync().await;
        assert_eq!(pending.len(), 2);
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_mark_synced() {
        let manager = OfflineModeManager::new();

        let id = manager
            .queue_for_sync("test".to_string(), json!({"key": "value"}))
            .await;

        let pending_before = manager.get_pending_sync().await;
        assert_eq!(pending_before.len(), 1);
        assert!(!pending_before[0].synced);

        manager.mark_synced(&id).await;

        let pending_after = manager.get_pending_sync().await;
        assert!(pending_after.is_empty());
    }

    #[tokio::test]
    async fn test_clear_synced() {
        let manager = OfflineModeManager::new();

        let id1 = manager
            .queue_for_sync("type1".to_string(), json!({"data": 1}))
            .await;
        let id2 = manager
            .queue_for_sync("type2".to_string(), json!({"data": 2}))
            .await;

        manager.mark_synced(&id1).await;
        manager.clear_synced().await;

        let pending = manager.get_pending_sync().await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].data_type, "type2");
    }

    #[tokio::test]
    async fn test_get_pending_sync_filtered() {
        let manager = OfflineModeManager::new();

        let id1 = manager
            .queue_for_sync("type1".to_string(), json!({"data": 1}))
            .await;
        let id2 = manager
            .queue_for_sync("type2".to_string(), json!({"data": 2}))
            .await;

        manager.mark_synced(&id1).await;

        let pending = manager.get_pending_sync().await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id2);
    }

    #[tokio::test]
    async fn test_mark_nonexistent_synced() {
        let manager = OfflineModeManager::new();
        manager
            .queue_for_sync("test".to_string(), json!({"key": "value"}))
            .await;

        manager.mark_synced("nonexistent-id").await;

        let pending = manager.get_pending_sync().await;
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn test_connection_status_equality() {
        let status1 = ConnectionStatus::Online;
        let status2 = ConnectionStatus::Online;
        let status3 = ConnectionStatus::Offline;

        assert_eq!(status1, status2);
        assert_ne!(status1, status3);
    }
}
