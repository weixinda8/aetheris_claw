use crate::utils::{AetherisError, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SandboxSecurityLevel {
    Level0 = 0,
    Level1 = 1,
    Level2 = 2,
    Level3 = 3,
    Level4 = 4,
}

impl SandboxSecurityLevel {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(SandboxSecurityLevel::Level0),
            1 => Some(SandboxSecurityLevel::Level1),
            2 => Some(SandboxSecurityLevel::Level2),
            3 => Some(SandboxSecurityLevel::Level3),
            4 => Some(SandboxSecurityLevel::Level4),
            _ => None,
        }
    }

    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SandboxSecurityLevel::Level0 => "level_0_no_isolation",
            SandboxSecurityLevel::Level1 => "level_1_basic_isolation",
            SandboxSecurityLevel::Level2 => "level_2_enhanced_isolation",
            SandboxSecurityLevel::Level3 => "level_3_strict_isolation",
            SandboxSecurityLevel::Level4 => "level_4_maximum_isolation",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SandboxSecurityLevel::Level0 => "无隔离（开发模式）",
            SandboxSecurityLevel::Level1 => "基础隔离（进程级）",
            SandboxSecurityLevel::Level2 => "增强隔离（容器级）",
            SandboxSecurityLevel::Level3 => "严格隔离（网络+资源限制）",
            SandboxSecurityLevel::Level4 => "最大隔离（完全沙箱化）",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ResourceLimits {
    pub cpu_cores: Option<f64>,
    pub memory_bytes: Option<u64>,
    pub disk_io_read_bytes_per_second: Option<u64>,
    pub disk_io_write_bytes_per_second: Option<u64>,
    pub network_bandwidth_bytes_per_second: Option<u64>,
    pub max_processes: Option<u32>,
    pub max_open_files: Option<u32>,
}


impl ResourceLimits {
    pub fn level_0() -> Self {
        Self {
            cpu_cores: None,
            memory_bytes: None,
            disk_io_read_bytes_per_second: None,
            disk_io_write_bytes_per_second: None,
            network_bandwidth_bytes_per_second: None,
            max_processes: None,
            max_open_files: None,
        }
    }

    pub fn level_1() -> Self {
        Self {
            cpu_cores: Some(4.0),
            memory_bytes: Some(4 * 1024 * 1024 * 1024),
            disk_io_read_bytes_per_second: None,
            disk_io_write_bytes_per_second: None,
            network_bandwidth_bytes_per_second: None,
            max_processes: Some(100),
            max_open_files: Some(1024),
        }
    }

    pub fn level_2() -> Self {
        Self {
            cpu_cores: Some(2.0),
            memory_bytes: Some(2 * 1024 * 1024 * 1024),
            disk_io_read_bytes_per_second: Some(100 * 1024 * 1024),
            disk_io_write_bytes_per_second: Some(50 * 1024 * 1024),
            network_bandwidth_bytes_per_second: None,
            max_processes: Some(50),
            max_open_files: Some(512),
        }
    }

    pub fn level_3() -> Self {
        Self {
            cpu_cores: Some(1.0),
            memory_bytes: Some(1024 * 1024 * 1024),
            disk_io_read_bytes_per_second: Some(50 * 1024 * 1024),
            disk_io_write_bytes_per_second: Some(25 * 1024 * 1024),
            network_bandwidth_bytes_per_second: Some(10 * 1024 * 1024),
            max_processes: Some(25),
            max_open_files: Some(256),
        }
    }

    pub fn level_4() -> Self {
        Self {
            cpu_cores: Some(0.5),
            memory_bytes: Some(512 * 1024 * 1024),
            disk_io_read_bytes_per_second: Some(10 * 1024 * 1024),
            disk_io_write_bytes_per_second: Some(5 * 1024 * 1024),
            network_bandwidth_bytes_per_second: Some(1024 * 1024),
            max_processes: Some(10),
            max_open_files: Some(128),
        }
    }

    pub fn for_level(level: SandboxSecurityLevel) -> Self {
        match level {
            SandboxSecurityLevel::Level0 => Self::level_0(),
            SandboxSecurityLevel::Level1 => Self::level_1(),
            SandboxSecurityLevel::Level2 => Self::level_2(),
            SandboxSecurityLevel::Level3 => Self::level_3(),
            SandboxSecurityLevel::Level4 => Self::level_4(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub security_level: SandboxSecurityLevel,
    pub allow_network_access: bool,
    pub allow_file_system_access: bool,
    pub allowed_file_paths: Vec<String>,
    pub allowed_network_endpoints: Vec<String>,
    pub resource_limits: ResourceLimits,
    pub enable_audit_logging: bool,
    pub enable_anomaly_detection: bool,
    pub max_execution_time_seconds: u64,
    pub allowed_commands: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self::for_level(SandboxSecurityLevel::Level1)
    }
}

impl SandboxConfig {
    pub fn for_level(level: SandboxSecurityLevel) -> Self {
        let resource_limits = ResourceLimits::for_level(level);

        match level {
            SandboxSecurityLevel::Level0 => Self {
                security_level: level,
                allow_network_access: true,
                allow_file_system_access: true,
                allowed_file_paths: vec!["/".to_string()],
                allowed_network_endpoints: vec!["*".to_string()],
                resource_limits,
                enable_audit_logging: false,
                enable_anomaly_detection: false,
                max_execution_time_seconds: 3600,
                allowed_commands: vec!["*".to_string()],
            },
            SandboxSecurityLevel::Level1 => Self {
                security_level: level,
                allow_network_access: true,
                allow_file_system_access: true,
                allowed_file_paths: vec!["/tmp".to_string()],
                allowed_network_endpoints: vec![],
                resource_limits,
                enable_audit_logging: true,
                enable_anomaly_detection: false,
                max_execution_time_seconds: 300,
                allowed_commands: vec!["ls".to_string(), "echo".to_string(), "cat".to_string()],
            },
            SandboxSecurityLevel::Level2 => Self {
                security_level: level,
                allow_network_access: false,
                allow_file_system_access: true,
                allowed_file_paths: vec!["/tmp".to_string()],
                allowed_network_endpoints: vec![],
                resource_limits,
                enable_audit_logging: true,
                enable_anomaly_detection: true,
                max_execution_time_seconds: 120,
                allowed_commands: vec!["ls".to_string(), "echo".to_string()],
            },
            SandboxSecurityLevel::Level3 => Self {
                security_level: level,
                allow_network_access: false,
                allow_file_system_access: false,
                allowed_file_paths: vec![],
                allowed_network_endpoints: vec![],
                resource_limits,
                enable_audit_logging: true,
                enable_anomaly_detection: true,
                max_execution_time_seconds: 60,
                allowed_commands: vec!["echo".to_string()],
            },
            SandboxSecurityLevel::Level4 => Self {
                security_level: level,
                allow_network_access: false,
                allow_file_system_access: false,
                allowed_file_paths: vec![],
                allowed_network_endpoints: vec![],
                resource_limits,
                enable_audit_logging: true,
                enable_anomaly_detection: true,
                max_execution_time_seconds: 30,
                allowed_commands: vec![],
            },
        }
    }

    pub fn is_command_allowed(&self, command: &str) -> bool {
        if self.allowed_commands.contains(&"*".to_string()) {
            return true;
        }
        let command_name = command.split_whitespace().next().unwrap_or("");
        self.allowed_commands.contains(&command_name.to_string())
    }

    pub fn is_file_path_allowed(&self, path: &str) -> bool {
        if self.allowed_file_paths.contains(&"*".to_string()) {
            return true;
        }
        for allowed_path in &self.allowed_file_paths {
            if path.starts_with(allowed_path) {
                return true;
            }
        }
        false
    }

    pub fn is_network_endpoint_allowed(&self, endpoint: &str) -> bool {
        if self.allowed_network_endpoints.contains(&"*".to_string()) {
            return true;
        }
        self.allowed_network_endpoints
            .contains(&endpoint.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnomalyType {
    HighCpuUsage,
    HighMemoryUsage,
    UnusualNetworkActivity,
    UnusualFileSystemActivity,
    UnauthorizedCommandAttempt,
    UnauthorizedFileAccess,
    UnauthorizedNetworkAccess,
    SuspiciousProcessCreation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyAlert {
    pub alert_id: String,
    pub sandbox_id: String,
    pub anomaly_type: AnomalyType,
    pub severity: AlertSeverity,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxAuditLog {
    pub log_id: String,
    pub sandbox_id: String,
    pub event_type: AuditEventType,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditEventType {
    SandboxCreated,
    SandboxDestroyed,
    CommandExecuted,
    FileAccessed,
    NetworkAccessAttempted,
    ResourceLimitExceeded,
    AnomalyDetected,
    AlertTriggered,
    ConfigurationChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInstance {
    pub sandbox_id: String,
    pub config: SandboxConfig,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub is_active: bool,
    pub metrics: SandboxMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SandboxMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub disk_io_read_bytes: u64,
    pub disk_io_write_bytes: u64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
    pub commands_executed: u64,
    pub files_accessed: u64,
    pub network_accesses: u64,
    pub anomaly_count: u64,
}

pub struct SandboxManager {
    sandboxes: Arc<DashMap<String, SandboxInstance>>,
    audit_logs: Arc<RwLock<Vec<SandboxAuditLog>>>,
    alerts: Arc<RwLock<Vec<AnomalyAlert>>>,
    max_sandboxes: usize,
    default_config: SandboxConfig,
}

impl SandboxManager {
    pub fn new() -> Self {
        Self::with_default_config(SandboxConfig::default())
    }

    pub fn with_default_config(default_config: SandboxConfig) -> Self {
        Self {
            sandboxes: Arc::new(DashMap::new()),
            audit_logs: Arc::new(RwLock::new(Vec::new())),
            alerts: Arc::new(RwLock::new(Vec::new())),
            max_sandboxes: 100,
            default_config,
        }
    }

    pub fn with_max_sandboxes(mut self, max_sandboxes: usize) -> Self {
        self.max_sandboxes = max_sandboxes;
        self
    }

    pub async fn create_sandbox(&self, config: Option<SandboxConfig>) -> Result<String> {
        let config = config.unwrap_or_else(|| self.default_config.clone());

        if self.sandboxes.len() >= self.max_sandboxes {
            return Err(AetherisError::Security(format!(
                "Maximum number of sandboxes reached: {}",
                self.max_sandboxes
            )));
        }

        let sandbox_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let instance = SandboxInstance {
            sandbox_id: sandbox_id.clone(),
            config: config.clone(),
            created_at: now,
            last_activity: now,
            is_active: true,
            metrics: SandboxMetrics::default(),
        };

        self.sandboxes.insert(sandbox_id.clone(), instance);

        let audit_log = SandboxAuditLog {
            log_id: Uuid::new_v4().to_string(),
            sandbox_id: sandbox_id.clone(),
            event_type: AuditEventType::SandboxCreated,
            timestamp: now,
            success: true,
            details: serde_json::json!({
                "security_level": config.security_level.as_str(),
                "config": config
            }),
        };

        self.log_audit_event(audit_log).await?;

        info!(
            "Created sandbox: {} at level {}",
            sandbox_id,
            config.security_level.as_str()
        );

        Ok(sandbox_id)
    }

    pub async fn destroy_sandbox(&self, sandbox_id: &str) -> Result<()> {
        let sandbox = self.sandboxes.remove(sandbox_id);

        if let Some((_, mut instance)) = sandbox {
            instance.is_active = false;

            let audit_log = SandboxAuditLog {
                log_id: Uuid::new_v4().to_string(),
                sandbox_id: sandbox_id.to_string(),
                event_type: AuditEventType::SandboxDestroyed,
                timestamp: Utc::now(),
                success: true,
                details: serde_json::json!({}),
            };

            self.log_audit_event(audit_log).await?;

            info!("Destroyed sandbox: {}", sandbox_id);
            Ok(())
        } else {
            Err(AetherisError::NotFound(format!(
                "Sandbox not found: {}",
                sandbox_id
            )))
        }
    }

    pub fn get_sandbox(&self, sandbox_id: &str) -> Option<SandboxInstance> {
        self.sandboxes.get(sandbox_id).map(|s| s.clone())
    }

    pub fn list_sandboxes(&self) -> Vec<SandboxInstance> {
        self.sandboxes.iter().map(|s| s.clone()).collect()
    }

    pub async fn execute_command(
        &self,
        sandbox_id: &str,
        command: &str,
    ) -> Result<SandboxExecutionResult> {
        let mut sandbox = self
            .sandboxes
            .get_mut(sandbox_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Sandbox not found: {}", sandbox_id)))?;

        if !sandbox.is_active {
            return Err(AetherisError::Security(format!(
                "Sandbox is not active: {}",
                sandbox_id
            )));
        }

        let config = sandbox.config.clone();

        if !config.is_command_allowed(command) {
            self.handle_unauthorized_attempt(
                sandbox_id,
                AnomalyType::UnauthorizedCommandAttempt,
                format!("Unauthorized command attempt: {}", command),
            )
            .await?;

            return Err(AetherisError::Security(format!(
                "Command not allowed: {}",
                command
            )));
        }

        sandbox.last_activity = Utc::now();
        sandbox.metrics.commands_executed += 1;

        let start = std::time::Instant::now();

        let result = self
            .execute_command_internal(sandbox_id, command, &config)
            .await;

        let duration = start.elapsed();

        let execution_result = match result {
            Ok(output) => SandboxExecutionResult {
                success: true,
                output: Some(output),
                error: None,
                execution_time_ms: duration.as_millis() as u64,
            },
            Err(e) => SandboxExecutionResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
                execution_time_ms: duration.as_millis() as u64,
            },
        };

        let audit_log = SandboxAuditLog {
            log_id: Uuid::new_v4().to_string(),
            sandbox_id: sandbox_id.to_string(),
            event_type: AuditEventType::CommandExecuted,
            timestamp: Utc::now(),
            success: execution_result.success,
            details: serde_json::json!({
                "command": command,
                "execution_time_ms": execution_result.execution_time_ms
            }),
        };

        if config.enable_audit_logging {
            self.log_audit_event(audit_log).await?;
        }

        Ok(execution_result)
    }

    async fn execute_command_internal(
        &self,
        _sandbox_id: &str,
        command: &str,
        config: &SandboxConfig,
    ) -> Result<String> {
        let timeout = std::time::Duration::from_secs(config.max_execution_time_seconds);

        tokio::time::timeout(timeout, async {
            tokio::task::yield_now().await;
            Ok(format!("Executed command: {}", command))
        })
        .await
        .map_err(|_| AetherisError::Timeout("Command execution timed out".to_string()))?
    }

    pub async fn check_file_access(&self, sandbox_id: &str, path: &str) -> Result<bool> {
        let mut sandbox = self
            .sandboxes
            .get_mut(sandbox_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Sandbox not found: {}", sandbox_id)))?;

        let config = sandbox.config.clone();
        sandbox.last_activity = Utc::now();
        sandbox.metrics.files_accessed += 1;

        let allowed = config.is_file_path_allowed(path);

        if !allowed {
            self.handle_unauthorized_attempt(
                sandbox_id,
                AnomalyType::UnauthorizedFileAccess,
                format!("Unauthorized file access attempt: {}", path),
            )
            .await?;
        }

        if config.enable_audit_logging {
            let audit_log = SandboxAuditLog {
                log_id: Uuid::new_v4().to_string(),
                sandbox_id: sandbox_id.to_string(),
                event_type: AuditEventType::FileAccessed,
                timestamp: Utc::now(),
                success: allowed,
                details: serde_json::json!({
                    "path": path
                }),
            };
            self.log_audit_event(audit_log).await?;
        }

        Ok(allowed)
    }

    pub async fn check_network_access(&self, sandbox_id: &str, endpoint: &str) -> Result<bool> {
        let mut sandbox = self
            .sandboxes
            .get_mut(sandbox_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Sandbox not found: {}", sandbox_id)))?;

        let config = sandbox.config.clone();
        sandbox.last_activity = Utc::now();
        sandbox.metrics.network_accesses += 1;

        let allowed = config.allow_network_access && config.is_network_endpoint_allowed(endpoint);

        if !allowed {
            self.handle_unauthorized_attempt(
                sandbox_id,
                AnomalyType::UnauthorizedNetworkAccess,
                format!("Unauthorized network access attempt: {}", endpoint),
            )
            .await?;
        }

        if config.enable_audit_logging {
            let audit_log = SandboxAuditLog {
                log_id: Uuid::new_v4().to_string(),
                sandbox_id: sandbox_id.to_string(),
                event_type: AuditEventType::NetworkAccessAttempted,
                timestamp: Utc::now(),
                success: allowed,
                details: serde_json::json!({
                    "endpoint": endpoint
                }),
            };
            self.log_audit_event(audit_log).await?;
        }

        Ok(allowed)
    }

    async fn handle_unauthorized_attempt(
        &self,
        sandbox_id: &str,
        anomaly_type: AnomalyType,
        description: String,
    ) -> Result<()> {
        warn!("{}", description);

        let enable_anomaly_detection = self
            .sandboxes
            .get(sandbox_id)
            .map(|s| s.config.enable_anomaly_detection)
            .unwrap_or(false);

        if enable_anomaly_detection {
            let alert = AnomalyAlert {
                alert_id: Uuid::new_v4().to_string(),
                sandbox_id: sandbox_id.to_string(),
                anomaly_type: anomaly_type.clone(),
                severity: AlertSeverity::High,
                description: description.clone(),
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            };

            self.log_alert(alert).await?;

            if let Some(mut sandbox) = self.sandboxes.get_mut(sandbox_id) {
                sandbox.metrics.anomaly_count += 1;
            }
        }

        Ok(())
    }

    pub async fn update_metrics(&self, sandbox_id: &str, metrics: SandboxMetrics) -> Result<()> {
        let mut sandbox = self
            .sandboxes
            .get_mut(sandbox_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Sandbox not found: {}", sandbox_id)))?;

        let config = sandbox.config.clone();
        sandbox.metrics = metrics.clone();
        sandbox.last_activity = Utc::now();

        if config.enable_anomaly_detection {
            self.check_for_anomalies(sandbox_id, &metrics, &config)
                .await?;
        }

        Ok(())
    }

    async fn check_for_anomalies(
        &self,
        sandbox_id: &str,
        metrics: &SandboxMetrics,
        config: &SandboxConfig,
    ) -> Result<()> {
        let limits = &config.resource_limits;

        if let Some(mem_limit) = limits.memory_bytes {
            if metrics.memory_usage_bytes > mem_limit * 90 / 100 {
                self.create_anomaly_alert(
                    sandbox_id,
                    AnomalyType::HighMemoryUsage,
                    AlertSeverity::Medium,
                    format!(
                        "Memory usage approaching limit: {} bytes",
                        metrics.memory_usage_bytes
                    ),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn create_anomaly_alert(
        &self,
        sandbox_id: &str,
        anomaly_type: AnomalyType,
        severity: AlertSeverity,
        description: String,
    ) -> Result<()> {
        let alert = AnomalyAlert {
            alert_id: Uuid::new_v4().to_string(),
            sandbox_id: sandbox_id.to_string(),
            anomaly_type,
            severity,
            description,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        self.log_alert(alert.clone()).await?;

        if let Some(mut sandbox) = self.sandboxes.get_mut(sandbox_id) {
            sandbox.metrics.anomaly_count += 1;
        }

        let audit_log = SandboxAuditLog {
            log_id: Uuid::new_v4().to_string(),
            sandbox_id: sandbox_id.to_string(),
            event_type: AuditEventType::AnomalyDetected,
            timestamp: Utc::now(),
            success: true,
            details: serde_json::json!({
                "alert_id": alert.alert_id,
                "anomaly_type": format!("{:?}", alert.anomaly_type),
                "severity": format!("{:?}", alert.severity)
            }),
        };

        self.log_audit_event(audit_log).await?;

        Ok(())
    }

    async fn log_audit_event(&self, event: SandboxAuditLog) -> Result<()> {
        let mut logs = self.audit_logs.write().await;
        logs.push(event);
        if logs.len() > 10000 {
            logs.drain(0..1000);
        }
        Ok(())
    }

    async fn log_alert(&self, alert: AnomalyAlert) -> Result<()> {
        let mut alerts = self.alerts.write().await;
        alerts.push(alert);
        if alerts.len() > 1000 {
            alerts.drain(0..100);
        }
        Ok(())
    }

    pub async fn get_audit_logs(&self, sandbox_id: Option<&str>) -> Vec<SandboxAuditLog> {
        let logs = self.audit_logs.read().await;
        match sandbox_id {
            Some(id) => logs
                .iter()
                .filter(|log| log.sandbox_id == id)
                .cloned()
                .collect(),
            None => logs.clone(),
        }
    }

    pub async fn get_alerts(&self, sandbox_id: Option<&str>) -> Vec<AnomalyAlert> {
        let alerts = self.alerts.read().await;
        match sandbox_id {
            Some(id) => alerts
                .iter()
                .filter(|alert| alert.sandbox_id == id)
                .cloned()
                .collect(),
            None => alerts.clone(),
        }
    }

    pub async fn update_config(&self, sandbox_id: &str, config: SandboxConfig) -> Result<()> {
        let mut sandbox = self
            .sandboxes
            .get_mut(sandbox_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Sandbox not found: {}", sandbox_id)))?;

        let old_config = sandbox.config.clone();
        sandbox.config = config.clone();
        sandbox.last_activity = Utc::now();

        let audit_log = SandboxAuditLog {
            log_id: Uuid::new_v4().to_string(),
            sandbox_id: sandbox_id.to_string(),
            event_type: AuditEventType::ConfigurationChanged,
            timestamp: Utc::now(),
            success: true,
            details: serde_json::json!({
                "old_security_level": old_config.security_level.as_str(),
                "new_security_level": config.security_level.as_str()
            }),
        };

        if config.enable_audit_logging {
            self.log_audit_event(audit_log).await?;
        }

        info!(
            "Updated sandbox config: {} from {} to {}",
            sandbox_id,
            old_config.security_level.as_str(),
            config.security_level.as_str()
        );

        Ok(())
    }
}

impl Default for SandboxManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxExecutionResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sandbox_security_level() {
        assert_eq!(SandboxSecurityLevel::Level0.as_u8(), 0);
        assert_eq!(
            SandboxSecurityLevel::from_u8(0),
            Some(SandboxSecurityLevel::Level0)
        );
        assert_eq!(SandboxSecurityLevel::from_u8(5), None);
    }

    #[tokio::test]
    async fn test_resource_limits() {
        let limits = ResourceLimits::for_level(SandboxSecurityLevel::Level1);
        assert_eq!(limits.cpu_cores, Some(4.0));
        assert_eq!(limits.memory_bytes, Some(4 * 1024 * 1024 * 1024));
    }

    #[tokio::test]
    async fn test_sandbox_config() {
        let config = SandboxConfig::for_level(SandboxSecurityLevel::Level1);
        assert_eq!(config.security_level, SandboxSecurityLevel::Level1);
        assert!(config.allow_network_access);
        assert!(config.is_command_allowed("ls"));
        assert!(!config.is_command_allowed("rm"));
    }

    #[tokio::test]
    async fn test_sandbox_manager() {
        let manager = SandboxManager::new();

        let sandbox_id = manager.create_sandbox(None).await.unwrap();
        assert!(!sandbox_id.is_empty());

        let sandbox = manager.get_sandbox(&sandbox_id).unwrap();
        assert_eq!(sandbox.sandbox_id, sandbox_id);
        assert!(sandbox.is_active);

        let result = manager.execute_command(&sandbox_id, "echo test").await;
        assert!(result.is_ok());

        let sandboxes = manager.list_sandboxes();
        assert_eq!(sandboxes.len(), 1);

        manager.destroy_sandbox(&sandbox_id).await.unwrap();
        assert!(manager.get_sandbox(&sandbox_id).is_none());
    }

    #[tokio::test]
    async fn test_unauthorized_command() {
        let manager = SandboxManager::new();
        let sandbox_id = manager.create_sandbox(None).await.unwrap();

        let result = manager.execute_command(&sandbox_id, "rm -rf /").await;
        assert!(result.is_err());

        manager.destroy_sandbox(&sandbox_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_file_access_control() {
        let manager = SandboxManager::new();
        let sandbox_id = manager.create_sandbox(None).await.unwrap();

        let allowed = manager
            .check_file_access(&sandbox_id, "/tmp/test.txt")
            .await
            .unwrap();
        assert!(allowed);

        let allowed = manager
            .check_file_access(&sandbox_id, "/etc/passwd")
            .await
            .unwrap();
        assert!(!allowed);

        manager.destroy_sandbox(&sandbox_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_network_access_control() {
        let manager = SandboxManager::new();
        let sandbox_id = manager.create_sandbox(None).await.unwrap();

        let allowed = manager
            .check_network_access(&sandbox_id, "https://example.com")
            .await
            .unwrap();
        assert!(!allowed);

        manager.destroy_sandbox(&sandbox_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_multiple_sandboxes() {
        let manager = SandboxManager::new();

        let id1 = manager
            .create_sandbox(Some(SandboxConfig::for_level(SandboxSecurityLevel::Level0)))
            .await
            .unwrap();
        let id2 = manager
            .create_sandbox(Some(SandboxConfig::for_level(SandboxSecurityLevel::Level4)))
            .await
            .unwrap();

        let sandboxes = manager.list_sandboxes();
        assert_eq!(sandboxes.len(), 2);

        let sb1 = manager.get_sandbox(&id1).unwrap();
        let sb2 = manager.get_sandbox(&id2).unwrap();

        assert_eq!(sb1.config.security_level, SandboxSecurityLevel::Level0);
        assert_eq!(sb2.config.security_level, SandboxSecurityLevel::Level4);

        manager.destroy_sandbox(&id1).await.unwrap();
        manager.destroy_sandbox(&id2).await.unwrap();

        assert_eq!(manager.list_sandboxes().len(), 0);
    }

    #[tokio::test]
    async fn test_config_update() {
        let manager = SandboxManager::new();
        let sandbox_id = manager.create_sandbox(None).await.unwrap();

        let new_config = SandboxConfig::for_level(SandboxSecurityLevel::Level4);
        manager
            .update_config(&sandbox_id, new_config.clone())
            .await
            .unwrap();

        let sandbox = manager.get_sandbox(&sandbox_id).unwrap();
        assert_eq!(sandbox.config.security_level, SandboxSecurityLevel::Level4);

        manager.destroy_sandbox(&sandbox_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_audit_logging() {
        let manager = SandboxManager::new();
        let sandbox_id = manager.create_sandbox(None).await.unwrap();

        let logs = manager.get_audit_logs(Some(&sandbox_id)).await;
        assert!(!logs.is_empty());
        assert_eq!(logs[0].event_type, AuditEventType::SandboxCreated);

        manager.destroy_sandbox(&sandbox_id).await.unwrap();
    }
}
