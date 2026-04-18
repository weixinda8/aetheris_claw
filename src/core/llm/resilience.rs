use crate::core::llm::{ChatRequest, ChatResponse, LlmAdapter, LlmConfig, LlmProvider};
use crate::utils::{AetherisError, Result};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// 熔断器状态
///
/// 表示熔断器的当前状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// 关闭状态，正常允许请求通过
    Closed,
    /// 打开状态，拒绝所有请求
    Open,
    /// 半开状态，允许少量请求以检测服务是否恢复
    HalfOpen,
}

/// 重试配置
///
/// 配置指数退避重试策略的参数
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 初始延迟时间
    pub initial_delay: Duration,
    /// 最大延迟时间
    pub max_delay: Duration,
    /// 退避因子，每次重试延迟乘以该值
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_factor: 2.0,
        }
    }
}

/// 熔断器配置
///
/// 配置熔断器的行为参数
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// 失败阈值，连续失败次数达到此值后熔断器打开
    pub failure_threshold: u32,
    /// 重置超时，熔断器打开后等待此时间后进入半开状态
    pub reset_timeout: Duration,
    /// 半开状态允许的最大请求数
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
            half_open_max_calls: 1,
        }
    }
}

/// 弹性配置
///
/// 组合重试和熔断器配置
#[derive(Debug, Clone, Default)]
pub struct ResilienceConfig {
    /// 重试配置
    pub retry: RetryConfig,
    /// 熔断器配置
    pub circuit_breaker: CircuitBreakerConfig,
}


struct CircuitBreakerInner {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    half_open_calls: u32,
}

impl CircuitBreakerInner {
    fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            half_open_calls: 0,
        }
    }
}

/// 熔断器
///
/// 实现熔断器模式，用于防止级联故障
///
/// 熔断器有三种状态：
/// - Closed: 正常状态，允许所有请求通过
/// - Open: 失败过多，拒绝所有请求
/// - HalfOpen: 冷却后尝试恢复，允许少量请求
///
/// # Examples
///
/// ```
/// use aetheris::core::llm::resilience::{CircuitBreaker, CircuitBreakerConfig};
/// use std::time::Duration;
///
/// let config = CircuitBreakerConfig {
///     failure_threshold: 3,
///     reset_timeout: Duration::from_secs(10),
///     half_open_max_calls: 1,
/// };
/// let cb = CircuitBreaker::new(config);
///
/// if cb.allow_request() {
///     // 执行请求
///     // 如果成功: cb.record.record
/// }
/// ```
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    inner: Arc<RwLock<CircuitBreakerInner>>,
}

impl CircuitBreaker {
    /// 创建一个新的熔断器
    ///
    /// # Arguments
    ///
    /// * `config` - 熔断器配置
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::resilience::{CircuitBreaker, CircuitBreakerConfig};
    ///
    /// let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
    /// ```
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            inner: Arc::new(RwLock::new(CircuitBreakerInner::new())),
        }
    }

    /// 获取当前熔断器状态
    ///
    /// # Returns
    ///
    /// 返回当前的 CircuitState
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::resilience::{CircuitBreaker, CircuitState};
    ///
    /// let cb = CircuitBreaker::default();
    /// assert_eq!(cb.state(), CircuitState::Closed);
    /// ```
    pub fn state(&self) -> CircuitState {
        self.inner.read().state
    }

    /// 检查是否允许请求通过
    ///
    /// 此方法会根据当前状态自动进行状态转换
    ///
    /// # Returns
    ///
    /// 如果允许请求返回 true，否则返回 false
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::resilience::CircuitBreaker;
    ///
    /// let cb = CircuitBreaker::default();
    /// assert!(cb.allow_request());
    /// ```
    pub fn allow_request(&self) -> bool {
        let mut inner = self.inner.write();

        match inner.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = inner.last_failure_time {
                    if last_failure.elapsed() >= self.config.reset_timeout {
                        inner.state = CircuitState::HalfOpen;
                        inner.half_open_calls = 0;
                        info!("Circuit breaker transitioning from Open to HalfOpen");
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => inner.half_open_calls < self.config.half_open_max_calls,
        }
    }

    /// 记录成功的请求
    ///
    /// 成功的请求会影响熔断器的状态转换
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::resilience::CircuitBreaker;
    ///
    /// let cb = CircuitBreaker::default();
    /// cb.record_record_success();
    /// ```
    pub fn record_success(&self) {
        let mut inner = self.inner.write();

        match inner.state {
            CircuitState::Closed => {
                inner.failure_count = 0;
                inner.success_count += 1;
            }
            CircuitState::HalfOpen => {
                inner.success_count += 1;
                inner.half_open_calls += 1;

                if inner.success_count >= self.config.half_open_max_calls {
                    inner.state = CircuitState::Closed;
                    inner.failure_count = 0;
                    inner.success_count = 0;
                    info!("Circuit breaker transitioning from HalfOpen to Closed");
                }
            }
            CircuitState::Open => {}
        }
    }

    /// 记录失败的请求
    ///
    /// 失败的请求会影响熔断器的状态转换
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::resilience::CircuitBreaker;
    ///
    /// let cb = CircuitBreaker::default();
    /// cb.record_failure();
    /// ```
    pub fn record_failure(&self) {
        let mut inner = self.inner.write();

        match inner.state {
            CircuitState::Closed => {
                inner.failure_count += 1;
                inner.last_failure_time = Some(Instant::now());

                if inner.failure_count >= self.config.failure_threshold {
                    inner.state = CircuitState::Open;
                    warn!(
                        "Circuit breaker transitioning from Closed to Open after {} failures",
                        inner.failure_count
                    );
                }
            }
            CircuitState::HalfOpen => {
                inner.state = CircuitState::Open;
                inner.last_failure_time = Some(Instant::now());
                inner.half_open_calls = 0;
                warn!("Circuit breaker transitioning from HalfOpen to Open");
            }
            CircuitState::Open => {
                inner.last_failure_time = Some(Instant::now());
            }
        }
    }
}

impl Default for CircuitBreaker {
    /// 创建使用默认配置的熔断器
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::resilience::CircuitBreaker;
    ///
    /// let cb = CircuitBreaker::default();
    /// ```
    fn default() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }
}

/// 指数退避
///
/// 实现指数退避重试策略，每次重试延迟会按指数增长
///
/// # Examples
///
/// ```
/// use aetheris::core::llm::resilience::{ExponentialBackoff, RetryConfig};
/// use std::time::Duration;
///
/// let config = RetryConfig {
///     max_retries: 3,
///     initial_delay: Duration::from_millis(100),
///     max_delay: Duration::from_secs(10),
///     backoff_factor: 2.0,
/// };
/// let backoff = ExponentialBackoff::new(config);
/// ```
pub struct ExponentialBackoff {
    config: RetryConfig,
}

impl ExponentialBackoff {
    /// 创建一个新的指数退避实例
    ///
    /// # Arguments
    ///
    /// * `config` - 重试配置
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::resilience::{ExponentialBackoff, RetryConfig};
    ///
    /// let backoff = ExponentialBackoff::new(RetryConfig::default());
    /// ```
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// 执行退避等待
    ///
    /// 根据尝试次数计算延迟并等待
    ///
    /// # Arguments
    ///
    /// * `attempt` - 当前尝试次数（从 0 开始）
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use aetheris::core::llm::resilience::ExponentialBackoff;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let backoff = ExponentialBackoff::default();
    /// backoff.sleep(1).await; // 等待计算出的延迟时间
    /// # }
    /// ```
    pub async fn sleep(&self, attempt: u32) {
        if attempt == 0 {
            return;
        }

        let delay_ms = self.config.initial_delay.as_millis() as f64
            * self.config.backoff_factor.powf(attempt as f64);
        let delay_ms = delay_ms.min(self.config.max_delay.as_millis() as f64);
        let delay = Duration::from_millis(delay_ms as u64);

        debug!(
            "Exponential backoff: sleeping for {:?} (attempt {})",
            delay, attempt
        );
        tokio::time::sleep(delay).await;
    }

    /// 获取最大重试次数
    ///
    /// # Returns
    ///
    /// 返回配置的最大重试次数
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::resilience::ExponentialBackoff;
    ///
    /// let backoff = ExponentialBackoff::default();
    /// assert_eq!(backoff.max_retries(), 3);
    /// ```
    pub fn max_retries(&self) -> u32 {
        self.config.max_retries
    }
}

impl Default for ExponentialBackoff {
    /// 创建使用默认配置的指数退避实例
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::resilience::ExponentialBackoff;
    ///
    /// let backoff = ExponentialBackoff::default();
    /// ```
    fn default() -> Self {
        Self::new(RetryConfig::default())
    }
}

/// 弹性 LLM 适配器
///
/// 包装其他 LLM 适配器，添加重试和熔断机制
///
/// # Examples
///
/// ```
/// use aetheris::core::llm::{MockLlmAdapter, LlmAdapter};
/// use aetheris::core::llm::resilience::{ResilientLlmAdapter, ResilienceConfig};
/// use std::sync::Arc;
///
/// let mock = MockLlmAdapter::new();
/// let resilient = ResilientLlmAdapter::with_default_config(Arc::new(mock));
/// ```
pub struct ResilientLlmAdapter {
    inner: Arc<dyn LlmAdapter>,
    circuit_breaker: CircuitBreaker,
    backoff: ExponentialBackoff,
}

impl ResilientLlmAdapter {
    /// 创建一个新的弹性适配器
    ///
    /// # Arguments
    ///
    /// * `inner` - 要包装的 LLM 适配器
    /// * `config` - 弹性配置
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{MockLlmAdapter, LlmAdapter};
    /// use aetheris::core::llm::resilience::{ResilientLlmAdapter, ResilienceConfig};
    /// use std::sync::Arc;
    ///
    /// let mock = MockLlmAdapter::new();
    /// let config = ResilienceConfig::default();
    /// let resilient = ResilientLlmAdapter::new(Arc::new(mock), config);
    /// ```
    pub fn new(inner: Arc<dyn LlmAdapter>, config: ResilienceConfig) -> Self {
        Self {
            inner,
            circuit_breaker: CircuitBreaker::new(config.circuit_breaker),
            backoff: ExponentialBackoff::new(config.retry),
        }
    }

    /// 创建使用默认配置的弹性适配器
    ///
    /// # Arguments
    ///
    /// * `inner` - 要包装的 LLM 适配器
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{MockLlmAdapter, LlmAdapter};
    /// use aetheris::core::llm::resilience::ResilientLlmAdapter;
    /// use std::sync::Arc;
    ///
    /// let mock = MockLlmAdapter::new();
    /// let resilient = ResilientLlmAdapter::with_default_config(Arc::new(mock));
    /// ```
    pub fn with_default_config(inner: Arc<dyn LlmAdapter>) -> Self {
        Self::new(inner, ResilienceConfig::default())
    }

    fn is_retryable_error(error: &AetherisError) -> bool {
        matches!(
            error,
            AetherisError::Llm(_) | AetherisError::Timeout(_) | AetherisError::Internal(_)
        )
    }

    async fn execute_with_retry<F, Fut>(&self, operation: F) -> Result<ChatResponse>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<ChatResponse>>,
    {
        let mut last_error = None;

        for attempt in 0..=self.backoff.max_retries() {
            if !self.circuit_breaker.allow_request() {
                error!("Circuit breaker is open, request denied");
                return Err(AetherisError::Llm(
                    "Circuit breaker is open, request denied".to_string(),
                ));
            }

            match operation().await {
                Ok(response) => {
                    self.circuit_breaker.record_success();
                    if attempt > 0 {
                        info!("Request succeeded on attempt {}", attempt + 1);
                    }
                    return Ok(response);
                }
                Err(error) => {
                    self.circuit_breaker.record_failure();
                    last_error = Some(error);

                    if attempt < self.backoff.max_retries() {
                        if Self::is_retryable_error(last_error.as_ref().unwrap()) {
                            warn!("Request failed on attempt {}, retrying...", attempt + 1);
                            self.backoff.sleep(attempt).await;
                        } else {
                            debug!("Non-retryable error, not retrying");
                            break;
                        }
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| AetherisError::Internal("Unknown error occurred".to_string())))
    }
}

#[async_trait]
impl LlmAdapter for ResilientLlmAdapter {
    fn provider(&self) -> LlmProvider {
        self.inner.provider()
    }

    fn config(&self) -> &LlmConfig {
        self.inner.config()
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let inner = self.inner.clone();
        self.execute_with_retry(|| inner.chat(request.clone()))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::llm::{ChatMessage, ChatRequest, ChatResponse, MockLlmAdapter};
    use std::sync::Arc;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        let cb = CircuitBreaker::default();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_request());
    }

    #[tokio::test]
    async fn test_circuit_breaker_open_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            reset_timeout: Duration::from_millis(100),
            half_open_max_calls: 1,
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_request());

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow_request());
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_after_timeout() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(50),
            half_open_max_calls: 1,
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        sleep(Duration::from_millis(60)).await;
        assert!(cb.allow_request());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_breaker_closed_after_half_open_success() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(50),
            half_open_max_calls: 1,
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        sleep(Duration::from_millis(60)).await;
        assert!(cb.allow_request());

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_exponential_backoff_sleep() {
        let config = RetryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            backoff_factor: 2.0,
        };
        let backoff = ExponentialBackoff::new(config);
        let start = Instant::now();
        backoff.sleep(1).await;
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(10));
        assert!(elapsed < Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_resilient_adapter_success() {
        let mock = MockLlmAdapter::default();
        let adapter = ResilientLlmAdapter::with_default_config(Arc::new(mock));

        let request = ChatRequest::new(
            "test-model".to_string(),
            vec![ChatMessage::user("Hello".to_string())],
        );

        let result = adapter.chat(request).await;
        assert!(result.is_ok());
    }

    struct FailingLlmAdapter {
        fail_count: Arc<RwLock<u32>>,
    }

    impl FailingLlmAdapter {
        fn new() -> Self {
            Self {
                fail_count: Arc::new(RwLock::new(0)),
            }
        }
    }

    #[async_trait]
    impl LlmAdapter for FailingLlmAdapter {
        fn provider(&self) -> LlmProvider {
            LlmProvider::Mock
        }

        fn config(&self) -> &LlmConfig {
            &LlmConfig::default()
        }

        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse> {
            let mut fail_count = self.fail_count.write();
            *fail_count += 1;
            Err(AetherisError::Llm("Simulated failure".to_string()))
        }
    }

    #[tokio::test]
    async fn test_resilient_adapter_retry_on_failure() {
        let failing = FailingLlmAdapter::new();
        let fail_count_clone = failing.fail_count.clone();

        let config = ResilienceConfig {
            retry: RetryConfig {
                max_retries: 2,
                initial_delay: Duration::from_millis(10),
                max_delay: Duration::from_millis(100),
                backoff_factor: 2.0,
            },
            circuit_breaker: CircuitBreakerConfig::default(),
        };

        let adapter = ResilientLlmAdapter::new(Arc::new(failing), config);

        let request = ChatRequest::new(
            "test-model".to_string(),
            vec![ChatMessage::user("Hello".to_string())],
        );

        let result = adapter.chat(request).await;
        assert!(result.is_err());

        let final_fail_count = *fail_count_clone.read();
        assert_eq!(*final_fail_count, 3);
    }
}
