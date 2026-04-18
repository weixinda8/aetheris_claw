use crate::digital_twin::{CommandStatus, TwinCommand, TwinModel, TwinStateUpdate};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncDirection {
    PhysicalToDigital,
    DigitalToPhysical,
    Bidirectional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub direction: SyncDirection,
    pub sync_interval_ms: u64,
    pub max_latency_ms: u64,
    pub enable_cache: bool,
    pub cache_size: usize,
    pub retry_attempts: u32,
    pub retry_delay_ms: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            direction: SyncDirection::Bidirectional,
            sync_interval_ms: 50,
            max_latency_ms: 100,
            enable_cache: true,
            cache_size: 1000,
            retry_attempts: 3,
            retry_delay_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStats {
    pub total_updates: u64,
    pub successful_updates: u64,
    pub failed_updates: u64,
    pub average_latency_ms: f64,
    pub max_latency_ms: f64,
    pub last_sync_time: Option<DateTime<Utc>>,
    pub connection_status: ConnectionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Reconnecting,
    Degraded,
}

pub struct DigitalTwinSynchronizer {
    model: Arc<TwinModel>,
    config: SyncConfig,
    stats: Arc<RwLock<SyncStats>>,
    connection_status: Arc<RwLock<ConnectionStatus>>,
    pending_updates: Arc<DashMap<String, TwinStateUpdate>>,
    pending_commands: Arc<DashMap<String, TwinCommand>>,
    is_running: Arc<RwLock<bool>>,
}

impl DigitalTwinSynchronizer {
    pub fn new(model: Arc<TwinModel>, config: SyncConfig) -> Self {
        Self {
            model,
            config,
            stats: Arc::new(RwLock::new(SyncStats {
                total_updates: 0,
                successful_updates: 0,
                failed_updates: 0,
                average_latency_ms: 0.0,
                max_latency_ms: 0.0,
                last_sync_time: None,
                connection_status: ConnectionStatus::Disconnected,
            })),
            connection_status: Arc::new(RwLock::new(ConnectionStatus::Disconnected)),
            pending_updates: Arc::new(DashMap::new()),
            pending_commands: Arc::new(DashMap::new()),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return;
        }
        *is_running = true;
        drop(is_running);

        *self.connection_status.write().await = ConnectionStatus::Connected;
        tracing::info!("Digital twin synchronizer started");

        self.sync_loop().await;
    }

    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        *self.connection_status.write().await = ConnectionStatus::Disconnected;
        tracing::info!("Digital twin synchronizer stopped");
    }

    async fn sync_loop(&self) {
        let interval = Duration::from_millis(self.config.sync_interval_ms);

        loop {
            let is_running = *self.is_running.read().await;
            if !is_running {
                break;
            }

            let connection_status = self.connection_status.read().await.clone();
            if connection_status == ConnectionStatus::Connected {
                self.process_pending_updates().await;
                self.process_pending_commands().await;
            } else if connection_status == ConnectionStatus::Reconnecting {
                self.attempt_reconnect().await;
            }

            sleep(interval).await;
        }
    }

    pub async fn queue_update(&self, update: TwinStateUpdate) {
        let _start_time = Utc::now();
        let key = format!(
            "{}-{}",
            update.entity_id,
            update.timestamp.timestamp_nanos_opt().unwrap()
        );
        self.pending_updates.insert(key.clone(), update);

        let mut stats = self.stats.write().await;
        stats.total_updates += 1;
    }

    async fn process_pending_updates(&self) {
        let keys: Vec<_> = self
            .pending_updates
            .iter()
            .map(|u| u.key().clone())
            .collect();

        for key in keys {
            if let Some(update) = self.pending_updates.get(&key) {
                let start_time = Utc::now();
                let success = self.model.update_state(update.value().clone());

                let latency_ms = (Utc::now() - start_time).num_milliseconds() as f64;

                let mut stats = self.stats.write().await;
                if success {
                    stats.successful_updates += 1;
                    stats.last_sync_time = Some(Utc::now());

                    let total = stats.successful_updates as f64;
                    stats.average_latency_ms =
                        (stats.average_latency_ms * (total - 1.0) + latency_ms) / total;
                    stats.max_latency_ms = stats.max_latency_ms.max(latency_ms);

                    self.pending_updates.remove(&key);
                } else {
                    stats.failed_updates += 1;
                }
            }
        }
    }

    pub async fn queue_command(&self, command: TwinCommand) -> String {
        let command_id = command.id.clone();
        self.pending_commands.insert(command_id.clone(), command);
        command_id
    }

    async fn process_pending_commands(&self) {
        let commands: Vec<_> = self
            .pending_commands
            .iter()
            .map(|c| c.value().clone())
            .collect();

        for mut command in commands {
            if command.status == CommandStatus::Pending {
                command.status = CommandStatus::Executing;
                self.model
                    .update_command_status(&command.id, CommandStatus::Executing);

                self.execute_command(&mut command).await;

                self.pending_commands.remove(&command.id);
            }
        }
    }

    async fn execute_command(&self, command: &mut TwinCommand) {
        let mut attempt = 0;
        let max_attempts = self.config.retry_attempts;

        loop {
            attempt += 1;

            let result = self.send_to_physical_system(command).await;

            match result {
                Ok(_) => {
                    command.status = CommandStatus::Completed;
                    self.model
                        .update_command_status(&command.id, CommandStatus::Completed);
                    return;
                }
                Err(_) if attempt < max_attempts => {
                    tracing::warn!(
                        "Command {} failed, retrying (attempt {}/{})",
                        command.id,
                        attempt,
                        max_attempts
                    );
                    sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
                }
                Err(_) => {
                    command.status = CommandStatus::Failed;
                    self.model
                        .update_command_status(&command.id, CommandStatus::Failed);
                    return;
                }
            }
        }
    }

    async fn send_to_physical_system(&self, _command: &TwinCommand) -> crate::utils::Result<()> {
        tracing::debug!("Sending command to physical system: {:?}", _command);
        Ok(())
    }

    pub async fn trigger_reconnect(&self) {
        *self.connection_status.write().await = ConnectionStatus::Reconnecting;
        tracing::warn!("Connection lost, attempting reconnect");
    }

    async fn attempt_reconnect(&self) {
        tracing::debug!("Attempting to reconnect...");

        tokio::time::sleep(Duration::from_secs(1)).await;

        let mut stats = self.stats.write().await;
        stats.connection_status = ConnectionStatus::Connected;
        *self.connection_status.write().await = ConnectionStatus::Connected;
        tracing::info!("Reconnected successfully");
    }

    pub async fn get_stats(&self) -> SyncStats {
        let mut stats = self.stats.read().await.clone();
        stats.connection_status = self.connection_status.read().await.clone();
        stats
    }

    pub async fn get_connection_status(&self) -> ConnectionStatus {
        self.connection_status.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::digital_twin::{
        CommandStatus, TwinCommand, TwinEntity, TwinEntityType, TwinModel, TwinState,
        TwinStateUpdate,
    };
    use chrono::Utc;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn test_sync_config_default() {
        let config = SyncConfig::default();
        assert_eq!(config.direction, SyncDirection::Bidirectional);
        assert_eq!(config.sync_interval_ms, 50);
        assert_eq!(config.max_latency_ms, 100);
        assert!(config.enable_cache);
        assert_eq!(config.cache_size, 1000);
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.retry_delay_ms, 1000);
    }

    #[test]
    fn test_sync_stats_default() {
        let model = TwinModel::new();
        let config = SyncConfig::default();
        let synchronizer = DigitalTwinSynchronizer::new(Arc::new(model), config);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let stats = rt.block_on(synchronizer.get_stats());

        assert_eq!(stats.total_updates, 0);
        assert_eq!(stats.successful_updates, 0);
        assert_eq!(stats.failed_updates, 0);
        assert_eq!(stats.connection_status, ConnectionStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_synchronizer_new() {
        let model = TwinModel::new();
        let config = SyncConfig::default();
        let synchronizer = DigitalTwinSynchronizer::new(Arc::new(model), config);

        let status = synchronizer.get_connection_status().await;
        assert_eq!(status, ConnectionStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_queue_update() {
        let model = TwinModel::new();
        let config = SyncConfig::default();
        let synchronizer = DigitalTwinSynchronizer::new(Arc::new(model), config);

        let entity = TwinEntity {
            id: "test-entity".to_string(),
            name: "Test Entity".to_string(),
            entity_type: TwinEntityType::Device,
            properties: HashMap::new(),
            state: TwinState::Online,
            parent_id: None,
            children_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        synchronizer.model.add_entity(entity);

        let update = TwinStateUpdate {
            entity_id: "test-entity".to_string(),
            state: TwinState::Degraded,
            properties: None,
            timestamp: Utc::now(),
            source: "test".to_string(),
        };

        synchronizer.queue_update(update).await;

        let stats = synchronizer.get_stats().await;
        assert_eq!(stats.total_updates, 1);
    }

    #[tokio::test]
    async fn test_queue_command() {
        let model = TwinModel::new();
        let config = SyncConfig::default();
        let synchronizer = DigitalTwinSynchronizer::new(Arc::new(model), config);

        let command = TwinCommand {
            id: "cmd-1".to_string(),
            target_entity_id: "entity-1".to_string(),
            command_type: "start".to_string(),
            parameters: HashMap::new(),
            issued_at: Utc::now(),
            timeout_seconds: None,
            status: CommandStatus::Pending,
        };

        let command_id = synchronizer.queue_command(command).await;
        assert_eq!(command_id, "cmd-1");
    }

    #[tokio::test]
    async fn test_trigger_reconnect() {
        let model = TwinModel::new();
        let config = SyncConfig::default();
        let synchronizer = DigitalTwinSynchronizer::new(Arc::new(model), config);

        synchronizer.trigger_reconnect().await;

        let status = synchronizer.get_connection_status().await;
        assert_eq!(status, ConnectionStatus::Reconnecting);
    }

    #[tokio::test]
    async fn test_start_and_stop() {
        let model = TwinModel::new();
        let config = SyncConfig::default();
        let synchronizer = DigitalTwinSynchronizer::new(Arc::new(model), config);

        synchronizer.start().await;
        let status = synchronizer.get_connection_status().await;
        assert_eq!(status, ConnectionStatus::Connected);

        synchronizer.stop().await;
        let status = synchronizer.get_connection_status().await;
        assert_eq!(status, ConnectionStatus::Disconnected);
    }
}
