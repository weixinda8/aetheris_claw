use crate::core::llm::adapter::{ChatRequest, ChatResponse, LlmAdapter, LlmConfig};
use crate::utils::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hashlink::LruCache;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tracing::{debug, info, instrument};

/// 缓存配置
///
/// 配置 LLM 响应缓存的行为
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// 最大缓存条目数
    pub max_capacity: usize,
    /// 缓存条目 TTL（秒）
    pub ttl_seconds: u64,
    /// 是否启用缓存
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 100,
            ttl_seconds: 3600,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    response: ChatResponse,
    created_at: DateTime<Utc>,
}

/// 缓存统计信息
///
/// 记录缓存的命中、未命中和淘汰次数
#[derive(Debug, Default)]
pub struct CacheStats {
    /// 缓存命中次数
    pub hits: AtomicU64,
    /// 缓存未命中次数
    pub misses: AtomicU64,
    /// 缓存淘汰次数
    pub evictions: AtomicU64,
}

impl CacheStats {
    /// 创建新的缓存统计实例
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::cache::CacheStats;
    ///
    /// let stats = CacheStats::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// 计算缓存命中率
    ///
    /// # Returns
    ///
    /// 返回命中率（0.0-1.0）
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::cache::CacheStats;
    /// use std::sync::atomic::Ordering;
    ///
    /// let stats = CacheStats::new();
    /// stats.hits.store(7, Ordering::Relaxed);
    /// stats.misses.store(3, Ordering::Relaxed);
    ///
    /// assert!((stats.hit_rate() - 0.7).abs() < 0.001);
    /// ```
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let total = hits + self.misses.load(Ordering::Relaxed);
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// 重置所有统计数据
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::cache::CacheStats;
    /// use std::sync::atomic::Ordering;
    ///
    /// let stats = CacheStats::new();
    /// stats.hits.store(10, Ordering::Relaxed);
    /// stats.reset();
    /// assert_eq!(stats.hits.load(Ordering::Relaxed), 0);
    /// ```
    pub fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
    }
}

fn compute_cache_key(request: &ChatRequest) -> String {
    let mut hasher = Sha256::new();

    hasher.update(request.model.as_bytes());

    for msg in &request.messages {
        hasher.update(msg.role.to_string().as_bytes());
        hasher.update(msg.content.as_bytes());
    }

    if let Some(temp) = request.temperature {
        hasher.update(temp.to_be_bytes());
    }

    if let Some(max_tokens) = request.max_tokens {
        hasher.update(max_tokens.to_be_bytes());
    }

    if let Some(top_p) = request.top_p {
        hasher.update(top_p.to_be_bytes());
    }

    let result = hasher.finalize();
    hex::encode(result)
}

struct LlmCache {
    cache: Mutex<LruCache<String, CacheEntry>>,
    config: CacheConfig,
    stats: CacheStats,
}

impl LlmCache {
    fn new(config: CacheConfig) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(config.max_capacity)),
            config,
            stats: CacheStats::new(),
        }
    }

    fn get(&self, key: &str) -> Option<ChatResponse> {
        let mut cache = self.cache.lock();

        if let Some(entry) = cache.get(key) {
            if Utc::now().signed_duration_since(entry.created_at)
                < chrono::Duration::from_std(Duration::from_secs(self.config.ttl_seconds)).unwrap()
            {
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                debug!("Cache hit for key: {}", key);
                return Some(entry.response.clone());
            } else {
                cache.remove(key);
                self.stats.evictions.fetch_add(1, Ordering::Relaxed);
                debug!("Cache entry expired for key: {}", key);
            }
        }

        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        debug!("Cache miss for key: {}", key);
        None
    }

    fn put(&self, key: String, response: ChatResponse) {
        let mut cache = self.cache.lock();

        if cache.len() >= self.config.max_capacity {
            self.stats.evictions.fetch_add(1, Ordering::Relaxed);
        }

        cache.insert(
            key,
            CacheEntry {
                response,
                created_at: Utc::now(),
            },
        );
    }

    fn clear(&self) {
        let mut cache = self.cache.lock();
        cache.clear();
        self.stats.reset();
        info!("Cache cleared");
    }

    fn stats(&self) -> &CacheStats {
        &self.stats
    }
}

/// 带缓存的 LLM 适配器
///
/// 包装其他 LLM 适配器，添加响应缓存功能
///
/// 缓存键考虑以下因素：
/// - 模型名称
/// - 所有消息的角色和内容
/// - 温度参数
/// - 最大 token 数
/// - Top-p 参数
///
/// # Examples
///
/// ```
/// use aetheris::core::llm::{MockLlmAdapter, LlmAdapter};
/// use aetheris::core::llm::cache::{CachedLlmAdapter, CacheConfig};
///
/// let mock = MockLlmAdapter::new();
/// let cached = CachedLlmAdapter::with_default_config(mock);
/// ```
pub struct CachedLlmAdapter<T: LlmAdapter> {
    inner: T,
    cache: LlmCache,
}

impl<T: LlmAdapter> CachedLlmAdapter<T> {
    /// 创建一个新的带缓存的适配器
    ///
    /// # Arguments
    ///
    /// * `inner` - 要包装的 LLM 适配器
    /// * `config` - 缓存配置
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{MockLlmAdapter, LlmAdapter};
    /// use aetheris::core::llm::cache::{CachedLlmAdapter, CacheConfig};
    ///
    /// let mock = MockLlmAdapter::new();
    /// let config = CacheConfig::default();
    /// let cached = CachedLlmAdapter::new(mock, config);
    /// ```
    pub fn new(inner: T, config: CacheConfig) -> Self {
        Self {
            inner,
            cache: LlmCache::new(config),
        }
    }

    /// 创建使用默认缓存配置的适配器
    ///
    /// # Arguments
    ///
    /// * `inner` - 要包装的 LLM 适配器
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{MockLlmAdapter, LlmAdapter};
    /// use aetheris::core::llm::cache::CachedLlmAdapter;
    ///
    /// let mock = MockLlmAdapter::new();
    /// let cached = CachedLlmAdapter::with_default_config(mock);
    /// ```
    pub fn with_default_config(inner: T) -> Self {
        Self::new(inner, CacheConfig::default())
    }

    /// 清空缓存
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{MockLlmAdapter, LlmAdapter};
    /// use aetheris::core::llm::cache::CachedLlmAdapter;
    ///
    /// let mock = MockLlmAdapter::new();
    /// let cached = CachedLlmAdapter::with_default_config(mock);
    /// cached.clear_cache();
    /// ```
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// 获取缓存统计信息
    ///
    /// # Returns
    ///
    /// 返回缓存统计信息的引用
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{MockLlmAdapter, LlmAdapter};
    /// use aetheris::core::llm::cache::CachedLlmAdapter;
    ///
    /// let mock = MockLlmAdapter::new();
    /// let cached = CachedLlmAdapter::with_default_config(mock);
    /// let stats = cached.cache_stats();
    /// ```
    pub fn cache_stats(&self) -> &CacheStats {
        self.cache.stats()
    }

    /// 获取内部适配器
    ///
    /// # Returns
    ///
    /// 返回内部适配器的引用
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{MockLlmAdapter, LlmAdapter};
    /// use aetheris::core::llm::cache::CachedLlmAdapter;
    ///
    /// let mock = MockLlmAdapter::new();
    /// let cached = CachedLlmAdapter::with_default_config(mock);
    /// let inner = cached.inner();
    /// ```
    pub fn inner(&self) -> &T {
        &self.inner
    }
}

#[async_trait]
impl<T: LlmAdapter> LlmAdapter for CachedLlmAdapter<T> {
    fn provider(&self) -> crate::core::llm::adapter::LlmProvider {
        self.inner.provider()
    }

    fn config(&self) -> &LlmConfig {
        self.inner.config()
    }

    #[instrument(skip(self, request))]
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        if !self.cache.config.enabled {
            return self.inner.chat(request).await;
        }

        let key = compute_cache_key(&request);

        if let Some(cached) = self.cache.get(&key) {
            return Ok(cached);
        }

        let response = self.inner.chat(request).await?;
        self.cache.put(key, response.clone());
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::llm::adapter::{ChatMessage, ChatRequest, LlmAdapter};
    use crate::core::llm::mock::MockLlmAdapter;

    #[tokio::test]
    async fn test_cache_hit() {
        let mock = MockLlmAdapter::new();
        let cache_config = CacheConfig {
            max_capacity: 10,
            ttl_seconds: 60,
            enabled: true,
        };
        let cached = CachedLlmAdapter::new(mock, cache_config);

        let request = ChatRequest::new(
            "test-model".to_string(),
            vec![ChatMessage::user("Hello".to_string())],
        );

        let response1 = cached.chat(request.clone()).await.unwrap();
        let hits_before = cached.cache_stats().hits.load(Ordering::Relaxed);

        let response2 = cached.chat(request).await.unwrap();
        let hits_after = cached.cache_stats().hits.load(Ordering::Relaxed);

        assert_eq!(hits_after, hits_before + 1);
        assert_eq!(response1.id, response2.id);
    }

    #[tokio::test]
    async fn test_cache_disabled() {
        let mock = MockLlmAdapter::new();
        let cache_config = CacheConfig {
            enabled: false,
            ..Default::default()
        };
        let cached = CachedLlmAdapter::new(mock, cache_config);

        let request = ChatRequest::new(
            "test-model".to_string(),
            vec![ChatMessage::user("Hello".to_string())],
        );

        let _ = cached.chat(request.clone()).await.unwrap();
        let hits_before = cached.cache_stats().hits.load(Ordering::Relaxed);

        let _ = cached.chat(request).await.unwrap();
        let hits_after = cached.cache_stats().hits.load(Ordering::Relaxed);

        assert_eq!(hits_after, hits_before);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let mock = MockLlmAdapter::new();
        let cached = CachedLlmAdapter::with_default_config(mock);

        let request = ChatRequest::new(
            "test-model".to_string(),
            vec![ChatMessage::user("Hello".to_string())],
        );

        let _ = cached.chat(request.clone()).await.unwrap();
        let _ = cached.chat(request.clone()).await.unwrap();

        assert_eq!(cached.cache_stats().hits.load(Ordering::Relaxed), 1);

        cached.clear_cache();

        assert_eq!(cached.cache_stats().hits.load(Ordering::Relaxed), 0);

        let _ = cached.chat(request).await.unwrap();
        assert_eq!(cached.cache_stats().misses.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_cache_key_different_temperature() {
        let request1 = ChatRequest::new(
            "model".to_string(),
            vec![ChatMessage::user("test".to_string())],
        )
        .with_temperature(0.5);

        let request2 = ChatRequest::new(
            "model".to_string(),
            vec![ChatMessage::user("test".to_string())],
        )
        .with_temperature(0.8);

        let key1 = compute_cache_key(&request1);
        let key2 = compute_cache_key(&request2);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_cache_key_same_request() {
        let request1 = ChatRequest::new(
            "model".to_string(),
            vec![ChatMessage::user("test".to_string())],
        )
        .with_temperature(0.5)
        .with_top_p(0.9);

        let request2 = ChatRequest::new(
            "model".to_string(),
            vec![ChatMessage::user("test".to_string())],
        )
        .with_temperature(0.5)
        .with_top_p(0.9);

        let key1 = compute_cache_key(&request1);
        let key2 = compute_cache_key(&request2);

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let stats = CacheStats::new();
        stats.hits.store(7, Ordering::Relaxed);
        stats.misses.store(3, Ordering::Relaxed);

        assert!((stats.hit_rate() - 0.7).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let mock = MockLlmAdapter::new();
        let cache_config = CacheConfig {
            max_capacity: 2,
            ttl_seconds: 60,
            enabled: true,
        };
        let cached = CachedLlmAdapter::new(mock, cache_config);

        let request1 = ChatRequest::new(
            "model".to_string(),
            vec![ChatMessage::user("request 1".to_string())],
        );
        let request2 = ChatRequest::new(
            "model".to_string(),
            vec![ChatMessage::user("request 2".to_string())],
        );
        let request3 = ChatRequest::new(
            "model".to_string(),
            vec![ChatMessage::user("request 3".to_string())],
        );

        let _ = cached.chat(request1.clone()).await.unwrap();
        let _ = cached.chat(request2.clone()).await.unwrap();
        let _ = cached.chat(request3.clone()).await.unwrap();

        assert_eq!(cached.cache_stats().evictions.load(Ordering::Relaxed), 1);
    }
}
