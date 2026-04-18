use crate::core::llm::adapter::{
    ChatChoice, ChatMessage, ChatRequest, ChatResponse, LlmAdapter, LlmConfig, LlmProvider,
    MessageRole, TokenUsage,
};
use crate::utils::Result;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Mutex;
use tracing::{debug, info};

/// 模拟 LLM 适配器
///
/// 用于测试和开发的模拟适配器，不需要真实的 API 调用
///
/// # Examples
///
/// ```
/// use aetheris::core::llm::{MockLlmAdapter, ChatRequest, ChatMessage};
///
/// let adapter = MockLlmAdapter::new();
/// adapter.add_mock_response("Hello, I'm a mock response!".to_string());
/// ```
pub struct MockLlmAdapter {
    config: LlmConfig,
    responses: Mutex<VecDeque<String>>,
}

impl MockLlmAdapter {
    /// 创建一个新的模拟适配器
    ///
    /// 使用默认配置
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::MockLlmAdapter;
    ///
    /// let adapter = MockLlmAdapter::new();
    /// ```
    pub fn new() -> Self {
        Self {
            config: LlmConfig {
                provider: LlmProvider::Mock,
                api_key: None,
                api_base: None,
                model: "mock-model".to_string(),
                temperature: 0.7,
                max_tokens: 2000,
                timeout_seconds: 30,
            },
            responses: Mutex::new(VecDeque::new()),
        }
    }

    /// 使用指定配置创建模拟适配器
    ///
    /// # Arguments
    ///
    /// * `config` - LLM 配置
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{MockLlmAdapter, LlmConfig};
    ///
    /// let config = LlmConfig::default();
    /// let adapter = MockLlmAdapter::with_config(config);
    /// ```
    pub fn with_config(config: LlmConfig) -> Self {
        Self {
            config,
            responses: Mutex::new(VecDeque::new()),
        }
    }

    /// 添加模拟响应
    ///
    /// 响应会按照添加顺序依次返回
    ///
    /// # Arguments
    ///
    /// * `response` - 要返回的响应文本
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::MockLlmAdapter;
    ///
    /// let adapter = MockLlmAdapter::new();
    /// adapter.add_mock_response("First response".to_string());
    /// adapter.add_mock_response("Second response".to_string());
    /// ```
    pub fn add_mock_response(&self, response: String) {
        self.responses.lock().unwrap().push_back(response);
    }

    /// 清空所有模拟响应
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::MockLlmAdapter;
    ///
    /// let adapter = MockLlmAdapter::new();
    /// adapter.add_mock_response("Test response".to_string());
    /// adapter.clear_mock_responses();
    /// ```
    pub fn clear_mock_responses(&self) {
        self.responses.lock().unwrap().clear();
    }

    fn generate_default_response(&self, user_message: &str) -> String {
        if user_message.to_lowercase().contains("task")
            || user_message.to_lowercase().contains("任务")
        {
            format!(
                "I understand you want to: {}. Let me break this down into actionable steps.",
                user_message
            )
        } else if user_message.to_lowercase().contains("plan")
            || user_message.to_lowercase().contains("计划")
        {
            format!(
                "Planning for: {}. Here's a step-by-step approach.",
                user_message
            )
        } else if user_message.to_lowercase().contains("intent")
            || user_message.to_lowercase().contains("意图")
        {
            format!(
                "Intent analysis for: {}. Identified key goals and constraints.",
                user_message
            )
        } else {
            format!(
                "Processing your request: {}. Here's what I can do for you.",
                user_message
            )
        }
    }
}

#[async_trait]
impl LlmAdapter for MockLlmAdapter {
    fn provider(&self) -> LlmProvider {
        LlmProvider::Mock
    }

    fn config(&self) -> &LlmConfig {
        &self.config
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        info!("Mock LLM adapter processing chat request");
        debug!(
            "Request model: {}, messages: {:?}",
            request.model, request.messages
        );

        let user_message = request
            .messages
            .iter()
            .find(|m| m.role == MessageRole::User)
            .map(|m| m.content.clone())
            .unwrap_or_else(|| "No user message".to_string());

        let content = if let Some(response) = self.responses.lock().unwrap().pop_front() {
            response
        } else {
            self.generate_default_response(&user_message)
        };

        let response = ChatResponse {
            id: format!("mock-{}", uuid::Uuid::new_v4()),
            model: request.model,
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage::assistant(content),
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(TokenUsage {
                prompt_tokens: user_message.len() as u32 / 4,
                completion_tokens: 100,
                total_tokens: user_message.len() as u32 / 4 + 100,
            }),
        };

        debug!("Mock LLM response: {:?}", response);
        Ok(response)
    }
}

impl Default for MockLlmAdapter {
    fn default() -> Self {
        Self::new()
    }
}
