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

const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Anthropic Claude LLM 适配器
///
/// 实现与 Anthropic Claude API 的集成，支持 Claude 3 系列模型
///
/// # Examples
///
/// ```no_run
/// use aetheris::core::llm::{AnthropicLlmAdapter, LlmConfig};
///
/// let mut config = LlmConfig::default();
/// config.api_key = Some("your-api-key".to_string());
/// let adapter = AnthropicLlmAdapter::new(config).unwrap();
/// ```
pub struct AnthropicLlmAdapter {
    config: LlmConfig,
    client: Client,
}

impl AnthropicLlmAdapter {
    /// 创建一个新的 Anthropic 适配器
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
    /// use aetheris::core::llm::{AnthropicLlmAdapter, LlmConfig};
    ///
    /// let mut config = LlmConfig::default();
    /// config.api_key = Some("your-api-key".to_string());
    /// let adapter = AnthropicLlmAdapter::new(config).unwrap();
    /// ```
    pub fn new(config: LlmConfig) -> Result<Self> {
        if config.api_key.is_none() {
            return Err(AetherisError::Llm(
                "API key is required for Anthropic adapter".to_string(),
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
            .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string())
    }

    fn api_key(&self) -> String {
        self.config.api_key.clone().unwrap_or_default()
    }

    fn convert_request(&self, request: ChatRequest) -> AnthropicChatRequest {
        let mut system_prompt: Option<String> = None;
        let mut messages = Vec::new();

        for message in request.messages {
            match message.role {
                MessageRole::System => {
                    system_prompt = if let Some(existing) = system_prompt {
                        Some(format!("{}\n{}", existing, message.content))
                    } else {
                        Some(message.content)
                    };
                }
                MessageRole::User => messages.push(AnthropicMessage {
                    role: "user".to_string(),
                    content: message.content,
                }),
                MessageRole::Assistant => messages.push(AnthropicMessage {
                    role: "assistant".to_string(),
                    content: message.content,
                }),
            }
        }

        AnthropicChatRequest {
            model: request.model,
            messages,
            system: system_prompt,
            temperature: request.temperature,
            max_tokens: request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            top_p: request.top_p,
        }
    }

    fn convert_response(&self, response: AnthropicChatResponse) -> ChatResponse {
        let mut text_content = String::new();
        for block in response.content {
            if let Some(text) = block.text {
                if !text_content.is_empty() {
                    text_content.push('\n');
                }
                text_content.push_str(&text);
            }
        }

        ChatResponse {
            id: response.id,
            model: response.model,
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: MessageRole::Assistant,
                    content: text_content,
                },
                finish_reason: response.stop_reason,
            }],
            usage: response.usage.map(|usage| TokenUsage {
                prompt_tokens: usage.input_tokens,
                completion_tokens: usage.output_tokens,
                total_tokens: usage.input_tokens + usage.output_tokens,
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicChatRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicChatResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<AnthropicContentBlock>,
    model: String,
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[async_trait]
impl LlmAdapter for AnthropicLlmAdapter {
    fn provider(&self) -> LlmProvider {
        LlmProvider::Anthropic
    }

    fn config(&self) -> &LlmConfig {
        &self.config
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        info!("Anthropic LLM adapter processing chat request");
        debug!("Request model: {}", request.model);

        let anthropic_request = self.convert_request(request);
        let api_url = format!("{}/messages", self.api_base());

        debug!("Sending request to: {}", api_url);

        let response = self
            .client
            .post(&api_url)
            .header("x-api-key", self.api_key())
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("Content-Type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| AetherisError::Llm(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            warn!("Anthropic API error: {} - {}", status, error_text);
            return Err(AetherisError::Llm(format!(
                "Anthropic API error: {} - {}",
                status, error_text
            )));
        }

        let anthropic_response: AnthropicChatResponse = response.json().await.map_err(|e| {
            AetherisError::Llm(format!("Failed to parse Anthropic response: {}", e))
        })?;

        let chat_response = self.convert_response(anthropic_response);
        debug!("Anthropic response received: {:?}", chat_response);

        Ok(chat_response)
    }
}
