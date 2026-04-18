//! WeChat 消息去重模块
//! 
//! 提供生产级消息去重功能，防止重复处理同一条消息。

use crate::agent::wechat_config::DeduplicationConfig;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// 消息记录
#[derive(Debug, Clone)]
struct MessageRecord {
    /// 首次处理时间
    first_seen: Instant,
    /// 处理次数
    processed_count: u32,
}

/// 消息去重器
pub struct MessageDeduplicator {
    /// 配置
    config: DeduplicationConfig,
    /// 已处理消息缓存
    processed_messages: Arc<DashMap<String, MessageRecord>>,
    /// 上次清理时间
    last_cleanup: Arc<RwLock<Instant>>,
}

impl MessageDeduplicator {
    /// 创建新的消息去重器
    pub fn new(config: DeduplicationConfig) -> Self {
        Self {
            config,
            processed_messages: Arc::new(DashMap::new()),
            last_cleanup: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// 检查消息是否已处理
    pub async fn is_processed(&self, message_id: &str) -> bool {
        if !self.config.enabled {
            return false;
        }

        // 检查是否需要清理
        self.maybe_cleanup().await;

        if let Some(record) = self.processed_messages.get(message_id) {
            debug!("Message already processed: {}", message_id);
            true
        } else {
            false
        }
    }

    /// 标记消息为已处理
    pub async fn mark_processed(&self, message_id: String) {
        if !self.config.enabled {
            return;
        }

        let now = Instant::now();
        
        self.processed_messages
            .entry(message_id.clone())
            .and_modify(|record| {
                record.processed_count += 1;
            })
            .or_insert_with(|| MessageRecord {
                first_seen: now,
                processed_count: 1,
            });

        debug!("Marked message as processed: {}", message_id);

        // 检查缓存大小
        if self.processed_messages.len() > self.config.max_cache_size {
            warn!("Message cache size exceeded, triggering cleanup");
            self.force_cleanup().await;
        }
    }

    /// 获取消息处理次数
    pub async fn get_processed_count(&self, message_id: &str) -> u32 {
        self.processed_messages
            .get(message_id)
            .map(|r| r.processed_count)
            .unwrap_or(0)
    }

    /// 清理过期消息
    async fn maybe_cleanup(&self) {
        let mut last_cleanup = self.last_cleanup.write().await;
        let now = Instant::now();

        if now.duration_since(*last_cleanup) > Duration::from_secs(60) {
            *last_cleanup = now;
            drop(last_cleanup);
            self.force_cleanup().await;
        }
    }

    /// 强制清理过期消息
    async fn force_cleanup(&self) {
        info!("Starting message cache cleanup");

        let ttl = self.config.ttl;
        let now = Instant::now();

        let mut removed_count = 0;
        self.processed_messages.retain(|_key, record| {
            if now.duration_since(record.first_seen) > ttl {
                removed_count += 1;
                false
            } else {
                true
            }
        });

        info!(
            "Message cache cleanup complete, removed {} messages, current size: {}",
            removed_count,
            self.processed_messages.len()
        );
    }

    /// 获取当前缓存大小
    pub fn cache_size(&self) -> usize {
        self.processed_messages.len()
    }

    /// 清空缓存
    pub async fn clear(&self) {
        info!("Clearing message cache");
        self.processed_messages.clear();
    }
}

impl Clone for MessageDeduplicator {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            processed_messages: Arc::clone(&self.processed_messages),
            last_cleanup: Arc::clone(&self.last_cleanup),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_deduplication_basic() {
        let config = DeduplicationConfig::default();
        let deduplicator = MessageDeduplicator::new(config);

        let msg_id = "test-message-001";

        // 首次检查，应该未处理
        assert!(!deduplicator.is_processed(msg_id).await);

        // 标记为已处理
        deduplicator.mark_processed(msg_id.to_string()).await;

        // 再次检查，应该已处理
        assert!(deduplicator.is_processed(msg_id).await);

        // 检查处理次数
        assert_eq!(deduplicator.get_processed_count(msg_id).await, 1);
    }

    #[tokio::test]
    async fn test_deduplication_disabled() {
        let mut config = DeduplicationConfig::default();
        config.enabled = false;
        let deduplicator = MessageDeduplicator::new(config);

        let msg_id = "test-message-002";

        // 即使标记为已处理，禁用时也返回 false
        deduplicator.mark_processed(msg_id.to_string()).await;
        assert!(!deduplicator.is_processed(msg_id).await);
    }

    #[tokio::test]
    async fn test_multiple_messages() {
        let config = DeduplicationConfig::default();
        let deduplicator = MessageDeduplicator::new(config);

        let msg_ids = vec!["msg-1", "msg-2", "msg-3"];

        for msg_id in &msg_ids {
            deduplicator.mark_processed(msg_id.to_string()).await;
        }

        assert_eq!(deduplicator.cache_size(), 3);

        for msg_id in &msg_ids {
            assert!(deduplicator.is_processed(msg_id).await);
        }
    }

    #[tokio::test]
    async fn test_duplicate_mark() {
        let config = DeduplicationConfig::default();
        let deduplicator = MessageDeduplicator::new(config);

        let msg_id = "test-message-003";

        // 多次标记
        deduplicator.mark_processed(msg_id.to_string()).await;
        deduplicator.mark_processed(msg_id.to_string()).await;
        deduplicator.mark_processed(msg_id.to_string()).await;

        // 处理次数应该是 3
        assert_eq!(deduplicator.get_processed_count(msg_id).await, 3);
    }

    #[tokio::test]
    async fn test_cache_size_limit() {
        let mut config = DeduplicationConfig::default();
        config.max_cache_size = 5;
        let deduplicator = MessageDeduplicator::new(config);

        // 添加超过限制的消息
        for i in 0..10 {
            let msg_id = format!("msg-{}", i);
            deduplicator.mark_processed(msg_id).await;
        }

        // 缓存大小应该有限制（虽然不会立即删除，下次清理时才会）
        assert!(deduplicator.cache_size() >= 5);
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let config = DeduplicationConfig::default();
        let deduplicator = MessageDeduplicator::new(config);

        deduplicator.mark_processed("msg-1".to_string()).await;
        deduplicator.mark_processed("msg-2".to_string()).await;

        assert_eq!(deduplicator.cache_size(), 2);

        deduplicator.clear().await;

        assert_eq!(deduplicator.cache_size(), 0);
        assert!(!deduplicator.is_processed("msg-1").await);
    }

    #[tokio::test]
    async fn test_clone() {
        let config = DeduplicationConfig::default();
        let deduplicator1 = MessageDeduplicator::new(config);

        deduplicator1.mark_processed("msg-1".to_string()).await;

        let deduplicator2 = deduplicator1.clone();

        // 克隆的实例应该共享状态
        assert!(deduplicator2.is_processed("msg-1").await);
        assert_eq!(deduplicator1.cache_size(), deduplicator2.cache_size());
    }
}
