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

/// Azure OpenAI LLM 适配器
///
/// 实现与 Azure OpenAI Service 的集成，支持使用 API Key 认证
///
/// # Examples
///
/// ```no_run
/// use aetheris::core::llm::{AzureOpenAiLlmAdapter, LlmConfig};
///
/// let mut config = LlmConfig::default();
/// config.api_key = Some("your-api-key".to_string());
/// config.api_base = Some("https://your-resource.openai.azure.com/openai/deployments/your-deployment".to_string());
/// let adapter = AzureOpenAiLlmAdapter::new(config).unwrap();
/// ```
pub struct AzureOpenAiLlmAdapter {
    config: LlmConfig,
    client: Client,
    deployment_name: String,
}

impl AzureOpenAiLlmAdapter {
    /// 创建一个新的 Azure OpenAI 适配器
    ///
    /// # Arguments
    ///
    /// * `config` - LLM 配置，必须包含 API key
    ///
    /// # Errors
    ///
    /// 如果没有提供 API key、部署名称或 HTTP 客户端创建失败，返回错误
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use aetheris::core::llm::{AzureOpenAiLlmAdapter, LlmConfig};
    ///
    /// let mut config = LlmConfig::default();
    /// config.api_key = Some("your-api-key".to_string());
    /// config.api_base = Some("https://your-resource.openai.azure.com/openai/deployments/your-deployment".to_string());
    /// let adapter = AzureOpenAiLlmAdapter::new(config).unwrap();
    /// ```
    pub fn new(config: LlmConfig) -> Result<Self> {
        if config.api_key.is_none() {
            return Err(AetherisError::Llm(
                "API key is required for Azure OpenAI adapter".to_string(),
            ));
        }

        let deployment_name = std::env::var("AZURE_OPENAI_DEPLOYMENT_NAME")
            .ok()
            .or_else(|| config.api_base.as_ref().and_then(|base| {
                base.split('/').rfind(|s| !s.is_empty())
                    .map(|s| s.to_string())
            }))
            .ok_or_else(|| {
                AetherisError::Llm(
                    "Deployment name is required. Set AZURE_OPENAI_DEPLOYMENT_NAME environment variable or include it in api_base".to_string(),
                )
            })?;

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| AetherisError::Llm(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            client,
            deployment_name,
        })
    }

    fn api_base(&self) -> String {
        self.config.api_base.clone().unwrap_or_else(|| {
            format!(
                "https://{}.openai.azure.com/openai/deployments/{}",
                std::env::var("AZURE_OPENAI_RESOURCE_NAME")
                    .unwrap_or_else(|_| "your-resource".to_string()),
                self.deployment_name
            )
        })
    }

    fn api_key(&self) -> String {
        self.config.api_key.clone().unwrap_or_default()
    }

    fn api_version(&self) -> String {
        std::env::var("AZURE_OPENAI_API_VERSION")
            .unwrap_or_else(|_| "2024-02-15-preview".to_string())
    }

    fn convert_message(&self, message: &ChatMessage) -> AzureOpenAiMessage {
        AzureOpenAiMessage {
            role: message.role.to_string(),
            content: message.content.clone(),
        }
    }

    fn convert_request(&self, request: ChatRequest) -> AzureOpenAiChatRequest {
        AzureOpenAiChatRequest {
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

    fn convert_response(&self, response: AzureOpenAiChatResponse) -> ChatResponse {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AzureOpenAiChatRequest {
    messages: Vec<AzureOpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AzureOpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AzureOpenAiChatResponse {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<AzureOpenAiChoice>,
    usage: Option<AzureOpenAiUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AzureOpenAiChoice {
    index: u32,
    message: AzureOpenAiMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AzureOpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[async_trait]
impl LlmAdapter for AzureOpenAiLlmAdapter {
    fn provider(&self) -> LlmProvider {
        LlmProvider::AzureOpenAi
    }

    fn config(&self) -> &LlmConfig {
        &self.config
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        info!("Azure OpenAI LLM adapter processing chat request");
        debug!("Request model: {}", request.model);

        let azure_request = self.convert_request(request);
        let api_url = format!(
            "{}/chat/completions?api-version={}",
            self.api_base(),
            self.api_version()
        );

        debug!("Sending request to: {}", api_url);

        let response = self
            .client
            .post(&api_url)
            .header("api-key", self.api_key())
            .header("Content-Type", "application/json")
            .json(&azure_request)
            .send()
            .await
            .map_err(|e| AetherisError::Llm(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            warn!("Azure OpenAI API error: {} - {}", status, error_text);
            return Err(AetherisError::Llm(format!(
                "Azure OpenAI API error: {} - {}",
                status, error_text
            )));
        }

        let azure_response: AzureOpenAiChatResponse = response.json().await.map_err(|e| {
            AetherisError::Llm(format!("Failed to parse Azure OpenAI response: {}", e))
        })?;

        let chat_response = self.convert_response(azure_response);
        debug!("Azure OpenAI response received: {:?}", chat_response);

        Ok(chat_response)
    }
}
