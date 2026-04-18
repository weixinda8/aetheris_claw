use crate::core::llm::adapter::{ChatRequest, ChatResponse, LlmAdapter, LlmConfig, LlmProvider};
use crate::core::llm::anthropic::AnthropicLlmAdapter;
use crate::core::llm::azure::AzureOpenAiLlmAdapter;
use crate::core::llm::deepseek::DeepSeekLlmAdapter;
use crate::core::llm::mock::MockLlmAdapter;
use crate::core::llm::openai::OpenAiLlmAdapter;
use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use std::sync::Arc;
use tracing::info;

/// LLM 管理器
///
/// 管理多个 LLM 适配器，支持注册、获取和切换不同的 LLM 提供商
///
/// # Examples
///
/// ```
/// use aetheris::core::llm::{LlmManager, LlmConfig, LlmProvider, MockLlmAdapter};
/// use std::sync::Arc;
///
/// let mut manager = LlmManager::new();
/// let mock_adapter = Arc::new(MockLlmAdapter::new());
/// manager.register_adapter(mock_adapter);
///
/// // 或者从配置创建
/// let config = LlmConfig::default();
/// let manager = LlmManager::from_config(config).unwrap();
/// ```
pub struct LlmManager {
    adapters: DashMap<LlmProvider, Arc<dyn LlmAdapter>>,
    default_provider: LlmProvider,
}

impl LlmManager {
    /// 创建一个新的 LLM 管理器
    ///
    /// 默认使用 Mock 提供商
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::LlmManager;
    ///
    /// let manager = LlmManager::new();
    /// ```
    pub fn new() -> Self {
        Self {
            adapters: DashMap::new(),
            default_provider: LlmProvider::DeepSeek,
        }
    }

    /// 设置默认提供商
    ///
    /// # Arguments
    ///
    /// * `provider` - 要设置为默认的提供商
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{LlmManager, LlmProvider};
    ///
    /// let manager = LlmManager::new()
    ///     .with_default_provider(LlmProvider::OpenAi);
    /// ```
    pub fn with_default_provider(mut self, provider: LlmProvider) -> Self {
        self.default_provider = provider;
        self
    }

    /// 注册一个 LLM 适配器
    ///
    /// # Arguments
    ///
    /// * `adapter` - 要注册的适配器，包装在 Arc 中
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{LlmManager, MockLlmAdapter};
    /// use std::sync::Arc;
    ///
    /// let manager = LlmManager::new();
    /// let adapter = Arc::new(MockLlmAdapter::new());
    /// manager.register_adapter(adapter);
    /// ```
    pub fn register_adapter(&self, adapter: Arc<dyn LlmAdapter>) {
        let provider = adapter.provider();
        info!("Registering LLM adapter: {:?}", provider);
        self.adapters.insert(provider, adapter);
    }

    /// 获取指定提供商的适配器
    ///
    /// # Arguments
    ///
    /// * `provider` - 要获取的提供商
    ///
    /// # Errors
    ///
    /// 如果未找到该提供商的适配器，返回错误
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{LlmManager, LlmProvider, MockLlmAdapter};
    /// use std::sync::Arc;
    ///
    /// let manager = LlmManager::new();
    /// let adapter = Arc::new(MockLlmAdapter::new());
    /// manager.register_adapter(adapter);
    ///
    /// let mock_adapter = manager.get_adapter(&LlmProvider::Mock).unwrap();
    /// ```
    pub fn get_adapter(&self, provider: &LlmProvider) -> Result<Arc<dyn LlmAdapter>> {
        self.adapters
            .get(provider)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| {
                AetherisError::Llm(format!(
                    "LLM adapter not found for provider: {:?}",
                    provider
                ))
            })
    }

    /// 获取默认适配器
    ///
    /// # Errors
    ///
    /// 如果未找到默认提供商的适配器，返回错误
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{LlmManager, MockLlmAdapter};
    /// use std::sync::Arc;
    ///
    /// let manager = LlmManager::new();
    /// let adapter = Arc::new(MockLlmAdapter::new());
    /// manager.register_adapter(adapter);
    ///
    /// let default_adapter = manager.get_default_adapter().unwrap();
    /// ```
    pub fn get_default_adapter(&self) -> Result<Arc<dyn LlmAdapter>> {
        self.get_adapter(&self.default_provider)
    }

    /// 使用默认适配器发送聊天请求
    ///
    /// # Arguments
    ///
    /// * `request` - 聊天请求
    ///
    /// # Errors
    ///
    /// 如果未找到默认适配器或请求失败，返回错误
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use aetheris::core::llm::{LlmManager, ChatRequest, ChatMessage, MockLlmAdapter};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let manager = LlmManager::new();
    ///     let adapter = Arc::new(MockLlmAdapter::new());
    ///     manager.register_adapter(adapter);
    ///
    ///     let request = ChatRequest::new(
    ///         "gpt-4".to_string(),
    ///         vec![ChatMessage::user("Hello".to_string())]
    ///     );
    ///     let response = manager.chat(request).await.unwrap();
    /// }
    /// ```
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let adapter = self.get_default_adapter()?;
        adapter.chat(request).await
    }

    /// 使用指定提供商发送聊天请求
    ///
    /// # Arguments
    ///
    /// * `provider` - 要使用的提供商
    /// * `request` - 聊天请求
    ///
    /// # Errors
    ///
    /// 如果未找到指定提供商的适配器或请求失败，返回错误
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use aetheris::core::llm::{LlmManager, ChatRequest, ChatMessage, LlmProvider, MockLlmAdapter};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let manager = LlmManager::new();
    ///     let adapter = Arc::new(MockLlmAdapter::new());
    ///     manager.register_adapter(adapter);
    ///
    ///     let request = ChatRequest::new(
    ///         "gpt-4".to_string(),
    ///         vec![ChatMessage::user("Hello".to_string())]
    ///     );
    ///     let response = manager.chat_with_provider(&LlmProvider::Mock, request).await.unwrap();
    /// }
    /// ```
    pub async fn chat_with_provider(
        &self,
        provider: &LlmProvider,
        request: ChatRequest,
    ) -> Result<ChatResponse> {
        let adapter = self.get_adapter(provider)?;
        adapter.chat(request).await
    }

    /// 使用默认适配器发送带系统提示的聊天请求
    ///
    /// # Arguments
    ///
    /// * `system_prompt` - 系统提示词
    /// * `user_message` - 用户消息
    ///
    /// # Errors
    ///
    /// 如果未找到默认适配器或请求失败，返回错误
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use aetheris::core::llm::{LlmManager, MockLlmAdapter};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let manager = LlmManager::new();
    ///     let adapter = Arc::new(MockLlmAdapter::new());
    ///     manager.register_adapter(adapter);
    ///
    ///     let response = manager.chat_with_system_prompt(
    ///         "You are helpful".to_string(),
    ///         "Hello".to_string()
    ///     ).await.unwrap();
    /// }
    /// ```
    pub async fn chat_with_system_prompt(
        &self,
        system_prompt: String,
        user_message: String,
    ) -> Result<ChatResponse> {
        let adapter = self.get_default_adapter()?;
        adapter
            .chat_with_system_prompt(system_prompt, user_message)
            .await
    }

    /// 使用指定提供商发送带系统提示的聊天请求
    ///
    /// # Arguments
    ///
    /// * `provider` - 要使用的提供商
    /// * `system_prompt` - 系统提示词
    /// * `user_message` - 用户消息
    ///
    /// # Errors
    ///
    /// 如果未找到指定提供商的适配器或请求失败，返回错误
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use aetheris::core::llm::{LlmManager, LlmProvider, MockLlmAdapter};
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let manager = LlmManager::new();
    ///     let adapter = Arc::new(MockLlmAdapter::new());
    ///     manager.register_adapter(adapter);
    ///
    ///     let response = manager.chat_with_system_prompt_and_provider(
    ///         &LlmProvider::Mock,
    ///         "You are helpful".to_string(),
    ///         "Hello".to_string()
    ///     ).await.unwrap();
    /// }
    /// ```
    pub async fn chat_with_system_prompt_and_provider(
        &self,
        provider: &LlmProvider,
        system_prompt: String,
        user_message: String,
    ) -> Result<ChatResponse> {
        let adapter = self.get_adapter(provider)?;
        adapter
            .chat_with_system_prompt(system_prompt, user_message)
            .await
    }

    /// 设置默认提供商
    ///
    /// # Arguments
    ///
    /// * `provider` - 要设置为默认的提供商
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{LlmManager, LlmProvider};
    ///
    /// let mut manager = LlmManager::new();
    /// manager.set_default_provider(LlmProvider::OpenAi);
    /// ```
    pub fn set_default_provider(&mut self, provider: LlmProvider) {
        info!("Setting default LLM provider: {:?}", provider);
        self.default_provider = provider;
    }

    /// 列出所有已注册的提供商
    ///
    /// # Returns
    ///
    /// 返回所有已注册提供商的列表
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{LlmManager, MockLlmAdapter};
    /// use std::sync::Arc;
    ///
    /// let manager = LlmManager::new();
    /// let adapter = Arc::new(MockLlmAdapter::new());
    /// manager.register_adapter(adapter);
    ///
    /// let providers = manager.list_providers();
    /// ```
    pub fn list_providers(&self) -> Vec<LlmProvider> {
        self.adapters
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// 检查是否已注册指定提供商
    ///
    /// # Arguments
    ///
    /// * `provider` - 要检查的提供商
    ///
    /// # Returns
    ///
    /// 如果已注册返回 true，否则返回 false
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{LlmManager, LlmProvider, MockLlmAdapter};
    /// use std::sync::Arc;
    ///
    /// let manager = LlmManager::new();
    /// let adapter = Arc::new(MockLlmAdapter::new());
    /// manager.register_adapter(adapter);
    ///
    /// assert!(manager.has_provider(&LlmProvider::Mock));
    /// ```
    pub fn has_provider(&self, provider: &LlmProvider) -> bool {
        self.adapters.contains_key(provider)
    }
}

impl LlmManager {
    /// 从配置创建 LLM 管理器
    ///
    /// 会自动根据配置创建并注册相应的适配器
    ///
    /// # Arguments
    ///
    /// * `config` - LLM 配置
    ///
    /// # Errors
    ///
    /// 如果适配器创建失败，返回错误
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{LlmManager, LlmConfig};
    ///
    /// let config = LlmConfig::default();
    /// let manager = LlmManager::from_config(config).unwrap();
    /// ```
    pub fn from_config(config: LlmConfig) -> Result<Self> {
        let manager = Self::new().with_default_provider(config.provider.clone());
        manager.register_adapter_from_config(config)?;
        Ok(manager)
    }

    /// 从配置注册适配器
    ///
    /// # Arguments
    ///
    /// * `config` - LLM 配置
    ///
    /// # Errors
    ///
    /// 如果适配器创建失败，返回错误
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{LlmManager, LlmConfig};
    ///
    /// let manager = LlmManager::new();
    /// let config = LlmConfig::default();
    /// manager.register_adapter_from_config(config).unwrap();
    /// ```
    pub fn register_adapter_from_config(&self, config: LlmConfig) -> Result<()> {
        let adapter: Arc<dyn LlmAdapter> = match config.provider {
            LlmProvider::Mock => Arc::new(MockLlmAdapter::with_config(config)),
            LlmProvider::OpenAi => Arc::new(OpenAiLlmAdapter::new(config)?),
            LlmProvider::Anthropic => Arc::new(AnthropicLlmAdapter::new(config)?),
            LlmProvider::AzureOpenAi => Arc::new(AzureOpenAiLlmAdapter::new(config)?),
            LlmProvider::DeepSeek => Arc::new(DeepSeekLlmAdapter::new(config)?),
            LlmProvider::Custom(_) => {
                return Err(AetherisError::Llm(
                    "Custom adapter requires manual registration".to_string(),
                ));
            }
        };
        self.register_adapter(adapter);
        Ok(())
    }
}

impl LlmManager {
    pub fn adapters(&self) -> Vec<LlmProvider> {
        self.list_providers()
    }
}

impl Default for LlmManager {
    fn default() -> Self {
        Self::new()
    }
}
