use crate::agent::config::webhook::{IMPlatform, WebhookManager, WebhookMessage};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{Duration, sleep};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILinkConfig {
    pub enabled: bool,
    pub ilink_url: String,
    pub app_id: String,
    pub app_secret: String,
    pub poll_interval_seconds: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILinkMessage {
    pub msg_id: String,
    pub from_user: String,
    pub to_user: String,
    pub content: String,
    pub msg_type: String,
    pub create_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILinkPollResponse {
    pub messages: Vec<ILinkMessage>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[async_trait]
pub trait ILinkClient: Send + Sync {
    async fn poll_messages(&self, cursor: Option<String>) -> Result<ILinkPollResponse, ILinkError>;
    async fn send_message(&self, message: ILinkMessage) -> Result<(), ILinkError>;
    async fn acknowledge_message(&self, msg_id: &str) -> Result<(), ILinkError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ILinkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Authentication error: {0}")]
    Authentication(String),
    #[error("Polling failed after {0} retries")]
    PollingFailed(u32),
}

pub struct ILinkAdapter {
    config: ILinkConfig,
    client: Arc<dyn ILinkClient>,
    webhook_manager: Arc<WebhookManager>,
    is_running: Arc<RwLock<bool>>,
    message_sender: mpsc::Sender<ILinkMessage>,
    message_receiver: RwLock<Option<mpsc::Receiver<ILinkMessage>>>,
}

impl ILinkAdapter {
    pub fn new(
        config: ILinkConfig,
        client: Arc<dyn ILinkClient>,
        webhook_manager: Arc<WebhookManager>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(100);

        Self {
            config,
            client,
            webhook_manager,
            is_running: Arc::new(RwLock::new(false)),
            message_sender: sender,
            message_receiver: RwLock::new(Some(receiver)),
        }
    }

    pub async fn start(&self) -> Result<(), ILinkError> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Ok(());
        }

        *is_running = true;
        drop(is_running);

        let is_running_clone = self.is_running.clone();
        let client_clone = self.client.clone();
        let sender_clone = self.message_sender.clone();
        let config_clone = self.config.clone();

        tokio::spawn(async move {
            let mut cursor = None;
            let mut retry_count = 0;

            loop {
                {
                    let running = is_running_clone.read().await;
                    if !*running {
                        break;
                    }
                }

                match client_clone.poll_messages(cursor.clone()).await {
                    Ok(response) => {
                        retry_count = 0;

                        for message in response.messages {
                            if let Err(e) = sender_clone.send(message).await {
                                eprintln!("Failed to send message to channel: {}", e);
                            }
                        }

                        if response.has_more {
                            cursor = response.next_cursor;
                        } else {
                            cursor = None;
                            sleep(Duration::from_secs(config_clone.poll_interval_seconds)).await;
                        }
                    }
                    Err(e) => {
                        retry_count += 1;
                        eprintln!("Polling failed (attempt {}): {}", retry_count, e);

                        if retry_count >= config_clone.max_retries {
                            eprintln!("Max retries exceeded, stopping poller");
                            let mut running = is_running_clone.write().await;
                            *running = false;
                            break;
                        }

                        let backoff = Duration::from_secs(2u64.pow(retry_count.min(5)));
                        sleep(backoff).await;
                    }
                }
            }
        });

        let is_running_clone = self.is_running.clone();
        let webhook_manager_clone = self.webhook_manager.clone();
        let mut receiver = self.message_receiver.write().await.take().unwrap();

        tokio::spawn(async move {
            while let Some(ilink_msg) = receiver.recv().await {
                {
                    let running = is_running_clone.read().await;
                    if !*running {
                        break;
                    }
                }

                let webhook_msg = WebhookMessage {
                    message_id: ilink_msg.msg_id.clone(),
                    from_platform: IMPlatform::WeChat,
                    from_user: ilink_msg.from_user,
                    to_agent: ilink_msg.to_user,
                    content: ilink_msg.content,
                    timestamp: ilink_msg.create_time,
                    metadata: None,
                };

                if let Err(e) = webhook_manager_clone.send_message(webhook_msg).await {
                    eprintln!("Failed to send webhook message: {}", e);
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
}

pub struct MockILinkClient {
    messages: RwLock<Vec<ILinkMessage>>,
}

impl MockILinkClient {
    pub fn new() -> Self {
        Self {
            messages: RwLock::new(Vec::new()),
        }
    }

    pub async fn add_message(&self, message: ILinkMessage) {
        let mut messages = self.messages.write().await;
        messages.push(message);
    }
}

impl Default for MockILinkClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ILinkClient for MockILinkClient {
    async fn poll_messages(
        &self,
        _cursor: Option<String>,
    ) -> Result<ILinkPollResponse, ILinkError> {
        let mut messages = self.messages.write().await;
        let messages_to_return: Vec<ILinkMessage> = messages.drain(..).collect();

        Ok(ILinkPollResponse {
            messages: messages_to_return,
            has_more: false,
            next_cursor: None,
        })
    }

    async fn send_message(&self, _message: ILinkMessage) -> Result<(), ILinkError> {
        Ok(())
    }

    async fn acknowledge_message(&self, _msg_id: &str) -> Result<(), ILinkError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ilink_config() {
        let config = ILinkConfig {
            enabled: true,
            ilink_url: "https://ilink.example.com".to_string(),
            app_id: "test-app-id".to_string(),
            app_secret: "test-secret".to_string(),
            poll_interval_seconds: 5,
            max_retries: 3,
        };

        assert!(config.enabled);
        assert_eq!(config.poll_interval_seconds, 5);
    }

    #[tokio::test]
    async fn test_mock_ilink_client() {
        let client = MockILinkClient::new();

        let message = ILinkMessage {
            msg_id: "msg-1".to_string(),
            from_user: "user1".to_string(),
            to_user: "agent1".to_string(),
            content: "Hello".to_string(),
            msg_type: "text".to_string(),
            create_time: 1234567890,
        };

        client.add_message(message).await;

        let response = client.poll_messages(None).await.unwrap();
        assert_eq!(response.messages.len(), 1);
        assert_eq!(response.messages[0].msg_id, "msg-1");
    }

    #[tokio::test]
    async fn test_ilink_adapter_creation() {
        let config = ILinkConfig {
            enabled: true,
            ilink_url: "https://ilink.example.com".to_string(),
            app_id: "test-app-id".to_string(),
            app_secret: "test-secret".to_string(),
            poll_interval_seconds: 5,
            max_retries: 3,
        };

        let client = Arc::new(MockILinkClient::new());
        let webhook_config = crate::agent::config::webhook::WebhookConfig {
            enabled: true,
            endpoint: "/webhook".to_string(),
            secret: None,
            verify_signature: false,
        };
        let webhook_manager = Arc::new(crate::agent::config::webhook::WebhookManager::new(
            webhook_config,
        ));

        let adapter = ILinkAdapter::new(config, client, webhook_manager);

        assert!(!adapter.is_running().await);
    }
}
