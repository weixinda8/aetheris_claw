use crate::protocol::industrial::types::*;
use crate::utils::Result;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tokio::time::interval;

pub struct ConnectionManager {
    config: IndustrialProtocolConfig,
    status: Arc<RwLock<ConnectionStatus>>,
    reconnect_attempts: AtomicU32,
    last_heartbeat: Arc<Mutex<Option<chrono::DateTime<chrono::Utc>>>>,
    status_tx: broadcast::Sender<ConnectionStatus>,
    shutdown_flag: Arc<AtomicU32>,
}

impl ConnectionManager {
    pub fn new(config: IndustrialProtocolConfig) -> Self {
        let (status_tx, _) = broadcast::channel(100);
        Self {
            config,
            status: Arc::new(RwLock::new(ConnectionStatus::Disconnected)),
            reconnect_attempts: AtomicU32::new(0),
            last_heartbeat: Arc::new(Mutex::new(None)),
            status_tx,
            shutdown_flag: Arc::new(AtomicU32::new(0)),
        }
    }

    pub async fn set_status(&self, new_status: ConnectionStatus) {
        let mut status = self.status.write().await;
        let old_status = *status;
        if old_status != new_status {
            *status = new_status;
            let _ = self.status_tx.send(new_status);
        }
    }

    pub async fn get_status(&self) -> ConnectionStatus {
        *self.status.read().await
    }

    pub fn subscribe_status(&self) -> broadcast::Receiver<ConnectionStatus> {
        self.status_tx.subscribe()
    }

    pub fn increment_reconnect_attempts(&self) -> u32 {
        self.reconnect_attempts.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn reset_reconnect_attempts(&self) {
        self.reconnect_attempts.store(0, Ordering::SeqCst);
    }

    pub fn get_reconnect_attempts(&self) -> u32 {
        self.reconnect_attempts.load(Ordering::SeqCst)
    }

    pub fn can_reconnect(&self) -> bool {
        let attempts = self.get_reconnect_attempts();
        attempts < self.config.max_reconnect_attempts
    }

    pub fn update_heartbeat(&self) {
        let mut heartbeat = self.last_heartbeat.lock().unwrap();
        *heartbeat = Some(chrono::Utc::now());
    }

    pub fn get_last_heartbeat(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        *self.last_heartbeat.lock().unwrap()
    }

    pub fn is_heartbeat_stale(&self, timeout_ms: u64) -> bool {
        if let Some(last) = self.get_last_heartbeat() {
            let elapsed = (chrono::Utc::now() - last).num_milliseconds();
            elapsed > timeout_ms as i64
        } else {
            true
        }
    }

    pub fn get_reconnect_interval(&self) -> Duration {
        Duration::from_millis(self.config.reconnect_interval_ms)
    }

    pub fn get_timeout(&self) -> Duration {
        Duration::from_millis(self.config.timeout_ms)
    }

    pub fn get_config(&self) -> &IndustrialProtocolConfig {
        &self.config
    }

    pub fn shutdown(&self) {
        self.shutdown_flag.store(1, Ordering::SeqCst);
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst) == 1
    }

    pub fn is_connected(&self) -> bool {
        matches!(*self.status.blocking_read(), ConnectionStatus::Connected)
    }
}

pub struct HealthChecker {
    connection_manager: Arc<ConnectionManager>,
    check_interval_ms: u64,
    heartbeat_timeout_ms: u64,
}

impl HealthChecker {
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        check_interval_ms: u64,
        heartbeat_timeout_ms: u64,
    ) -> Self {
        Self {
            connection_manager,
            check_interval_ms,
            heartbeat_timeout_ms,
        }
    }

    pub async fn start<F, Fut>(self, health_check_fn: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = bool> + Send,
    {
        let mut interval = interval(Duration::from_millis(self.check_interval_ms));

        while !self.connection_manager.is_shutdown() {
            interval.tick().await;

            if self.connection_manager.is_connected() {
                let is_healthy = health_check_fn().await;

                if is_healthy {
                    self.connection_manager.update_heartbeat();
                } else {
                    if self
                        .connection_manager
                        .is_heartbeat_stale(self.heartbeat_timeout_ms)
                    {
                        self.connection_manager
                            .set_status(ConnectionStatus::Error)
                            .await;
                    }
                }
            }
        }
    }
}

pub struct AutoReconnector {
    connection_manager: Arc<ConnectionManager>,
}

impl AutoReconnector {
    pub fn new(connection_manager: Arc<ConnectionManager>) -> Self {
        Self { connection_manager }
    }

    pub async fn start<F, Fut>(self, reconnect_fn: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        let mut status_rx = self.connection_manager.subscribe_status();

        while !self.connection_manager.is_shutdown() {
            tokio::select! {
                Ok(status) = status_rx.recv() => {
                    match status {
                        ConnectionStatus::Disconnected | ConnectionStatus::Error => {
                            if self.connection_manager.can_reconnect() {
                                self.handle_reconnect(&reconnect_fn).await;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn handle_reconnect<F, Fut>(&self, reconnect_fn: &F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        self.connection_manager
            .set_status(ConnectionStatus::Reconnecting)
            .await;

        let attempts = self.connection_manager.increment_reconnect_attempts();
        log::info!(
            "Attempting to reconnect (attempt {}/{})",
            attempts,
            self.connection_manager.get_config().max_reconnect_attempts
        );

        tokio::time::sleep(self.connection_manager.get_reconnect_interval()).await;

        match reconnect_fn().await {
            Ok(_) => {
                self.connection_manager.reset_reconnect_attempts();
                self.connection_manager.update_heartbeat();
                log::info!("Successfully reconnected");
            }
            Err(e) => {
                log::error!("Reconnection failed: {}", e);
                if !self.connection_manager.can_reconnect() {
                    self.connection_manager
                        .set_status(ConnectionStatus::Error)
                        .await;
                }
            }
        }
    }
}
