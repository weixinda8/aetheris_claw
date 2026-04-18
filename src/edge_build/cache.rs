use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use hashlink::LinkedHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CacheEvictionPolicy {
    LRU,
    FIFO,
    LFU,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub hit_count: u64,
    pub last_accessed: DateTime<Utc>,
}

pub struct EdgeCacheManager {
    cache: Arc<DashMap<String, CacheEntry>>,
    lru_order: Arc<RwLock<LinkedHashMap<String, ()>>>,
    policy: CacheEvictionPolicy,
    max_entries: usize,
    default_ttl: Duration,
    hit_count: Arc<std::sync::atomic::AtomicU64>,
    miss_count: Arc<std::sync::atomic::AtomicU64>,
}

impl EdgeCacheManager {
    pub fn new(policy: CacheEvictionPolicy, max_entries: usize, default_ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            lru_order: Arc::new(RwLock::new(LinkedHashMap::new())),
            policy,
            max_entries,
            default_ttl: Duration::seconds(default_ttl_seconds as i64),
            hit_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            miss_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        if let Some(mut entry) = self.cache.get_mut(key) {
            if let Some(expires_at) = entry.expires_at {
                if Utc::now() > expires_at {
                    self.cache.remove(key);
                    self.miss_count
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return None;
                }
            }

            entry.hit_count += 1;
            entry.last_accessed = Utc::now();
            self.hit_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            self.update_access_order(key).await;

            return Some(entry.value.clone());
        }

        self.miss_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        None
    }

    pub async fn set(&self, key: String, value: serde_json::Value, ttl_seconds: Option<u64>) {
        self.evict_if_needed().await;

        let expires_at = ttl_seconds
            .map(|ttl| Utc::now() + Duration::seconds(ttl as i64))
            .or_else(|| Some(Utc::now() + self.default_ttl));

        let entry = CacheEntry {
            key: key.clone(),
            value,
            created_at: Utc::now(),
            expires_at,
            hit_count: 0,
            last_accessed: Utc::now(),
        };

        self.cache.insert(key.clone(), entry);
        self.update_access_order(&key).await;
    }

    pub fn delete(&self, key: &str) {
        self.cache.remove(key);
    }

    pub fn clear(&self) {
        self.cache.clear();
    }

    pub async fn cleanup_expired(&self) -> usize {
        let now = Utc::now();
        let mut removed = 0;

        self.cache.retain(|_, entry| {
            if let Some(expires_at) = entry.expires_at {
                if now > expires_at {
                    removed += 1;
                    return false;
                }
            }
            true
        });

        removed
    }

    pub fn get_stats(&self) -> CacheStats {
        let hits = self.hit_count.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.miss_count.load(std::sync::atomic::Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };

        CacheStats {
            entries: self.cache.len(),
            max_entries: self.max_entries,
            hits,
            misses,
            hit_rate,
            policy: self.policy,
        }
    }

    async fn update_access_order(&self, key: &str) {
        if self.policy == CacheEvictionPolicy::LRU {
            let mut order = self.lru_order.write().await;
            order.remove(key);
            order.insert(key.to_string(), ());
        }
    }

    async fn evict_if_needed(&self) {
        if self.cache.len() >= self.max_entries {
            match self.policy {
                CacheEvictionPolicy::LRU => {
                    let mut order = self.lru_order.write().await;
                    if let Some((key, _)) = order.pop_front() {
                        self.cache.remove(&key);
                    }
                }
                CacheEvictionPolicy::FIFO => {
                    if let Some(entry) = self.cache.iter().next() {
                        let key = entry.key().clone();
                        self.cache.remove(&key);
                    }
                }
                CacheEvictionPolicy::LFU => {
                    let mut min_hits = u64::MAX;
                    let mut evict_key = None;

                    for entry in self.cache.iter() {
                        if entry.hit_count < min_hits {
                            min_hits = entry.hit_count;
                            evict_key = Some(entry.key().clone());
                        }
                    }

                    if let Some(key) = evict_key {
                        self.cache.remove(&key);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub entries: usize,
    pub max_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub policy: CacheEvictionPolicy,
}

impl Default for EdgeCacheManager {
    fn default() -> Self {
        Self::new(CacheEvictionPolicy::LRU, 10000, 3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_edge_cache_manager_new() {
        let cache = EdgeCacheManager::new(CacheEvictionPolicy::LRU, 100, 3600);
        let stats = cache.get_stats();
        assert_eq!(stats.entries, 0);
        assert_eq!(stats.max_entries, 100);
        assert_eq!(stats.policy, CacheEvictionPolicy::LRU);
    }

    #[test]
    fn test_edge_cache_manager_default() {
        let cache = EdgeCacheManager::default();
        let stats = cache.get_stats();
        assert_eq!(stats.entries, 0);
    }

    #[tokio::test]
    async fn test_set_and_get() {
        let cache = EdgeCacheManager::new(CacheEvictionPolicy::LRU, 100, 3600);

        cache.set("key1".to_string(), json!("value1"), None).await;

        let value = cache.get("key1").await;
        assert!(value.is_some());
        assert_eq!(value.unwrap(), json!("value1"));
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let cache = EdgeCacheManager::new(CacheEvictionPolicy::LRU, 100, 3600);

        let value = cache.get("nonexistent").await;
        assert!(value.is_none());

        let stats = cache.get_stats();
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_delete() {
        let cache = EdgeCacheManager::new(CacheEvictionPolicy::LRU, 100, 3600);

        cache.set("key1".to_string(), json!("value1"), None).await;
        assert!(cache.get("key1").await.is_some());

        cache.delete("key1");
        assert!(cache.get("key1").await.is_none());
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = EdgeCacheManager::new(CacheEvictionPolicy::LRU, 100, 3600);

        cache.set("key1".to_string(), json!("value1"), None).await;
        cache.set("key2".to_string(), json!("value2"), None).await;
        cache.set("key3".to_string(), json!("value3"), None).await;

        let stats = cache.get_stats();
        assert_eq!(stats.entries, 3);

        cache.clear();

        let stats = cache.get_stats();
        assert_eq!(stats.entries, 0);
    }

    #[tokio::test]
    async fn test_hit_stats() {
        let cache = EdgeCacheManager::new(CacheEvictionPolicy::LRU, 100, 3600);

        cache.set("key1".to_string(), json!("value1"), None).await;

        cache.get("key1").await;
        cache.get("key1").await;
        cache.get("nonexistent").await;

        let stats = cache.get_stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 0.666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let cache = EdgeCacheManager::new(CacheEvictionPolicy::LRU, 3, 3600);

        cache.set("key1".to_string(), json!("value1"), None).await;
        cache.set("key2".to_string(), json!("value2"), None).await;
        cache.set("key3".to_string(), json!("value3"), None).await;

        cache.get("key1").await;
        cache.get("key2").await;

        cache.set("key4".to_string(), json!("value4"), None).await;

        let stats = cache.get_stats();
        assert_eq!(stats.entries, 3);

        assert!(cache.get("key3").await.is_none());
        assert!(cache.get("key1").await.is_some());
        assert!(cache.get("key2").await.is_some());
        assert!(cache.get("key4").await.is_some());
    }

    #[tokio::test]
    async fn test_fifo_eviction() {
        let cache = EdgeCacheManager::new(CacheEvictionPolicy::FIFO, 3, 3600);

        cache.set("key1".to_string(), json!("value1"), None).await;
        cache.set("key2".to_string(), json!("value2"), None).await;
        cache.set("key3".to_string(), json!("value3"), None).await;

        cache.get("key1").await;
        cache.get("key2").await;

        cache.set("key4".to_string(), json!("value4"), None).await;

        let stats = cache.get_stats();
        assert_eq!(stats.entries, 3);

        assert!(cache.get("key1").await.is_none());
        assert!(cache.get("key2").await.is_some());
        assert!(cache.get("key3").await.is_some());
        assert!(cache.get("key4").await.is_some());
    }

    #[tokio::test]
    async fn test_lfu_eviction() {
        let cache = EdgeCacheManager::new(CacheEvictionPolicy::LFU, 3, 3600);

        cache.set("key1".to_string(), json!("value1"), None).await;
        cache.set("key2".to_string(), json!("value2"), None).await;
        cache.set("key3".to_string(), json!("value3"), None).await;

        cache.get("key1").await;
        cache.get("key1").await;
        cache.get("key2").await;

        cache.set("key4".to_string(), json!("value4"), None).await;

        let stats = cache.get_stats();
        assert_eq!(stats.entries, 3);

        assert!(cache.get("key3").await.is_none());
        assert!(cache.get("key1").await.is_some());
        assert!(cache.get("key2").await.is_some());
        assert!(cache.get("key4").await.is_some());
    }

    #[tokio::test]
    async fn test_custom_ttl() {
        let cache = EdgeCacheManager::new(CacheEvictionPolicy::LRU, 100, 3600);

        cache
            .set("key1".to_string(), json!("value1"), Some(10))
            .await;

        let value = cache.get("key1").await;
        assert!(value.is_some());
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let cache = EdgeCacheManager::new(CacheEvictionPolicy::LRU, 100, 1);

        cache
            .set("key1".to_string(), json!("value1"), Some(1))
            .await;
        cache.set("key2".to_string(), json!("value2"), None).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

        let removed = cache.cleanup_expired().await;
        assert!(removed >= 1);

        let stats = cache.get_stats();
        assert_eq!(stats.entries, 0);
    }

    #[tokio::test]
    async fn test_entry_hit_count() {
        let cache = EdgeCacheManager::new(CacheEvictionPolicy::LRU, 100, 3600);

        cache.set("key1".to_string(), json!("value1"), None).await;

        cache.get("key1").await;
        cache.get("key1").await;
        cache.get("key1").await;

        let stats = cache.get_stats();
        assert_eq!(stats.hits, 3);
    }

    #[test]
    fn test_cache_eviction_policy_equality() {
        assert_eq!(CacheEvictionPolicy::LRU, CacheEvictionPolicy::LRU);
        assert_eq!(CacheEvictionPolicy::FIFO, CacheEvictionPolicy::FIFO);
        assert_eq!(CacheEvictionPolicy::LFU, CacheEvictionPolicy::LFU);
    }

    #[tokio::test]
    async fn test_hit_rate_zero() {
        let cache = EdgeCacheManager::new(CacheEvictionPolicy::LRU, 100, 3600);
        let stats = cache.get_stats();
        assert_eq!(stats.hit_rate, 0.0);
    }

    #[tokio::test]
    async fn test_set_overwrite() {
        let cache = EdgeCacheManager::new(CacheEvictionPolicy::LRU, 100, 3600);

        cache.set("key1".to_string(), json!("value1"), None).await;
        cache.set("key1".to_string(), json!("value2"), None).await;

        let value = cache.get("key1").await;
        assert!(value.is_some());
        assert_eq!(value.unwrap(), json!("value2"));
    }
}
