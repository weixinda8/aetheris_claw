use aetheris::core::intent::IntentParser;
use aetheris::core::llm::{
    adapter::{
        ChatMessage, ChatRequest, ChatResponse, LlmAdapter, LlmConfig, LlmProvider, MessageRole,
        TokenUsage,
    },
    cache::{CacheConfig, CachedLlmAdapter},
    manager::LlmManager,
    mock::MockLlmAdapter,
    resilience::{ExponentialBackoff, ResilientLlmAdapter, RetryConfig},
    token_cost::{TokenCostLlmAdapter, TokenCostManager, TokenCostModelConfig},
};
use aetheris::utils::{AetherisError, Result};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Barrier;

#[tokio::test]
async fn test_mock_llm_adapter() {
    let mock_adapter = MockLlmAdapter::new();

    let request = ChatRequest::new(
        "test-model".to_string(),
        vec![ChatMessage::user("Test task".to_string())],
    );

    let response = LlmAdapter::chat(&mock_adapter, request).await;
    assert!(response.is_ok());

    let response = response.unwrap();
    assert!(!response.choices.is_empty());
    assert_eq!(response.choices[0].message.role, MessageRole::Assistant);
}

#[tokio::test]
async fn test_mock_llm_with_custom_response() {
    let mock_adapter = MockLlmAdapter::new();
    mock_adapter.add_mock_response("Custom response".to_string());

    let request = ChatRequest::new(
        "test-model".to_string(),
        vec![ChatMessage::user("Test".to_string())],
    );

    let response = LlmAdapter::chat(&mock_adapter, request).await.unwrap();
    assert_eq!(response.choices[0].message.content, "Custom response");
}

#[tokio::test]
async fn test_llm_manager() {
    let manager = LlmManager::new();
    let mock_adapter = Arc::new(MockLlmAdapter::new());
    manager.register_adapter(mock_adapter);

    assert!(manager.has_provider(&LlmProvider::Mock));

    let providers = manager.list_providers();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0], LlmProvider::Mock);

    let adapter = manager.get_default_adapter();
    assert!(adapter.is_ok());
}

#[tokio::test]
async fn test_llm_manager_chat() {
    let manager = LlmManager::new();
    let mock_adapter = Arc::new(MockLlmAdapter::new());
    manager.register_adapter(mock_adapter);

    let request = ChatRequest::new(
        "test-model".to_string(),
        vec![ChatMessage::user("Test task".to_string())],
    );

    let response = manager.chat(request).await;
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_llm_manager_chat_with_system_prompt() {
    let manager = LlmManager::new();
    let mock_adapter = Arc::new(MockLlmAdapter::new());
    manager.register_adapter(mock_adapter);

    let response = manager
        .chat_with_system_prompt("System prompt".to_string(), "User message".to_string())
        .await;
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_llm_config_default() {
    let config = LlmConfig::default();
    assert_eq!(config.provider, LlmProvider::Mock);
    assert_eq!(config.model, "gpt-4");
    assert_eq!(config.temperature, 0.7);
}

#[tokio::test]
async fn test_llm_config_from_provider_str() {
    assert_eq!(LlmProvider::from("mock"), LlmProvider::Mock);
    assert_eq!(LlmProvider::from("openai"), LlmProvider::OpenAi);
    assert_eq!(LlmProvider::from("anthropic"), LlmProvider::Anthropic);
    assert_eq!(LlmProvider::from("azureopenai"), LlmProvider::AzureOpenAi);
    assert_eq!(
        LlmProvider::from("custom"),
        LlmProvider::Custom("custom".to_string())
    );
}

#[tokio::test]
async fn test_intent_parser_with_llm() {
    let llm_manager = Arc::new(LlmManager::new());
    let mock_adapter = Arc::new(MockLlmAdapter::new());
    mock_adapter.add_mock_response(
        r#"{
        "goal": "Test task goal",
        "constraints": [],
        "requirements": [],
        "missing_information": [],
        "confidence_score": 80
    }"#
        .to_string(),
    );
    llm_manager.register_adapter(mock_adapter);

    let parser = IntentParser::new().with_llm_manager(llm_manager);
    let intent = parser.parse("Test task description").await;
    assert!(intent.is_ok());

    let intent = intent.unwrap();
    assert!(!intent.parsed_goal.is_empty());
}

#[tokio::test]
async fn test_switch_default_provider() {
    let mut manager = LlmManager::new();
    let mock_adapter = Arc::new(MockLlmAdapter::new());
    manager.register_adapter(mock_adapter);

    assert_eq!(manager.list_providers().len(), 1);

    manager.set_default_provider(LlmProvider::Mock);
    assert!(manager.get_default_adapter().is_ok());
}

#[tokio::test]
async fn test_llm_manager_from_config() {
    let config = LlmConfig::default();
    let manager = LlmManager::from_config(config);
    assert!(manager.is_ok());

    let manager = manager.unwrap();
    assert!(manager.has_provider(&LlmProvider::Mock));
}

#[cfg(test)]
mod resilience_tests {
    use super::*;

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

    #[tokio::test]
    async fn test_exponential_backoff_sleep() {
        let config = RetryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            backoff_factor: 2.0,
        };
        let backoff = ExponentialBackoff::new(config);
        let start = std::time::Instant::now();
        backoff.sleep(1).await;
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(10));
        assert!(elapsed < Duration::from_millis(100));
    }
}

#[cfg(test)]
mod token_cost_tests {
    use super::*;

    #[tokio::test]
    async fn test_register_model_cost() {
        let manager = TokenCostManager::new();
        let config = TokenCostModelConfig {
            prompt_cost_per_1k: 0.01,
            completion_cost_per_1k: 0.02,
        };
        manager.register_model_cost("test-model".to_string(), config.clone());
        let retrieved = manager.get_model_cost("test-model");
        assert_eq!(retrieved.prompt_cost_per_1k, 0.01);
        assert_eq!(retrieved.completion_cost_per_1k, 0.02);
    }

    #[tokio::test]
    async fn test_record_usage() {
        let manager = TokenCostManager::new();
        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 200,
            total_tokens: 300,
        };
        let record = manager
            .record_usage(
                LlmProvider::OpenAi,
                "gpt-4".to_string(),
                Some("task-1".to_string()),
                Some("user-1".to_string()),
                usage,
            )
            .unwrap();
        assert!(!record.id.is_empty());
        assert_eq!(record.provider, LlmProvider::OpenAi);
        assert_eq!(record.model, "gpt-4");
        assert!(record.cost > 0.0);
    }

    #[tokio::test]
    async fn test_can_proceed() {
        let manager = TokenCostManager::new();
        assert!(manager.can_proceed(None, None));
    }

    #[tokio::test]
    async fn test_get_stats_by_model() {
        let manager = TokenCostManager::new();
        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 200,
            total_tokens: 300,
        };
        manager
            .record_usage(
                LlmProvider::OpenAi,
                "gpt-4".to_string(),
                None,
                None,
                usage.clone(),
            )
            .unwrap();
        manager
            .record_usage(LlmProvider::OpenAi, "gpt-4".to_string(), None, None, usage)
            .unwrap();
        let (tokens, cost) = manager.get_stats_by_model("gpt-4");
        assert_eq!(tokens, 600);
        assert!(cost > 0.0);
    }

    #[tokio::test]
    async fn test_token_cost_llm_adapter() {
        let mock_adapter = Arc::new(MockLlmAdapter::default());
        let cost_manager = Arc::new(TokenCostManager::new());
        let adapter = TokenCostLlmAdapter::new(mock_adapter, cost_manager.clone());
        let messages = vec![ChatMessage::user("Hello".to_string())];
        let request = ChatRequest::new("gpt-4".to_string(), messages);
        let response = adapter.chat(request).await.unwrap();
        assert!(!response.id.is_empty());
        assert!(cost_manager.get_total_cost() > 0.0);
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use std::sync::atomic::Ordering;

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

    #[tokio::test]
    async fn test_cache_stats_hit_rate() {
        let mock = MockLlmAdapter::new();
        let cached = CachedLlmAdapter::with_default_config(mock);

        let request1 = ChatRequest::new(
            "model".to_string(),
            vec![ChatMessage::user("test1".to_string())],
        );
        let request2 = ChatRequest::new(
            "model".to_string(),
            vec![ChatMessage::user("test2".to_string())],
        );

        let _ = cached.chat(request1.clone()).await.unwrap();
        let _ = cached.chat(request1).await.unwrap();
        let _ = cached.chat(request2).await.unwrap();

        let hit_rate = cached.cache_stats().hit_rate();
        assert!((hit_rate - 1.0 / 3.0).abs() < 0.001);
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

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_resilience_and_cache_together() {
        let mock = MockLlmAdapter::default();
        let resilient = ResilientLlmAdapter::with_default_config(Arc::new(mock));
        let cached = CachedLlmAdapter::with_default_config(resilient);

        let request = ChatRequest::new(
            "test-model".to_string(),
            vec![ChatMessage::user("Hello".to_string())],
        );

        let response1 = cached.chat(request.clone()).await.unwrap();
        let response2 = cached.chat(request).await.unwrap();

        assert_eq!(response1.id, response2.id);
        assert!(
            cached
                .cache_stats()
                .hits
                .load(std::sync::atomic::Ordering::Relaxed)
                >= 1
        );
    }

    #[tokio::test]
    async fn test_cache_and_token_cost_together() {
        let mock = MockLlmAdapter::default();
        let cost_manager = Arc::new(TokenCostManager::new());
        let cost_adapter = TokenCostLlmAdapter::new(Arc::new(mock), cost_manager.clone());
        let cached = CachedLlmAdapter::with_default_config(cost_adapter);

        let request = ChatRequest::new(
            "gpt-4".to_string(),
            vec![ChatMessage::user("Hello".to_string())],
        );

        let response1 = cached.chat(request.clone()).await.unwrap();
        let total_cost_before = cost_manager.get_total_cost();

        let response2 = cached.chat(request).await.unwrap();
        let total_cost_after = cost_manager.get_total_cost();

        assert_eq!(response1.id, response2.id);
        assert_eq!(total_cost_before, total_cost_after);
    }

    #[tokio::test]
    async fn test_full_stack_integration() {
        let mock = MockLlmAdapter::default();
        let resilient = ResilientLlmAdapter::with_default_config(Arc::new(mock));
        let cost_manager = Arc::new(TokenCostManager::new());
        let cost_adapter = TokenCostLlmAdapter::new(Arc::new(resilient), cost_manager.clone());
        let cached = CachedLlmAdapter::with_default_config(cost_adapter);

        let request = ChatRequest::new(
            "gpt-4".to_string(),
            vec![ChatMessage::user("Hello".to_string())],
        );

        let result = cached.chat(request).await;
        assert!(result.is_ok());
        assert!(cost_manager.get_total_cost() > 0.0);
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_messages_request() {
        let mock = MockLlmAdapter::default();
        let request = ChatRequest::new("test-model".to_string(), vec![]);
        let result = mock.chat(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_extremely_long_message() {
        let long_message = "a".repeat(10000);
        let mock = MockLlmAdapter::default();
        let request = ChatRequest::new(
            "test-model".to_string(),
            vec![ChatMessage::user(long_message)],
        );
        let result = mock.chat(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cache_with_zero_ttl() {
        let mock = MockLlmAdapter::default();
        let cache_config = CacheConfig {
            max_capacity: 100,
            ttl_seconds: 0,
            enabled: true,
        };
        let cached = CachedLlmAdapter::new(mock, cache_config);

        let request = ChatRequest::new(
            "test-model".to_string(),
            vec![ChatMessage::user("Hello".to_string())],
        );

        let _ = cached.chat(request.clone()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        let _ = cached.chat(request).await.unwrap();

        assert_eq!(
            cached
                .cache_stats()
                .hits
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }
}

#[cfg(test)]
mod concurrency_tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_cache_access() {
        let mock = MockLlmAdapter::default();
        let cached = Arc::new(CachedLlmAdapter::with_default_config(mock));

        let request = ChatRequest::new(
            "test-model".to_string(),
            vec![ChatMessage::user("Hello".to_string())],
        );

        let mut handles = vec![];
        for _ in 0..10 {
            let cached_clone = Arc::clone(&cached);
            let request_clone = request.clone();
            handles.push(tokio::spawn(async move {
                cached_clone.chat(request_clone).await
            }));
        }

        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_concurrent_token_cost_tracking() {
        let mock = MockLlmAdapter::default();
        let cost_manager = Arc::new(TokenCostManager::new());
        let adapter = Arc::new(TokenCostLlmAdapter::new(
            Arc::new(mock),
            cost_manager.clone(),
        ));

        let mut handles = vec![];
        for i in 0..10 {
            let adapter_clone = Arc::clone(&adapter);
            let request = ChatRequest::new(
                "gpt-4".to_string(),
                vec![ChatMessage::user(format!("Hello {}", i))],
            );
            handles.push(tokio::spawn(
                async move { adapter_clone.chat(request).await },
            ));
        }

        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        assert!(cost_manager.get_total_cost() > 0.0);
    }

    #[tokio::test]
    async fn test_concurrent_full_stack() {
        let mock = MockLlmAdapter::default();
        let resilient = ResilientLlmAdapter::with_default_config(Arc::new(mock));
        let cost_manager = Arc::new(TokenCostManager::new());
        let cost_adapter = TokenCostLlmAdapter::new(Arc::new(resilient), cost_manager.clone());
        let cached = Arc::new(CachedLlmAdapter::with_default_config(cost_adapter));

        let barrier = Arc::new(Barrier::new(10));

        let mut handles = vec![];
        for i in 0..10 {
            let cached_clone = Arc::clone(&cached);
            let barrier_clone = Arc::clone(&barrier);
            let request = ChatRequest::new(
                "gpt-4".to_string(),
                vec![ChatMessage::user(format!("Message {}", i % 5))],
            );
            handles.push(tokio::spawn(async move {
                barrier_clone.wait().await;
                cached_clone.chat(request).await
            }));
        }

        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        assert!(cost_manager.get_total_cost() > 0.0);
    }
}

#[cfg(test)]
struct FailingLlmAdapter {
    pub fail_count: Arc<RwLock<u32>>,
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
        static CONFIG: std::sync::OnceLock<LlmConfig> = std::sync::OnceLock::new();
        CONFIG.get_or_init(LlmConfig::default)
    }

    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse> {
        let mut fail_count = self.fail_count.write();
        *fail_count += 1;
        Err(AetherisError::Llm("Simulated failure".to_string()))
    }
}

#[cfg(test)]
struct FailingThenSucceedAdapter {
    fail_until: u32,
    count: Arc<RwLock<u32>>,
}

impl FailingThenSucceedAdapter {
    fn new(fail_until: u32) -> Self {
        Self {
            fail_until,
            count: Arc::new(RwLock::new(0)),
        }
    }
}

#[async_trait]
impl LlmAdapter for FailingThenSucceedAdapter {
    fn provider(&self) -> LlmProvider {
        LlmProvider::Mock
    }

    fn config(&self) -> &LlmConfig {
        static CONFIG: std::sync::OnceLock<LlmConfig> = std::sync::OnceLock::new();
        CONFIG.get_or_init(LlmConfig::default)
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let current_count = {
            let mut count = self.count.write();
            *count += 1;
            *count
        };

        if current_count <= self.fail_until {
            Err(AetherisError::Llm("Simulated failure".to_string()))
        } else {
            MockLlmAdapter::default().chat(request).await
        }
    }
}

#[cfg(test)]
mod failing_adapter_tests {
    use super::*;

    #[tokio::test]
    async fn test_failing_llm_adapter() {
        let adapter = FailingLlmAdapter::new();
        let request = ChatRequest::new(
            "test-model".to_string(),
            vec![ChatMessage::user("Hello".to_string())],
        );

        let result = adapter.chat(request).await;
        assert!(result.is_err());
        assert_eq!(*adapter.fail_count.read(), 1);
    }

    #[tokio::test]
    async fn test_failing_then_succeed_adapter() {
        let adapter = FailingThenSucceedAdapter::new(2);
        let request = ChatRequest::new(
            "test-model".to_string(),
            vec![ChatMessage::user("Hello".to_string())],
        );

        let result1 = adapter.chat(request.clone()).await;
        assert!(result1.is_err());

        let result2 = adapter.chat(request.clone()).await;
        assert!(result2.is_err());

        let result3 = adapter.chat(request).await;
        assert!(result3.is_ok());
        assert_eq!(*adapter.count.read(), 3);
    }
}
