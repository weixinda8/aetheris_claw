use crate::agent::config::config::{AgentConfigError, ConfigFormat};
use crate::agent::config::loader::AgentConfigLoader;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookMessage {
    pub message_id: String,
    pub from_platform: IMPlatform,
    pub from_user: String,
    pub to_agent: String,
    pub content: String,
    pub timestamp: i64,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IMPlatform {
    WeChatWork,
    DingTalk,
    FeiShu,
    WeChat,
    Unknown,
}

impl std::fmt::Display for IMPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IMPlatform::WeChatWork => write!(f, "wechat_work"),
            IMPlatform::DingTalk => write!(f, "dingtalk"),
            IMPlatform::FeiShu => write!(f, "feishu"),
            IMPlatform::WeChat => write!(f, "wechat"),
            IMPlatform::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WebhookConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub secret: Option<String>,
    pub verify_signature: bool,
}

#[async_trait]
pub trait WebhookHandler: Send + Sync {
    async fn handle_message(&self, message: WebhookMessage) -> Result<(), WebhookError>;
    async fn send_response(
        &self,
        message: WebhookMessage,
        response: String,
    ) -> Result<(), WebhookError>;
}

#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Agent config error: {0}")]
    AgentConfig(#[from] AgentConfigError),
    #[error("HTTP error: {0}")]
    Http(String),
}

pub struct WebhookManager {
    handlers: RwLock<HashMap<IMPlatform, Arc<dyn WebhookHandler>>>,
    config_loader: Option<AgentConfigLoader>,
    webhook_config: WebhookConfig,
}

impl WebhookManager {
    pub fn new(webhook_config: WebhookConfig) -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            config_loader: None,
            webhook_config,
        }
    }

    pub fn with_config_loader(mut self, config_loader: AgentConfigLoader) -> Self {
        self.config_loader = Some(config_loader);
        self
    }

    pub async fn register_handler(&self, platform: IMPlatform, handler: Arc<dyn WebhookHandler>) {
        let mut handlers = self.handlers.write().await;
        handlers.insert(platform, handler);
    }

    pub async fn process_webhook(
        &self,
        platform: IMPlatform,
        payload: &[u8],
    ) -> Result<(), WebhookError> {
        if !self.webhook_config.enabled {
            return Ok(());
        }

        let handlers = self.handlers.read().await;
        let handler = handlers
            .get(&platform)
            .ok_or_else(|| WebhookError::PlatformNotSupported(platform.to_string()))?;

        let message = self.parse_message(platform, payload)?;

        if let Some(loader) = &self.config_loader {
            let config_str = serde_json::to_string(&message)?;
            let agent_config = loader.load_from_str(&config_str, ConfigFormat::Json5)?;
            if !agent_config.meta.enabled {
                return Ok(());
            }
        }

        handler.handle_message(message).await?;
        Ok(())
    }

    fn parse_message(
        &self,
        platform: IMPlatform,
        payload: &[u8],
    ) -> Result<WebhookMessage, WebhookError> {
        match platform {
            IMPlatform::WeChatWork => self.parse_wechat_work_message(payload),
            IMPlatform::DingTalk => self.parse_dingtalk_message(payload),
            IMPlatform::FeiShu => self.parse_feishu_message(payload),
            IMPlatform::WeChat => self.parse_wechat_message(payload),
            IMPlatform::Unknown => Err(WebhookError::PlatformNotSupported("unknown".to_string())),
        }
    }

    fn parse_wechat_work_message(&self, payload: &[u8]) -> Result<WebhookMessage, WebhookError> {
        #[derive(Deserialize)]
        struct WeChatWorkMessage {
            #[serde(rename = "MsgId")]
            msg_id: String,
            #[serde(rename = "FromUserName")]
            from_user: String,
            #[serde(rename = "Content")]
            content: String,
            #[serde(rename = "CreateTime")]
            create_time: i64,
        }

        let wx_msg: WeChatWorkMessage = serde_json::from_slice(payload)?;

        Ok(WebhookMessage {
            message_id: wx_msg.msg_id,
            from_platform: IMPlatform::WeChatWork,
            from_user: wx_msg.from_user,
            to_agent: "default".to_string(),
            content: wx_msg.content,
            timestamp: wx_msg.create_time,
            metadata: None,
        })
    }

    fn parse_dingtalk_message(&self, payload: &[u8]) -> Result<WebhookMessage, WebhookError> {
        #[derive(Deserialize)]
        struct DingTalkMessage {
            #[serde(rename = "msgId")]
            message_id: String,
            #[serde(rename = "senderId")]
            sender_id: String,
            text: DingTalkText,
            #[serde(rename = "createAt")]
            create_at: i64,
        }

        #[derive(Deserialize)]
        struct DingTalkText {
            content: String,
        }

        let dt_msg: DingTalkMessage = serde_json::from_slice(payload)?;

        Ok(WebhookMessage {
            message_id: dt_msg.message_id,
            from_platform: IMPlatform::DingTalk,
            from_user: dt_msg.sender_id,
            to_agent: "default".to_string(),
            content: dt_msg.text.content,
            timestamp: dt_msg.create_at,
            metadata: None,
        })
    }

    fn parse_feishu_message(&self, payload: &[u8]) -> Result<WebhookMessage, WebhookError> {
        #[derive(Deserialize)]
        struct FeiShuMessage {
            uuid: String,
            sender: FeiShuSender,
            event: FeiShuEvent,
            ts: String,
        }

        #[derive(Deserialize)]
        struct FeiShuSender {
            sender_id: FeiShuSenderId,
        }

        #[derive(Deserialize)]
        struct FeiShuSenderId {
            open_id: String,
        }

        #[derive(Deserialize)]
        struct FeiShuEvent {
            message: FeiShuEventMessage,
        }

        #[derive(Deserialize)]
        struct FeiShuEventMessage {
            #[allow(dead_code)]
            message_id: String,
            content: String,
        }

        let fs_msg: FeiShuMessage = serde_json::from_slice(payload)?;

        Ok(WebhookMessage {
            message_id: fs_msg.uuid,
            from_platform: IMPlatform::FeiShu,
            from_user: fs_msg.sender.sender_id.open_id,
            to_agent: "default".to_string(),
            content: fs_msg.event.message.content,
            timestamp: fs_msg.ts.parse().unwrap_or(0),
            metadata: None,
        })
    }

    fn parse_wechat_message(&self, payload: &[u8]) -> Result<WebhookMessage, WebhookError> {
        #[derive(Deserialize)]
        struct WeChatMessage {
            msgid: String,
            fromusername: String,
            content: String,
            createtime: i64,
        }

        let wc_msg: WeChatMessage = serde_json::from_slice(payload)?;

        Ok(WebhookMessage {
            message_id: wc_msg.msgid,
            from_platform: IMPlatform::WeChat,
            from_user: wc_msg.fromusername,
            to_agent: "default".to_string(),
            content: wc_msg.content,
            timestamp: wc_msg.createtime,
            metadata: None,
        })
    }

    pub async fn send_message(&self, message: WebhookMessage) -> Result<(), WebhookError> {
        let handlers = self.handlers.read().await;
        let handler = handlers
            .get(&message.from_platform)
            .ok_or_else(|| WebhookError::PlatformNotSupported(message.from_platform.to_string()))?;

        handler.send_response(message, "".to_string()).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler;

    #[async_trait]
    impl WebhookHandler for TestHandler {
        async fn handle_message(&self, _message: WebhookMessage) -> Result<(), WebhookError> {
            Ok(())
        }

        async fn send_response(
            &self,
            _message: WebhookMessage,
            _response: String,
        ) -> Result<(), WebhookError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_webhook_manager_creation() {
        let config = WebhookConfig {
            enabled: true,
            endpoint: "/webhook".to_string(),
            secret: None,
            verify_signature: false,
        };

        let manager = WebhookManager::new(config);
        assert!(manager.webhook_config.enabled);
    }

    #[tokio::test]
    async fn test_register_handler() {
        let config = WebhookConfig {
            enabled: true,
            endpoint: "/webhook".to_string(),
            secret: None,
            verify_signature: false,
        };

        let manager = WebhookManager::new(config);
        let handler = Arc::new(TestHandler);
        manager
            .register_handler(IMPlatform::WeChatWork, handler)
            .await;

        let handlers = manager.handlers.read().await;
        assert!(handlers.contains_key(&IMPlatform::WeChatWork));
    }

    #[test]
    fn test_im_platform_to_string() {
        assert_eq!(IMPlatform::WeChatWork.to_string(), "wechat_work");
        assert_eq!(IMPlatform::DingTalk.to_string(), "dingtalk");
        assert_eq!(IMPlatform::FeiShu.to_string(), "feishu");
        assert_eq!(IMPlatform::WeChat.to_string(), "wechat");
        assert_eq!(IMPlatform::Unknown.to_string(), "unknown");
    }
}
