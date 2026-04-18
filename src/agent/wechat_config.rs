//! WeChat 处理器配置模块
//! 
//! 提供生产级配置支持，包括：
//! - 处理器配置
//! - 消息去重配置
//! - 功能开关配置

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// WeChat 处理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeChatHandlerConfig {
    /// 是否启用
    pub enabled: bool,
    
    /// 自动确认的置信度阈值
    pub auto_confirm_threshold: ConfidenceThreshold,
    
    /// 启用进度通知
    pub enable_progress_notification: bool,
    
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
    
    /// 消息去重配置
    pub deduplication: DeduplicationConfig,
    
    /// 优雅降级配置
    pub graceful_degradation: GracefulDegradationConfig,
}

impl Default for WeChatHandlerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_confirm_threshold: ConfidenceThreshold::High,
            enable_progress_notification: false,
            max_concurrent_tasks: 5,
            deduplication: DeduplicationConfig::default(),
            graceful_degradation: GracefulDegradationConfig::default(),
        }
    }
}

/// 置信度阈值
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceThreshold {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

/// 消息去重配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationConfig {
    /// 是否启用去重
    pub enabled: bool,
    
    /// 消息 TTL（秒）
    #[serde(with = "duration_seconds")]
    pub ttl: Duration,
    
    /// 最大缓存消息数
    pub max_cache_size: usize,
}

impl Default for DeduplicationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl: Duration::from_secs(3600), // 1小时
            max_cache_size: 10000,
        }
    }
}

/// 优雅降级配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GracefulDegradationConfig {
    /// 是否启用优雅降级
    pub enabled: bool,
    
    /// 连续失败次数阈值
    pub failure_threshold: u32,
    
    /// 恢复时间窗口（秒）
    #[serde(with = "duration_seconds")]
    pub recovery_window: Duration,
}

impl Default for GracefulDegradationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            failure_threshold: 5,
            recovery_window: Duration::from_secs(60), // 1分钟
        }
    }
}

/// 处理器模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerMode {
    /// 完整功能模式
    Full,
    /// 简单回复模式
    SimpleReply,
    /// 离线模式
    Offline,
}

impl Default for HandlerMode {
    fn default() -> Self {
        HandlerMode::Full
    }
}

// Duration 序列化辅助模块
mod duration_seconds {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = WeChatHandlerConfig::default();
        assert!(config.enabled);
        assert_eq!(config.auto_confirm_threshold, ConfidenceThreshold::High);
        assert!(!config.enable_progress_notification);
        assert_eq!(config.max_concurrent_tasks, 5);
    }

    #[test]
    fn test_deduplication_config_default() {
        let config = DeduplicationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.ttl, Duration::from_secs(3600));
        assert_eq!(config.max_cache_size, 10000);
    }

    #[test]
    fn test_graceful_degradation_config_default() {
        let config = GracefulDegradationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.recovery_window, Duration::from_secs(60));
    }

    #[test]
    fn test_handler_mode_default() {
        let mode = HandlerMode::default();
        assert_eq!(mode, HandlerMode::Full);
    }

    #[test]
    fn test_config_serialization() {
        let config = WeChatHandlerConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: WeChatHandlerConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config.enabled, deserialized.enabled);
    }
}
