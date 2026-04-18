use crate::skill::Skill;
use serde::{Serialize, Deserialize};
use std::fmt::Debug;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_load_time: Duration,
}

impl CacheStats {
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            evictions: 0,
            total_load_time: Duration::from_secs(0),
        }
    }
}

impl Default for CacheStats {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub max_size: usize,
    pub ttl_ms: Option<u64>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: 100,
            ttl_ms: None,
        }
    }
}

impl CacheConfig {
    pub fn new(max_size: usize, ttl_ms: Option<u64>) -> Self {
        Self { max_size, ttl_ms }
    }

    pub fn with_max_size(mut self, max_size: usize) -> Self {
        self.max_size = max_size;
        self
    }

    pub fn with_ttl_ms(mut self, ttl_ms: u64) -> Self {
        self.ttl_ms = Some(ttl_ms);
        self
    }
}

struct LruCacheEntry {
    skill: Arc<dyn Skill>,
    timestamp: Instant,
}

pub struct SkillLoadCache {
    cache: hashlink::LinkedHashMap<String, LruCacheEntry>,
    max_size: usize,
    ttl: Option<Duration>,
    stats: CacheStats,
}

impl SkillLoadCache {
    pub fn new(config: CacheConfig) -> Self {
        info!(
            "Initializing SkillLoadCache with max_size={}, ttl_ms={:?}",
            config.max_size, config.ttl_ms
        );
        let ttl = config.ttl_ms.map(Duration::from_millis);
        Self {
            cache: hashlink::LinkedHashMap::new(),
            max_size: config.max_size,
            ttl,
            stats: CacheStats::new(),
        }
    }

    pub fn get(&mut self, skill_id: &str) -> Option<Arc<dyn Skill>> {
        debug!("Getting skill from cache: {}", skill_id);

        if let Some(entry) = self.cache.remove(skill_id) {
            let is_expired = self
                .ttl
                .map(|ttl| entry.timestamp.elapsed() > ttl)
                .unwrap_or(false);

            if is_expired {
                warn!("Cache entry expired for skill: {}", skill_id);
                self.stats.misses += 1;
                self.stats.evictions += 1;
                None
            } else {
                debug!("Cache hit for skill: {}", skill_id);
                self.stats.hits += 1;
                let skill = entry.skill.clone();
                self.cache.insert(skill_id.to_string(), entry);
                Some(skill)
            }
        } else {
            debug!("Cache miss for skill: {}", skill_id);
            self.stats.misses += 1;
            None
        }
    }

    pub fn put(&mut self, skill_id: String, skill: Arc<dyn Skill>) {
        debug!("Putting skill into cache: {}", skill_id);

        if self.cache.len() >= self.max_size {
            if let Some((old_key, _)) = self.cache.pop_front() {
                info!("Evicting skill from cache: {}", old_key);
                self.stats.evictions += 1;
            }
        }

        self.cache.insert(
            skill_id,
            LruCacheEntry {
                skill,
                timestamp: Instant::now(),
            },
        );
    }

    pub fn remove(&mut self, skill_id: &str) {
        debug!("Removing skill from cache: {}", skill_id);
        if self.cache.remove(skill_id).is_some() {
            info!("Skill removed from cache: {}", skill_id);
        }
    }

    pub fn clear(&mut self) {
        info!("Clearing entire skill cache");
        self.cache.clear();
    }

    pub fn stats(&self) -> CacheStats {
        self.stats.clone()
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.stats.hits + self.stats.misses;
        if total == 0 {
            0.0
        } else {
            self.stats.hits as f64 / total as f64
        }
    }
}

impl Debug for SkillLoadCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillLoadCache")
            .field("cache_size", &self.cache.len())
            .field("max_size", &self.max_size)
            .field("ttl", &self.ttl)
            .field("stats", &self.stats)
            .finish()
    }
}

impl Clone for SkillLoadCache {
    fn clone(&self) -> Self {
        Self {
            cache: hashlink::LinkedHashMap::new(),
            max_size: self.max_size,
            ttl: self.ttl,
            stats: self.stats.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{BaseSkill, SkillMetadata, Version};

    fn create_test_skill(id: &str) -> Arc<dyn Skill> {
        let version = Version::new(1, 0, 0);
        let metadata = SkillMetadata::new(
            id.to_string(),
            "Test Skill".to_string(),
            version,
            "Test skill".to_string(),
        );
        BaseSkill::new_arc(metadata)
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.max_size, 100);
        assert!(config.ttl.is_none());
    }

    #[test]
    fn test_cache_config_with_max_size() {
        let config = CacheConfig::default().with_max_size(50);
        assert_eq!(config.max_size, 50);
    }

    #[test]
    fn test_cache_config_with_ttl() {
        let config = CacheConfig::default().with_ttl(Duration::from_secs(60));
        assert!(config.ttl.is_some());
    }

    #[test]
    fn test_cache_stats_new() {
        let stats = CacheStats::new();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.evictions, 0);
        assert_eq!(stats.total_load_time, Duration::from_secs(0));
    }

    #[test]
    fn test_cache_put_and_get() {
        let config = CacheConfig::default();
        let mut cache = SkillLoadCache::new(config);
        let skill = create_test_skill("test-skill-1");

        cache.put("test-skill-1".to_string(), skill.clone());
        let retrieved = cache.get("test-skill-1");

        assert!(retrieved.is_some());
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn test_cache_miss() {
        let config = CacheConfig::default();
        let mut cache = SkillLoadCache::new(config);

        let retrieved = cache.get("non-existent-skill");

        assert!(retrieved.is_none());
        assert_eq!(cache.stats().misses, 1);
    }

    #[test]
    fn test_cache_eviction() {
        let config = CacheConfig::default().with_max_size(2);
        let mut cache = SkillLoadCache::new(config);

        cache.put("skill-1".to_string(), create_test_skill("skill-1"));
        cache.put("skill-2".to_string(), create_test_skill("skill-2"));
        cache.put("skill-3".to_string(), create_test_skill("skill-3"));

        assert_eq!(cache.stats().evictions, 1);
        assert!(cache.get("skill-1").is_none());
        assert!(cache.get("skill-2").is_some());
        assert!(cache.get("skill-3").is_some());
    }

    #[test]
    fn test_cache_remove() {
        let config = CacheConfig::default();
        let mut cache = SkillLoadCache::new(config);

        cache.put("test-skill".to_string(), create_test_skill("test-skill"));
        assert!(cache.get("test-skill").is_some());

        cache.remove("test-skill");
        assert!(cache.get("test-skill").is_none());
    }

    #[test]
    fn test_cache_clear() {
        let config = CacheConfig::default();
        let mut cache = SkillLoadCache::new(config);

        cache.put("skill-1".to_string(), create_test_skill("skill-1"));
        cache.put("skill-2".to_string(), create_test_skill("skill-2"));

        cache.clear();

        assert!(cache.get("skill-1").is_none());
        assert!(cache.get("skill-2").is_none());
    }

    #[test]
    fn test_cache_hit_rate() {
        let config = CacheConfig::default();
        let mut cache = SkillLoadCache::new(config);

        cache.put("skill-1".to_string(), create_test_skill("skill-1"));

        cache.get("skill-1");
        cache.get("skill-1");
        cache.get("skill-2");

        assert!((cache.hit_rate() - 2.0 / 3.0).abs() < 0.0001);
    }
}
