use crate::utils::{AetherisError, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub value: T,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub hits: u64,
}

impl<T> CacheEntry<T> {
    pub fn new(value: T, ttl: Option<Duration>) -> Self {
        let now = Utc::now();
        Self {
            value,
            created_at: now,
            expires_at: ttl.and_then(|t| {
                chrono::Duration::from_std(t)
                    .ok()
                    .and_then(|d| now.checked_add_signed(d))
            }),
            hits: 0,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|e| Utc::now() >= e).unwrap_or(false)
    }

    pub fn hit(&mut self) {
        self.hits += 1;
    }

    pub fn age(&self) -> Duration {
        Utc::now()
            .signed_duration_since(self.created_at)
            .to_std()
            .unwrap_or(Duration::from_secs(0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheTier {
    L1,
    L2,
    L3,
}

pub trait CacheBackend: Send + Sync {
    fn get<T: DeserializeOwned + Clone>(&self, key: &str) -> Option<CacheEntry<T>>;
    fn set<T: Serialize + Clone>(&self, key: &str, value: CacheEntry<T>) -> Result<()>;
    fn remove(&self, key: &str) -> Result<()>;
    fn clear(&self) -> Result<()>;
    fn contains_key(&self, key: &str) -> bool;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct InMemoryCache {
    entries: Arc<DashMap<String, Vec<u8>>>,
    max_size: usize,
    current_size: Arc<std::sync::atomic::AtomicUsize>,
}

impl InMemoryCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            max_size,
            current_size: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    fn ensure_size(&self, new_size: usize) {
        let mut current = self.current_size.load(std::sync::atomic::Ordering::Relaxed);
        while current + new_size > self.max_size && !self.entries.is_empty() {
            if let Some(entry) = self.entries.iter().next() {
                let _ = self.entries.remove(entry.key());
            }
            current = self.current_size.load(std::sync::atomic::Ordering::Relaxed);
        }
    }
}

impl CacheBackend for InMemoryCache {
    fn get<T: DeserializeOwned + Clone>(&self, key: &str) -> Option<CacheEntry<T>> {
        self.entries.get(key).and_then(|entry| {
            let result: std::result::Result<CacheEntry<T>, _> =
                bincode::deserialize(entry.value());
            result.ok()
        })
    }

    fn set<T: Serialize + Clone>(&self, key: &str, value: CacheEntry<T>) -> Result<()> {
        let serialized =
            bincode::serialize(&value).map_err(|e| AetherisError::Bincode(e.to_string()))?;
        let size = serialized.len();

        self.ensure_size(size);

        self.entries.insert(key.to_string(), serialized);
        self.current_size
            .fetch_add(size, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }

    fn remove(&self, key: &str) -> Result<()> {
        if let Some((_, serialized)) = self.entries.remove(key) {
            let size = serialized.len();
            self.current_size
                .fetch_sub(size, std::sync::atomic::Ordering::Relaxed);
        }
        Ok(())
    }

    fn clear(&self) -> Result<()> {
        self.entries.clear();
        self.current_size
            .store(0, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

pub struct FileCache {
    cache_dir: PathBuf,
    max_files: usize,
}

impl FileCache {
    pub fn new(cache_dir: PathBuf, max_files: usize) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            cache_dir,
            max_files,
        })
    }

    fn get_file_path(&self, key: &str) -> PathBuf {
        let hashed = format!("{:x}", md5::compute(key.as_bytes()));
        self.cache_dir.join(hashed)
    }

    fn ensure_file_count(&self) -> Result<()> {
        let mut files: Vec<_> = std::fs::read_dir(&self.cache_dir)?
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                e.metadata()
                    .ok()
                    .and_then(|m| m.modified().ok().map(|t| (e.path(), t)))
            })
            .collect();

        files.sort_by_key(|(_, t)| *t);

        while files.len() > self.max_files {
            if let Some((path, _)) = files.pop() {
                let _ = std::fs::remove_file(path);
            }
        }

        Ok(())
    }
}

impl CacheBackend for FileCache {
    fn get<T: DeserializeOwned + Clone>(&self, key: &str) -> Option<CacheEntry<T>> {
        let path = self.get_file_path(key);
        if !path.exists() {
            return None;
        }

        let content = std::fs::read(path).ok()?;
        let result: std::result::Result<CacheEntry<T>, _> = bincode::deserialize(&content);
        result.ok()
    }

    fn set<T: Serialize + Clone>(&self, key: &str, value: CacheEntry<T>) -> Result<()> {
        self.ensure_file_count()?;

        let path = self.get_file_path(key);
        let serialized =
            bincode::serialize(&value).map_err(|e| AetherisError::Bincode(e.to_string()))?;
        std::fs::write(path, serialized)?;

        Ok(())
    }

    fn remove(&self, key: &str) -> Result<()> {
        let path = self.get_file_path(key);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    fn clear(&self) -> Result<()> {
        for entry in std::fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let _ = std::fs::remove_file(path);
            }
        }
        Ok(())
    }

    fn contains_key(&self, key: &str) -> bool {
        self.get_file_path(key).exists()
    }

    fn len(&self) -> usize {
        std::fs::read_dir(&self.cache_dir)
            .map(|entries| entries.filter_map(|e| e.ok()).count())
            .unwrap_or(0)
    }
}

pub struct MultiLayerCache {
    l1: Arc<InMemoryCache>,
    l2: Arc<FileCache>,
    stats: Arc<CacheStats>,
    l1_ttl: Duration,
    l2_ttl: Duration,
}

#[derive(Debug, Default)]
pub struct CacheStats {
    pub l1_hits: std::sync::atomic::AtomicU64,
    pub l1_misses: std::sync::atomic::AtomicU64,
    pub l2_hits: std::sync::atomic::AtomicU64,
    pub l2_misses: std::sync::atomic::AtomicU64,
    pub l3_hits: std::sync::atomic::AtomicU64,
    pub l3_misses: std::sync::atomic::AtomicU64,
    pub total_writes: std::sync::atomic::AtomicU64,
    pub total_evictions: std::sync::atomic::AtomicU64,
}

impl CacheStats {
    pub fn hit_rate(&self, tier: CacheTier) -> f64 {
        let (hits, misses) = match tier {
            CacheTier::L1 => (
                self.l1_hits.load(std::sync::atomic::Ordering::Relaxed),
                self.l1_misses.load(std::sync::atomic::Ordering::Relaxed),
            ),
            CacheTier::L2 => (
                self.l2_hits.load(std::sync::atomic::Ordering::Relaxed),
                self.l2_misses.load(std::sync::atomic::Ordering::Relaxed),
            ),
            CacheTier::L3 => (
                self.l3_hits.load(std::sync::atomic::Ordering::Relaxed),
                self.l3_misses.load(std::sync::atomic::Ordering::Relaxed),
            ),
        };

        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    pub fn reset(&self) {
        self.l1_hits.store(0, std::sync::atomic::Ordering::Relaxed);
        self.l1_misses
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.l2_hits.store(0, std::sync::atomic::Ordering::Relaxed);
        self.l2_misses
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.l3_hits.store(0, std::sync::atomic::Ordering::Relaxed);
        self.l3_misses
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.total_writes
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.total_evictions
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

impl MultiLayerCache {
    pub fn new(l1_max_size: usize, l2_cache_dir: PathBuf, l2_max_files: usize, l1_ttl: Duration, l2_ttl: Duration) -> Result<Self> {
        Ok(Self {
            l1: Arc::new(InMemoryCache::new(l1_max_size)),
            l2: Arc::new(FileCache::new(l2_cache_dir, l2_max_files)?),
            stats: Arc::new(CacheStats::default()),
            l1_ttl,
            l2_ttl,
        })
    }

    pub fn get_ttl(&self, tier: CacheTier) -> Duration {
        match tier {
            CacheTier::L1 => self.l1_ttl,
            CacheTier::L2 => self.l2_ttl,
            CacheTier::L3 => Duration::from_secs(86400), // 24 hours for L3
        }
    }

    pub fn get<T: DeserializeOwned + Clone + Serialize>(&self, key: &str) -> Option<T> {
        if let Some(mut entry) = self.l1.get::<T>(key) {
            if !entry.is_expired() {
                self.stats
                    .l1_hits
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                entry.hit();
                let value = entry.value.clone();
                let _ = self.l1.set(key, entry);
                return Some(value);
            }
            self.stats
                .l1_misses
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let _ = self.l1.remove(key);
        } else {
            self.stats
                .l1_misses
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        if let Some(mut entry) = self.l2.get::<T>(key) {
            if !entry.is_expired() {
                self.stats
                    .l2_hits
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                entry.hit();
                let value = entry.value.clone();
                let _ = self.l1.set(key, entry.clone());
                let _ = self.l2.set(key, entry);
                return Some(value);
            }
            self.stats
                .l2_misses
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let _ = self.l2.remove(key);
        } else {
            self.stats
                .l2_misses
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        None
    }

    pub fn set<T: Serialize + Clone>(
        &self,
        key: &str,
        value: T,
        ttl: Option<Duration>,
    ) -> Result<()> {
        debug!("Setting cache entry: {}", key);
        let entry = CacheEntry::new(value, ttl);

        self.l1.set(key, entry.clone())?;
        self.l2.set(key, entry.clone())?;

        self.stats
            .total_writes
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }

    pub fn remove(&self, key: &str) -> Result<()> {
        info!("Removing cache entry: {}", key);
        self.l1.remove(key)?;
        self.l2.remove(key)?;

        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        info!("Clearing all cache entries");
        self.l1.clear()?;
        self.l2.clear()?;

        Ok(())
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.l1.contains_key(key) || self.l2.contains_key(key)
    }

    pub fn stats(&self) -> Arc<CacheStats> {
        self.stats.clone()
    }

    pub fn clear_l1(&self) -> Result<()> {
        self.l1.clear()
    }

    pub fn clear_l2(&self) -> Result<()> {
        self.l2.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_entry() {
        let entry = CacheEntry::new("test", Some(Duration::from_secs(60)));
        assert_eq!(entry.value, "test");
        assert!(!entry.is_expired());
        assert_eq!(entry.hits, 0);
    }

    #[test]
    fn test_in_memory_cache() {
        let cache = InMemoryCache::new(1024 * 1024);

        let entry = CacheEntry::new("test value", None);
        cache.set("test-key", entry).unwrap();

        let retrieved: Option<CacheEntry<String>> = cache.get("test-key");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().value, "test value");

        assert!(cache.contains_key("test-key"));
        assert_eq!(cache.len(), 1);

        cache.remove("test-key").unwrap();
        assert!(!cache.contains_key("test-key"));
    }

    #[test]
    fn test_file_cache() {
        let dir = tempdir().unwrap();
        let cache = FileCache::new(dir.path().to_path_buf(), 100).unwrap();

        let entry = CacheEntry::new("test value", None);
        cache.set("test-key", entry).unwrap();

        let retrieved: Option<CacheEntry<String>> = cache.get("test-key");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().value, "test value");

        cache.clear().unwrap();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_multi_layer_cache() {
        let dir = tempdir().unwrap();
        let cache = MultiLayerCache::new(1024 * 1024, dir.path().to_path_buf(), 100).unwrap();

        cache.set("test-key", "test value", None).unwrap();

        let retrieved = cache.get::<String>("test-key");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), "test value");

        assert!(cache.contains_key("test-key"));

        let stats = cache.stats();
        assert_eq!(stats.l1_hits.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(
            stats.l1_misses.load(std::sync::atomic::Ordering::Relaxed),
            1
        );
    }
}
