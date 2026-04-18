use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    Request,
    Response,
    Event,
    Command,
    Notification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHeader {
    pub id: Uuid,
    pub message_type: MessageType,
    pub sender_id: String,
    pub receiver_id: Option<String>,
    pub topic: Option<String>,
    pub correlation_id: Option<Uuid>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub priority: u8,
    pub ttl: Option<u64>,
    pub metadata: HashMap<String, String>,
}

impl MessageHeader {
    pub fn new(
        message_type: MessageType,
        sender_id: String,
        receiver_id: Option<String>,
        topic: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type,
            sender_id,
            receiver_id,
            topic,
            correlation_id: None,
            timestamp: chrono::Utc::now(),
            priority: 0,
            ttl: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_correlation_id(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl = Some(ttl_seconds);
        self
    }

    pub fn add_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl {
            let elapsed = chrono::Utc::now().signed_duration_since(self.timestamp);
            elapsed.num_seconds() > ttl as i64
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub header: MessageHeader,
    pub payload: serde_json::Value,
}

impl Message {
    pub fn new(
        message_type: MessageType,
        sender_id: String,
        receiver_id: Option<String>,
        topic: Option<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            header: MessageHeader::new(message_type, sender_id, receiver_id, topic),
            payload,
        }
    }

    pub fn request(sender_id: String, receiver_id: String, payload: serde_json::Value) -> Self {
        Self::new(
            MessageType::Request,
            sender_id,
            Some(receiver_id),
            None,
            payload,
        )
    }

    pub fn response(
        sender_id: String,
        receiver_id: String,
        correlation_id: Uuid,
        payload: serde_json::Value,
    ) -> Self {
        let mut msg = Self::new(
            MessageType::Response,
            sender_id,
            Some(receiver_id),
            None,
            payload,
        );
        msg.header.correlation_id = Some(correlation_id);
        msg
    }

    pub fn event(sender_id: String, topic: String, payload: serde_json::Value) -> Self {
        Self::new(MessageType::Event, sender_id, None, Some(topic), payload)
    }

    pub fn command(sender_id: String, receiver_id: String, payload: serde_json::Value) -> Self {
        Self::new(
            MessageType::Command,
            sender_id,
            Some(receiver_id),
            None,
            payload,
        )
    }

    pub fn notification(
        sender_id: String,
        receiver_id: Option<String>,
        topic: Option<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self::new(
            MessageType::Notification,
            sender_id,
            receiver_id,
            topic,
            payload,
        )
    }

    pub fn id(&self) -> Uuid {
        self.header.id
    }

    pub fn message_type(&self) -> MessageType {
        self.header.message_type
    }

    pub fn sender_id(&self) -> &str {
        &self.header.sender_id
    }

    pub fn receiver_id(&self) -> Option<&str> {
        self.header.receiver_id.as_deref()
    }

    pub fn topic(&self) -> Option<&str> {
        self.header.topic.as_deref()
    }

    pub fn correlation_id(&self) -> Option<Uuid> {
        self.header.correlation_id
    }

    pub fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.header.timestamp
    }

    pub fn priority(&self) -> u8 {
        self.header.priority
    }

    pub fn is_expired(&self) -> bool {
        self.header.is_expired()
    }

    pub fn validate(&self) -> MessageValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if self.header.sender_id.is_empty() {
            errors.push("Sender ID cannot be empty".to_string());
        }

        match self.header.message_type {
            MessageType::Request | MessageType::Response | MessageType::Command => {
                if self.header.receiver_id.is_none()
                    || self.header.receiver_id.as_ref().unwrap().is_empty()
                {
                    errors.push(format!(
                        "{:?} message requires a receiver ID",
                        self.header.message_type
                    ));
                }
            }
            MessageType::Event => {
                if self.header.topic.is_none() || self.header.topic.as_ref().unwrap().is_empty() {
                    errors.push("Event message requires a topic".to_string());
                }
            }
            MessageType::Notification => {}
        }

        if self.header.ttl.is_some() && self.is_expired() {
            warnings.push("Message has expired".to_string());
        }

        MessageValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("Invalid message: {0}")]
    InvalidMessage(String),
    #[error("Message expired")]
    Expired,
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Channel error: {0}")]
    ChannelError(String),
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("Topic not found: {0}")]
    TopicNotFound(String),
}

#[async_trait::async_trait]
pub trait CommunicationBus: Send + Sync {
    async fn send(&self, message: Message) -> Result<(), MessageError>;
    async fn receive(&self, agent_id: &str) -> Result<Option<Message>, MessageError>;
    async fn subscribe(&self, agent_id: &str, topic: &str) -> Result<(), MessageError>;
    async fn unsubscribe(&self, agent_id: &str, topic: &str) -> Result<(), MessageError>;
    async fn register_agent(&self, agent_id: &str) -> Result<(), MessageError>;
    async fn unregister_agent(&self, agent_id: &str) -> Result<(), MessageError>;
}
