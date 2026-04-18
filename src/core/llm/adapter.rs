use crate::utils::{AetherisError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 聊天消息角色
///
/// 表示一条聊天消息的发送者身份
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageRole {
    /// 系统消息，通常用于设置对话上下文和助手行为
    System,
    /// 用户消息，来自最终用户的输入
    User,
    /// 助手消息，来自 LLM 的响应
    Assistant,
}

impl fmt::Display for MessageRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageRole::System => write!(f, "system"),
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
        }
    }
}

/// 聊天消息
///
/// 表示单条聊天消息，包含角色和内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 消息角色
    pub role: MessageRole,
    /// 消息内容
    pub content: String,
}

impl ChatMessage {
    /// 创建一条新的聊天消息
    ///
    /// # Arguments
    ///
    /// * `role` - 消息角色
    /// * `content` - 消息内容
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{ChatMessage, MessageRole};
    ///
    /// let msg = ChatMessage::new(MessageRole::User, "Hello".to_string());
    /// ```
    pub fn new(role: MessageRole, content: String) -> Self {
        Self { role, content }
    }

    /// 创建一条系统消息
    ///
    /// # Arguments
    ///
    /// * `content` - 系统消息内容
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::ChatMessage;
    ///
    /// let msg = ChatMessage::system("You are a helpful assistant".to_string());
    /// ```
    pub fn system(content: String) -> Self {
        Self::new(MessageRole::System, content)
    }

    /// 创建一条用户消息
    ///
    /// # Arguments
    ///
    /// * `content` - 用户消息内容
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::ChatMessage;
    ///
    /// let msg = ChatMessage::user("Hello, how are you?".to_string());
    /// ```
    pub fn user(content: String) -> Self {
        Self::new(MessageRole::User, content)
    }

    /// 创建一条助手消息
    ///
    /// # Arguments
    ///
    /// * `content` - 助手消息内容
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::ChatMessage;
    ///
    /// let msg = ChatMessage::assistant("I'm doing well, thank you!".to_string());
    /// ```
    pub fn assistant(content: String) -> Self {
        Self::new(MessageRole::Assistant, content)
    }
}

/// 聊天请求
///
/// 表示发送给 LLM 的完整请求，包含模型、消息和参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// 要使用的模型名称
    pub model: String,
    /// 消息历史列表
    pub messages: Vec<ChatMessage>,
    /// 采样温度（0.0-2.0），越高越随机
    pub temperature: Option<f32>,
    /// 最大生成 token 数
    pub max_tokens: Option<u32>,
    /// Top-p 采样参数（0.0-1.0）
    pub top_p: Option<f32>,
}

impl ChatRequest {
    /// 创建一个新的聊天请求
    ///
    /// # Arguments
    ///
    /// * `model` - 模型名称
    /// * `messages` - 消息列表
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{ChatRequest, ChatMessage};
    ///
    /// let request = ChatRequest::new(
    ///     "gpt-4".to_string(),
    ///     vec![ChatMessage::user("Hello".to_string())]
    /// );
    /// ```
    pub fn new(model: String, messages: Vec<ChatMessage>) -> Self {
        Self {
            model,
            messages,
            temperature: None,
            max_tokens: None,
            top_p: None,
        }
    }

    /// 设置温度参数
    ///
    /// # Arguments
    ///
    /// * `temperature` - 温度值（0.0-2.0）
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{ChatRequest, ChatMessage};
    ///
    /// let request = ChatRequest::new("gpt-4".to_string(), vec![])
    ///     .with_temperature(0.7);
    /// ```
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// 设置最大 token 数
    ///
    /// # Arguments
    ///
    /// * `max_tokens` - 最大生成 token 数
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{ChatRequest, ChatMessage};
    ///
    /// let request = ChatRequest::new("gpt-4".to_string(), vec![])
    ///     .with_max_tokens(1000);
    /// ```
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// 设置 top-p 参数
    ///
    /// # Arguments
    ///
    /// * `top_p` - Top-p 值（0.0-1.0）
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{ChatRequest, ChatMessage};
    ///
    /// let request = ChatRequest::new("gpt-4".to_string(), vec![])
    ///     .with_top_p(0.9);
    /// ```
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }
}

/// 聊天响应
///
/// 表示从 LLM 返回的完整响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// 响应 ID
    pub id: String,
    /// 使用的模型
    pub model: String,
    /// 生成的选项列表
    pub choices: Vec<ChatChoice>,
    /// Token 使用统计
    pub usage: Option<TokenUsage>,
}

impl ChatResponse {
    pub fn content(&self) -> String {
        if let Some(choice) = self.choices.first() {
            choice.message.content.clone()
        } else {
            String::new()
        }
    }
}

/// 聊天选项
///
/// 表示单个生成的响应选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    /// 选项索引
    pub index: u32,
    /// 生成的消息
    pub message: ChatMessage,
    /// 完成原因
    pub finish_reason: Option<String>,
}

/// Token 使用统计
///
/// 表示一次请求的 Token 使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// 提示词 Token 数
    pub prompt_tokens: u32,
    /// 完成 Token 数
    pub completion_tokens: u32,
    /// 总 Token 数
    pub total_tokens: u32,
}

/// LLM 配置
///
/// 用于配置 LLM 适配器的各种参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// LLM 提供商
    pub provider: LlmProvider,
    /// API 密钥
    pub api_key: Option<String>,
    /// API 基础 URL
    pub api_base: Option<String>,
    /// 默认模型
    pub model: String,
    /// 默认温度
    pub temperature: f32,
    /// 默认最大 token 数
    pub max_tokens: u32,
    /// 请求超时时间（秒）
    pub timeout_seconds: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::DeepSeek,
            api_key: None,
            api_base: Some("https://api.deepseek.com/v1".to_string()),
            model: "deepseek-chat".to_string(),
            temperature: 0.7,
            max_tokens: 2000,
            timeout_seconds: 30,
        }
    }
}

impl LlmConfig {
    /// 从文件加载配置
    ///
    /// 支持 JSON、YAML 和 TOML 格式
    ///
    /// # Arguments
    ///
    /// * `path` - 配置文件路径
    ///
    /// # Errors
    ///
    /// 如果文件读取失败、格式不支持或解析失败，返回错误
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use aetheris::core::llm::LlmConfig;
    ///
    /// let config = LlmConfig::from_file("config.json").unwrap();
    /// ```
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AetherisError::Config(format!("Failed to read config file: {}", e)))?;

        let config: LlmConfig = if path.ends_with(".json") {
            serde_json::from_str(&content)
                .map_err(|e| AetherisError::Config(format!("Failed to parse JSON config: {}", e)))?
        } else if path.ends_with(".yaml") || path.ends_with(".yml") {
            serde_yaml::from_str(&content)
                .map_err(|e| AetherisError::Config(format!("Failed to parse YAML config: {}", e)))?
        } else if path.ends_with(".toml") {
            toml::from_str(&content)
                .map_err(|e| AetherisError::Config(format!("Failed to parse TOML config: {}", e)))?
        } else {
            return Err(AetherisError::Config(
                "Unsupported config file format. Use .json, .yaml, .yml, or .toml".to_string(),
            ));
        };

        Ok(config)
    }

    /// 从环境变量加载配置
    ///
    /// 支持以下环境变量：
    /// - `LLM_PROVIDER`: LLM 提供商
    /// - `LLM_API_KEY`: API 密钥
    /// - `LLM_API_BASE`: API 基础 URL
    /// - `LLM_MODEL`: 默认模型
    /// - `LLM_TEMPERATURE`: 默认温度
    /// - `LLM_MAX_TOKENS`: 默认最大 token 数
    /// - `LLM_TIMEOUT_SECONDS`: 请求超时时间
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::LlmConfig;
    ///
    /// let config = LlmConfig::from_env().unwrap();
    /// ```
    pub fn from_env() -> Result<Self> {
        let provider = std::env::var("LLM_PROVIDER")
            .map(|s| LlmProvider::from(s.as_str()))
            .unwrap_or(LlmProvider::DeepSeek);

        let api_key = std::env::var("LLM_API_KEY")
            .or_else(|_| std::env::var("DEEPSEEK_API_KEY"))
            .ok();
        let api_base = std::env::var("LLM_API_BASE")
            .or_else(|_| std::env::var("DEEPSEEK_API_BASE"))
            .ok()
            .or_else(|| Some("https://api.deepseek.com/v1".to_string()));
        let model = std::env::var("LLM_MODEL")
            .or_else(|_| std::env::var("DEEPSEEK_MODEL"))
            .unwrap_or_else(|_| "deepseek-chat".to_string());
        let temperature = std::env::var("LLM_TEMPERATURE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.7);
        let max_tokens = std::env::var("LLM_MAX_TOKENS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2000);
        let timeout_seconds = std::env::var("LLM_TIMEOUT_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        Ok(Self {
            provider,
            api_key,
            api_base,
            model,
            temperature,
            max_tokens,
            timeout_seconds,
        })
    }
}

/// LLM 提供商
///
/// 表示支持的 LLM 提供商类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LlmProvider {
    /// 模拟适配器，用于测试
    Mock,
    /// OpenAI 提供商
    OpenAi,
    /// Anthropic Claude 提供商
    Anthropic,
    /// Azure OpenAI 提供商
    AzureOpenAi,
    /// DeepSeek 提供商（默认）
    DeepSeek,
    /// 自定义提供商
    Custom(String),
}

/// LLM 适配器 trait
///
/// 所有 LLM 适配器必须实现的核心 trait
#[async_trait]
pub trait LlmAdapter: Send + Sync + 'static {
    /// 获取 LLM 提供商
    fn provider(&self) -> LlmProvider;

    /// 获取配置
    fn config(&self) -> &LlmConfig;

    /// 发送聊天请求
    ///
    /// # Arguments
    ///
    /// * `request` - 聊天请求
    ///
    /// # Returns
    ///
    /// 返回聊天响应或错误
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use aetheris::core::llm::{LlmAdapter, ChatRequest, ChatMessage, MockLlmAdapter};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let adapter = MockLlmAdapter::new();
    ///     let request = ChatRequest::new(
    ///         "gpt-4".to_string(),
    ///         vec![ChatMessage::user("Hello".to_string())]
    ///     );
    ///     let response = adapter.chat(request).await.unwrap();
    /// }
    /// ```
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;

    /// 发送带系统提示的聊天请求
    ///
    /// 这是一个便捷方法，会自动构造包含系统提示和用户消息的请求
    ///
    /// # Arguments
    ///
    /// * `system_prompt` - 系统提示词
    /// * `user_message` - 用户消息
    ///
    /// # Returns
    ///
    /// 返回聊天响应或错误
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use aetheris::core::llm::{LlmAdapter, MockLlmAdapter};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let adapter = MockLlmAdapter::new();
    ///     let response = adapter.chat_with_system_prompt(
    ///         "You are a helpful assistant".to_string(),
    ///         "Hello".to_string()
    ///     ).await.unwrap();
    /// }
    /// ```
    async fn chat_with_system_prompt(
        &self,
        system_prompt: String,
        user_message: String,
    ) -> Result<ChatResponse> {
        let messages = vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_message),
        ];

        let request = ChatRequest::new(self.config().model.clone(), messages)
            .with_temperature(self.config().temperature)
            .with_max_tokens(self.config().max_tokens);

        self.chat(request).await
    }
}

impl From<&str> for LlmProvider {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "mock" => LlmProvider::Mock,
            "openai" => LlmProvider::OpenAi,
            "anthropic" => LlmProvider::Anthropic,
            "azure" | "azureopenai" => LlmProvider::AzureOpenAi,
            "deepseek" => LlmProvider::DeepSeek,
            _ => LlmProvider::Custom(s.to_string()),
        }
    }
}

impl fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmProvider::Mock => write!(f, "mock"),
            LlmProvider::OpenAi => write!(f, "openai"),
            LlmProvider::Anthropic => write!(f, "anthropic"),
            LlmProvider::AzureOpenAi => write!(f, "azureopenai"),
            LlmProvider::DeepSeek => write!(f, "deepseek"),
            LlmProvider::Custom(s) => write!(f, "{}", s),
        }
    }
}
