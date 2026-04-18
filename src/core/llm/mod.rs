//! LLM（大型语言模型）集成模块
//!
//! 本模块提供与多种 LLM 提供商的集成，支持：
//! - 核心适配器 trait 和数据结构
//! - 多提供商支持（OpenAI、Anthropic Claude、Azure OpenAI）
//! - 弹性机制（熔断器、指数退避重试）
//! - Token 成本管理
//! - 响应缓存
//! - 统一的 LLM 管理器
//!
//! # 快速开始
//!
//! ```
//! use aetheris::core::llm::{LlmManager, LlmConfig, MockLlmAdapter, ChatRequest, ChatMessage};
//! use std::sync::Arc;
//!
//! # #[tokio::main]
//! # async fn main() {
//! // 创建 LLM 管理器
//! let mut manager = LlmManager::new();
//!
//! // 注册模拟适配器
//! let mock_adapter = Arc::new(MockLlmAdapter::new());
//! manager.register_adapter(mock_adapter);
//!
//! // 发送聊天请求
//! let request = ChatRequest::new(
//!     "mock-model".to_string(),
//!     vec![ChatMessage::user("Hello!".to_string())]
//! );
//! let response = manager.chat(request).await.unwrap();
//! # }
//! ```

pub mod adapter;
pub mod anthropic;
pub mod azure;
pub mod cache;
pub mod deepseek;
pub mod manager;
pub mod mock;
pub mod openai;
pub mod resilience;
pub mod token_cost;

pub use adapter::{
    ChatMessage, ChatRequest, ChatResponse, LlmAdapter, LlmConfig, LlmProvider, TokenUsage,
};
pub use anthropic::AnthropicLlmAdapter;
pub use azure::AzureOpenAiLlmAdapter;
pub use cache::{CacheConfig, CacheStats, CachedLlmAdapter};
pub use deepseek::DeepSeekLlmAdapter;
pub use manager::LlmManager;
pub use mock::MockLlmAdapter;
pub use openai::OpenAiLlmAdapter;
pub use resilience::{
    CircuitBreaker, CircuitBreakerConfig, CircuitState, ExponentialBackoff, ResilienceConfig,
    ResilientLlmAdapter, RetryConfig,
};
pub use token_cost::{
    AlertHandler, BudgetAlert, BudgetAlertLevel, LogAlertHandler, TokenBudget, TokenCostLlmAdapter,
    TokenCostManager, TokenCostModelConfig, TokenCostRecord,
};
