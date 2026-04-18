use super::protocol::{Message, MessageError};
use dashmap::DashMap;
use tokio::sync::mpsc;

pub struct PointToPointChannel {
    channels: DashMap<String, mpsc::UnboundedSender<Message>>,
    buffer_size: usize,
}

impl PointToPointChannel {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            channels: DashMap::new(),
            buffer_size,
        }
    }

    pub fn register_agent(&self, agent_id: String) -> mpsc::UnboundedReceiver<Message> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.channels.insert(agent_id, tx);
        rx
    }

    pub fn unregister_agent(&self, agent_id: &str) {
        self.channels.remove(agent_id);
    }

    pub fn send(&self, message: Message) -> Result<(), MessageError> {
        let validation = message.validate();
        if !validation.valid {
            return Err(MessageError::InvalidMessage(validation.errors.join(", ")));
        }

        if message.is_expired() {
            return Err(MessageError::Expired);
        }

        let receiver_id = message.receiver_id().ok_or_else(|| {
            MessageError::InvalidMessage("Point-to-point message requires receiver ID".to_string())
        })?;

        if let Some(channel) = self.channels.get(receiver_id) {
            channel.send(message).map_err(|e| {
                MessageError::ChannelError(format!("Failed to send message: {}", e))
            })?;
            Ok(())
        } else {
            Err(MessageError::AgentNotFound(receiver_id.to_string()))
        }
    }

    pub fn is_agent_registered(&self, agent_id: &str) -> bool {
        self.channels.contains_key(agent_id)
    }

    pub fn registered_agents(&self) -> Vec<String> {
        self.channels
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl Default for PointToPointChannel {
    fn default() -> Self {
        Self::new(1000)
    }
}
