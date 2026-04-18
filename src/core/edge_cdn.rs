use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CdnProvider {
    Cloudflare,
    Akamai,
    Fastly,
    AwsCloudFront,
    AzureCdn,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdnConfig {
    pub provider: CdnProvider,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub endpoint: Option<String>,
    pub zone_id: Option<String>,
    pub distribution_id: Option<String>,
    pub cache_ttl_seconds: u64,
    pub enabled: bool,
    pub purge_on_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedContent {
    pub content_id: String,
    pub content_type: String,
    pub content_hash: String,
    pub size_bytes: u64,
    pub cached_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub etag: String,
    pub last_modified: chrono::DateTime<chrono::Utc>,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub origin_url: Option<String>,
    pub cdn_url: Option<String>,
    pub provider: CdnProvider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePurgeRequest {
    pub purge_id: String,
    pub content_ids: Vec<String>,
    pub patterns: Vec<String>,
    pub purge_all: bool,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub status: PurgeStatus,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PurgeStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdnStats {
    pub total_cache_hits: u64,
    pub total_cache_misses: u64,
    pub hit_rate: f64,
    pub total_bytes_served: u64,
    pub total_bytes_origin: u64,
    pub bandwidth_saved_bytes: u64,
    pub active_cached_items: usize,
    pub total_purges: u64,
    pub successful_purges: u64,
    pub failed_purges: u64,
    pub avg_latency_ms: f64,
    pub peak_requests_per_second: u64,
}

pub struct EdgeCdnManager {
    config: CdnConfig,
    cache: Arc<DashMap<String, CachedContent>>,
    purge_requests: Arc<DashMap<String, CachePurgeRequest>>,
    stats: Arc<std::sync::RwLock<CdnStats>>,
    storage_path: PathBuf,
}

impl EdgeCdnManager {
    pub fn new(config: CdnConfig, storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;

        let manager = Self {
            config,
            cache: Arc::new(DashMap::new()),
            purge_requests: Arc::new(DashMap::new()),
            stats: Arc::new(std::sync::RwLock::new(CdnStats {
                total_cache_hits: 0,
                total_cache_misses: 0,
                hit_rate: 0.0,
                total_bytes_served: 0,
                total_bytes_origin: 0,
                bandwidth_saved_bytes: 0,
                active_cached_items: 0,
                total_purges: 0,
                successful_purges: 0,
                failed_purges: 0,
                avg_latency_ms: 0.0,
                peak_requests_per_second: 0,
            })),
            storage_path,
        };

        manager.load()?;
        Ok(manager)
    }

    fn save(&self) -> Result<()> {
        let cache_path = self.storage_path.join("cache.json");
        let cache: Vec<_> = self.cache.iter().map(|e| e.value().clone()).collect();
        std::fs::write(&cache_path, serde_json::to_string_pretty(&cache)?)?;

        let purge_requests_path = self.storage_path.join("purge_requests.json");
        let purge_requests: Vec<_> = self
            .purge_requests
            .iter()
            .map(|e| e.value().clone())
            .collect();
        std::fs::write(
            &purge_requests_path,
            serde_json::to_string_pretty(&purge_requests)?,
        )?;

        let stats_path = self.storage_path.join("stats.json");
        let stats = self.stats.read().unwrap().clone();
        std::fs::write(&stats_path, serde_json::to_string_pretty(&stats)?)?;

        Ok(())
    }

    fn load(&self) -> Result<()> {
        let cache_path = self.storage_path.join("cache.json");
        if cache_path.exists() {
            let content = std::fs::read_to_string(&cache_path)?;
            let cache: Vec<CachedContent> = serde_json::from_str(&content)?;
            for item in cache {
                self.cache.insert(item.content_id.clone(), item);
            }
        }

        let purge_requests_path = self.storage_path.join("purge_requests.json");
        if purge_requests_path.exists() {
            let content = std::fs::read_to_string(&purge_requests_path)?;
            let purge_requests: Vec<CachePurgeRequest> = serde_json::from_str(&content)?;
            for req in purge_requests {
                self.purge_requests.insert(req.purge_id.clone(), req);
            }
        }

        let stats_path = self.storage_path.join("stats.json");
        if stats_path.exists() {
            let content = std::fs::read_to_string(&stats_path)?;
            let stats: CdnStats = serde_json::from_str(&content)?;
            let mut w = self.stats.write().unwrap();
            *w = stats;
        }

        Ok(())
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub async fn cache_content(
        &self,
        content_id: String,
        content_type: String,
        content_data: Vec<u8>,
        origin_url: Option<String>,
    ) -> Result<CachedContent> {
        if !self.config.enabled {
            return Err(AetherisError::Cdn("CDN is not enabled".to_string()));
        }

        info!("Caching content: {} ({})", content_id, content_type);

        let content_hash = Self::compute_hash(&content_data);
        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::seconds(self.config.cache_ttl_seconds as i64);
        let etag = format!("\"{}\"", content_hash);

        let cached_content = CachedContent {
            content_id: content_id.clone(),
            content_type,
            content_hash,
            size_bytes: content_data.len() as u64,
            cached_at: now,
            expires_at,
            etag,
            last_modified: now,
            cache_hits: 0,
            cache_misses: 0,
            origin_url,
            cdn_url: self.generate_cdn_url(&content_id),
            provider: self.config.provider.clone(),
        };

        self.cache
            .insert(content_id.clone(), cached_content.clone());

        self.upload_to_cdn(&content_id, content_data).await?;

        self.update_stats_after_cache(&cached_content);
        self.save()?;

        Ok(cached_content)
    }

    pub async fn get_content(&self, content_id: &str) -> Result<Option<CachedContent>> {
        if !self.config.enabled {
            return Ok(None);
        }

        if let Some(mut entry) = self.cache.get_mut(content_id) {
            let now = chrono::Utc::now();
            if entry.expires_at > now {
                entry.cache_hits += 1;
                self.update_stats_after_hit(entry.size_bytes);
                self.save()?;
                return Ok(Some(entry.value().clone()));
            } else {
                entry.cache_misses += 1;
                self.update_stats_after_miss();
                self.cache.remove(content_id);
                self.save()?;
            }
        }

        Ok(None)
    }

    pub async fn purge_content(&self, content_ids: Vec<String>) -> Result<CachePurgeRequest> {
        if !self.config.enabled {
            return Err(AetherisError::Cdn("CDN is not enabled".to_string()));
        }

        info!("Purging {} content items from CDN", content_ids.len());

        let purge_request = CachePurgeRequest {
            purge_id: uuid::Uuid::new_v4().to_string(),
            content_ids: content_ids.clone(),
            patterns: vec![],
            purge_all: false,
            requested_at: chrono::Utc::now(),
            status: PurgeStatus::Pending,
            completed_at: None,
        };

        self.purge_requests
            .insert(purge_request.purge_id.clone(), purge_request.clone());

        self.execute_purge(&purge_request).await?;
        self.save()?;

        Ok(purge_request)
    }

    pub async fn purge_by_pattern(&self, patterns: Vec<String>) -> Result<CachePurgeRequest> {
        if !self.config.enabled {
            return Err(AetherisError::Cdn("CDN is not enabled".to_string()));
        }

        info!("Purging content by patterns: {:?}", patterns);

        let purge_request = CachePurgeRequest {
            purge_id: uuid::Uuid::new_v4().to_string(),
            content_ids: vec![],
            patterns,
            purge_all: false,
            requested_at: chrono::Utc::now(),
            status: PurgeStatus::Pending,
            completed_at: None,
        };

        self.purge_requests
            .insert(purge_request.purge_id.clone(), purge_request.clone());

        self.execute_purge(&purge_request).await?;
        self.save()?;

        Ok(purge_request)
    }

    pub async fn purge_all(&self) -> Result<CachePurgeRequest> {
        if !self.config.enabled {
            return Err(AetherisError::Cdn("CDN is not enabled".to_string()));
        }

        info!("Purging all content from CDN");

        let purge_request = CachePurgeRequest {
            purge_id: uuid::Uuid::new_v4().to_string(),
            content_ids: vec![],
            patterns: vec![],
            purge_all: true,
            requested_at: chrono::Utc::now(),
            status: PurgeStatus::Pending,
            completed_at: None,
        };

        self.purge_requests
            .insert(purge_request.purge_id.clone(), purge_request.clone());

        self.execute_purge(&purge_request).await?;

        self.cache.clear();
        self.save()?;

        Ok(purge_request)
    }

    pub fn get_purge_status(&self, purge_id: &str) -> Option<CachePurgeRequest> {
        self.purge_requests.get(purge_id).map(|r| r.value().clone())
    }

    pub fn get_stats(&self) -> CdnStats {
        self.stats.read().unwrap().clone()
    }

    pub fn list_cached_content(&self, limit: Option<usize>) -> Vec<CachedContent> {
        let mut contents: Vec<CachedContent> = self
            .cache
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        contents.sort_by(|a, b| b.cached_at.cmp(&a.cached_at));

        if let Some(limit) = limit {
            contents.truncate(limit);
        }

        contents
    }

    pub fn get_cached_content_count(&self) -> usize {
        self.cache.len()
    }

    fn compute_hash(data: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    fn generate_cdn_url(&self, content_id: &str) -> Option<String> {
        self.config.endpoint.as_ref().map(|endpoint| format!("{}/{}", endpoint, content_id))
    }

    async fn upload_to_cdn(&self, content_id: &str, content_data: Vec<u8>) -> Result<()> {
        match self.config.provider {
            CdnProvider::Cloudflare => self.upload_to_cloudflare(content_id, content_data).await,
            CdnProvider::AwsCloudFront => self.upload_to_cloudfront(content_id, content_data).await,
            CdnProvider::Custom(_) => self.upload_to_custom_cdn(content_id, content_data).await,
            _ => {
                debug!(
                    "CDN provider {:?} upload not implemented yet",
                    self.config.provider
                );
                Ok(())
            }
        }
    }

    async fn upload_to_cloudflare(&self, content_id: &str, _content_data: Vec<u8>) -> Result<()> {
        if let (Some(_api_key), Some(_zone_id)) = (&self.config.api_key, &self.config.zone_id) {
            debug!("Uploading to Cloudflare CDN: {}", content_id);
        }
        Ok(())
    }

    async fn upload_to_cloudfront(&self, content_id: &str, _content_data: Vec<u8>) -> Result<()> {
        if let (Some(_api_key), Some(_distribution_id)) =
            (&self.config.api_key, &self.config.distribution_id)
        {
            debug!("Uploading to CloudFront CDN: {}", content_id);
        }
        Ok(())
    }

    async fn upload_to_custom_cdn(&self, content_id: &str, _content_data: Vec<u8>) -> Result<()> {
        if let Some(endpoint) = &self.config.endpoint {
            debug!("Uploading to custom CDN: {} at {}", content_id, endpoint);
        }
        Ok(())
    }

    async fn execute_purge(&self, purge_request: &CachePurgeRequest) -> Result<()> {
        if let Some(mut req) = self.purge_requests.get_mut(&purge_request.purge_id) {
            req.status = PurgeStatus::InProgress;
        }

        match self.config.provider {
            CdnProvider::Cloudflare => {
                self.purge_cloudflare(purge_request).await?;
            }
            CdnProvider::AwsCloudFront => {
                self.purge_cloudfront(purge_request).await?;
            }
            _ => {
                debug!(
                    "CDN provider {:?} purge not implemented yet",
                    self.config.provider
                );
            }
        }

        if let Some(mut req) = self.purge_requests.get_mut(&purge_request.purge_id) {
            req.status = PurgeStatus::Completed;
            req.completed_at = Some(chrono::Utc::now());
        }

        {
            let mut stats = self.stats.write().unwrap();
            stats.total_purges += 1;
            stats.successful_purges += 1;
        }

        self.save()?;

        Ok(())
    }

    async fn purge_cloudflare(&self, purge_request: &CachePurgeRequest) -> Result<()> {
        debug!("Purging Cloudflare CDN: {:?}", purge_request.purge_id);
        Ok(())
    }

    async fn purge_cloudfront(&self, purge_request: &CachePurgeRequest) -> Result<()> {
        debug!("Purging CloudFront CDN: {:?}", purge_request.purge_id);
        Ok(())
    }

    fn update_stats_after_cache(&self, _content: &CachedContent) {
        let mut stats = self.stats.write().unwrap();
        stats.active_cached_items = self.cache.len();
    }

    fn update_stats_after_hit(&self, bytes_served: u64) {
        let mut stats = self.stats.write().unwrap();
        stats.total_cache_hits += 1;
        stats.total_bytes_served += bytes_served;
        stats.bandwidth_saved_bytes += bytes_served;

        let total = stats.total_cache_hits + stats.total_cache_misses;
        if total > 0 {
            stats.hit_rate = stats.total_cache_hits as f64 / total as f64;
        }
    }

    fn update_stats_after_miss(&self) {
        let mut stats = self.stats.write().unwrap();
        stats.total_cache_misses += 1;

        let total = stats.total_cache_hits + stats.total_cache_misses;
        if total > 0 {
            stats.hit_rate = stats.total_cache_hits as f64 / total as f64;
        }
    }
}

impl Default for CdnConfig {
    fn default() -> Self {
        Self {
            provider: CdnProvider::Cloudflare,
            api_key: None,
            api_secret: None,
            endpoint: None,
            zone_id: None,
            distribution_id: None,
            cache_ttl_seconds: 86400,
            enabled: false,
            purge_on_update: true,
        }
    }
}

impl Default for EdgeCdnManager {
    fn default() -> Self {
        let storage_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".aetheris")
            .join("cdn-cache");

        Self::new(CdnConfig::default(), storage_path).unwrap_or_else(|_| {
            let temp_dir = tempfile::tempdir().unwrap();
            Self::new(CdnConfig::default(), temp_dir.path().to_path_buf()).unwrap()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cdn_manager_new() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = CdnConfig::default();
        let manager = EdgeCdnManager::new(config, temp_dir.path().to_path_buf());
        assert!(manager.is_ok());
    }

    #[test]
    fn test_cdn_config_default() {
        let config = CdnConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.cache_ttl_seconds, 86400);
    }

    #[test]
    fn test_compute_hash() {
        let data = b"test content";
        let hash1 = EdgeCdnManager::compute_hash(data);
        let hash2 = EdgeCdnManager::compute_hash(data);
        assert_eq!(hash1, hash2);
    }
}
