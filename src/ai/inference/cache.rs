use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::info;

use super::InferenceOutput;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    output: InferenceOutput,
    created_at: chrono::DateTime<chrono::Utc>,
    ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_hits: u64,
    pub total_misses: u64,
    pub current_entries: usize,
    pub hit_rate: f64,
}

pub struct InferenceCache {
    cache: Arc<DashMap<String, CacheEntry>>,
    total_hits: AtomicU64,
    total_misses: AtomicU64,
}

impl InferenceCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            total_hits: AtomicU64::new(0),
            total_misses: AtomicU64::new(0),
        }
    }

    pub fn cache_lookup(&self, key: &str) -> Option<InferenceOutput> {
        if let Some(entry) = self.cache.get(key) {
            let now = chrono::Utc::now();
            let elapsed = (now - entry.created_at).num_seconds() as u64;

            if elapsed < entry.ttl_seconds {
                self.total_hits.fetch_add(1, Ordering::Relaxed);
                info!("Cache hit for key: {}", key);
                return Some(entry.output.clone());
            } else {
                info!("Cache entry expired for key: {}", key);
            }
        }

        self.total_misses.fetch_add(1, Ordering::Relaxed);
        info!("Cache miss for key: {}", key);
        None
    }

    pub fn cache_store(&self, key: String, output: InferenceOutput, ttl_seconds: u64) {
        let entry = CacheEntry {
            output,
            created_at: chrono::Utc::now(),
            ttl_seconds,
        };

        info!("Caching result for key: {} with TTL: {}s", key, ttl_seconds);
        self.cache.insert(key, entry);
    }

    pub fn get_cache_stats(&self) -> CacheStats {
        let total_hits = self.total_hits.load(Ordering::Relaxed);
        let total_misses = self.total_misses.load(Ordering::Relaxed);
        let current_entries = self.cache.len();
        let total = total_hits + total_misses;
        let hit_rate = if total > 0 {
            total_hits as f64 / total as f64
        } else {
            0.0
        };

        CacheStats {
            total_hits,
            total_misses,
            current_entries,
            hit_rate,
        }
    }
}

impl Default for InferenceCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::inference::InferenceOutput;
    use serde_json::json;

    #[test]
    fn test_inference_cache_new() {
        let cache = InferenceCache::new();
        let stats = cache.get_cache_stats();
        assert_eq!(stats.total_hits, 0);
        assert_eq!(stats.total_misses, 0);
        assert_eq!(stats.current_entries, 0);
        assert_eq!(stats.hit_rate, 0.0);
    }

    #[test]
    fn test_inference_cache_default() {
        let cache = InferenceCache::default();
        let stats = cache.get_cache_stats();
        assert_eq!(stats.total_hits, 0);
    }

    #[test]
    fn test_cache_store_and_lookup() {
        let cache = InferenceCache::new();

        let output = InferenceOutput {
            model_id: "model-1".to_string(),
            data: json!({"result": "test"}),
            latency_ms: 100,
            success: true,
            error_message: None,
        };

        cache.cache_store("key-1".to_string(), output.clone(), 3600);

        let stats = cache.get_cache_stats();
        assert_eq!(stats.current_entries, 1);

        let retrieved = cache.cache_lookup("key-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().model_id, "model-1");

        let stats = cache.get_cache_stats();
        assert_eq!(stats.total_hits, 1);
    }

    #[test]
    fn test_cache_miss() {
        let cache = InferenceCache::new();

        let retrieved = cache.cache_lookup("nonexistent");
        assert!(retrieved.is_none());

        let stats = cache.get_cache_stats();
        assert_eq!(stats.total_misses, 1);
    }

    #[test]
    fn test_cache_hit_rate() {
        let cache = InferenceCache::new();

        let output = InferenceOutput {
            model_id: "model-1".to_string(),
            data: json!({"result": "test"}),
            latency_ms: 100,
            success: true,
            error_message: None,
        };

        cache.cache_store("key-1".to_string(), output.clone(), 3600);

        cache.cache_lookup("key-1");
        cache.cache_lookup("key-1");
        cache.cache_lookup("key-1");
        cache.cache_lookup("nonexistent");

        let stats = cache.get_cache_stats();
        assert_eq!(stats.total_hits, 3);
        assert_eq!(stats.total_misses, 1);
        assert!((stats.hit_rate - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_cache_multiple_entries() {
        let cache = InferenceCache::new();

        for i in 0..5 {
            let output = InferenceOutput {
                model_id: format!("model-{}", i),
                data: json!({"result": i}),
                latency_ms: 100,
                success: true,
                error_message: None,
            };
            cache.cache_store(format!("key-{}", i), output, 3600);
        }

        let stats = cache.get_cache_stats();
        assert_eq!(stats.current_entries, 5);

        for i in 0..5 {
            let retrieved = cache.cache_lookup(&format!("key-{}", i));
            assert!(retrieved.is_some());
        }
    }

    #[test]
    fn test_cache_stats_zero_rate() {
        let cache = InferenceCache::new();
        let stats = cache.get_cache_stats();
        assert_eq!(stats.hit_rate, 0.0);
    }
}
