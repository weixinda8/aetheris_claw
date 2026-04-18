use crate::core::llm::adapter::{
    ChatChoice, ChatMessage, ChatRequest, ChatResponse, LlmAdapter, LlmConfig, LlmProvider,
    MessageRole, TokenUsage,
};
use crate::utils::{AetherisError, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeepSeekChatRequest {
    model: String,
    messages: Vec<DeepSeekMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeepSeekMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeepSeekChatResponse {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<DeepSeekChoice>,
    usage: Option<DeepSeekUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeepSeekChoice {
    index: u32,
    message: DeepSeekMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeepSeekUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// DeepSeek LLM 适配器
///
/// 实现与 DeepSeek API 的集成，支持 DeepSeek Chat、DeepSeek Coder 等模型
/// DeepSeek API 与 OpenAI API 完全兼容
///
/// # Examples
///
/// ```no_run
/// use aetheris::core::llm::{DeepSeekLlmAdapter, LlmConfig};
///
/// let mut config = LlmConfig::default();
/// config.api_key = Some("your-api-key".to_string());
/// let adapter = DeepSeekLlmAdapter::new(config).unwrap();
/// ```
pub struct DeepSeekLlmAdapter {
    config: LlmConfig,
    client: Client,
}

impl DeepSeekLlmAdapter {
    /// 创建一个新的 DeepSeek 适配器
    ///
    /// # Arguments
    ///
    /// * `config` - LLM 配置，必须包含 API key
    ///
    /// # Errors
    ///
    /// 如果没有提供 API key 或 HTTP 客户端创建失败，返回错误
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use aetheris::core::llm::{DeepSeekLlmAdapter, LlmConfig};
    ///
    /// let mut config = LlmConfig::default();
    /// config.api_key = Some("your-api-key".to_string());
    /// let adapter = DeepSeekLlmAdapter::new(config).unwrap();
    /// ```
    pub fn new(config: LlmConfig) -> Result<Self> {
        if config.api_key.is_none() {
            return Err(AetherisError::Llm(
                "API key is required for DeepSeek adapter".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| AetherisError::Llm(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    fn api_base(&self) -> String {
        self.config
            .api_base
            .clone()
            .unwrap_or_else(|| "https://api.deepseek.com/v1".to_string())
    }

    fn api_key(&self) -> String {
        self.config.api_key.clone().unwrap_or_default()
    }

    fn convert_message(&self, message: &ChatMessage) -> DeepSeekMessage {
        DeepSeekMessage {
            role: message.role.to_string(),
            content: message.content.clone(),
        }
    }

    fn convert_request(&self, request: ChatRequest) -> DeepSeekChatRequest {
        DeepSeekChatRequest {
            model: request.model,
            messages: request
                .messages
                .iter()
                .map(|m| self.convert_message(m))
                .collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
        }
    }

    fn convert_response(&self, response: DeepSeekChatResponse) -> ChatResponse {
        ChatResponse {
            id: response.id,
            model: response.model,
            choices: response
                .choices
                .into_iter()
                .map(|choice| ChatChoice {
                    index: choice.index,
                    message: ChatMessage {
                        role: match choice.message.role.as_str() {
                            "system" => MessageRole::System,
                            "user" => MessageRole::User,
                            "assistant" => MessageRole::Assistant,
                            _ => MessageRole::Assistant,
                        },
                        content: choice.message.content,
                    },
                    finish_reason: choice.finish_reason,
                })
                .collect(),
            usage: response.usage.map(|usage| TokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
            }),
        }
    }
}

#[async_trait]
impl LlmAdapter for DeepSeekLlmAdapter {
    fn provider(&self) -> LlmProvider {
        LlmProvider::DeepSeek
    }

    fn config(&self) -> &LlmConfig {
        &self.config
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        info!("DeepSeek LLM adapter processing chat request");
        debug!("Request model: {}", request.model);

        let deepseek_request = self.convert_request(request);
        let api_url = format!("{}/chat/completions", self.api_base());

        debug!("Sending request to: {}", api_url);

        let response = self
            .client
            .post(&api_url)
            .header("Authorization", format!("Bearer {}", self.api_key()))
            .header("Content-Type", "application/json")
            .json(&deepseek_request)
            .send()
            .await
            .map_err(|e| AetherisError::Llm(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            warn!("DeepSeek API error: {} - {}", status, error_text);
            return Err(AetherisError::Llm(format!(
                "DeepSeek API error: {} - {}",
                status, error_text
            )));
        }

        let deepseek_response: DeepSeekChatResponse = response
            .json()
            .await
            .map_err(|e| AetherisError::Llm(format!("Failed to parse DeepSeek response: {}", e)))?;

        let chat_response = self.convert_response(deepseek_response);
        debug!("DeepSeek response received: {:?}", chat_response);

        Ok(chat_response)
    }
}
