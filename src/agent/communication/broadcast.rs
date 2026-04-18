use super::protocol::{Message, MessageError};
use dashmap::DashMap;
use tokio::sync::mpsc;

pub struct BroadcastChannel {
    topics: DashMap<String, DashMap<String, mpsc::UnboundedSender<Message>>>,
}

impl BroadcastChannel {
    pub fn new() -> Self {
        Self {
            topics: DashMap::new(),
        }
    }

    pub fn subscribe(&self, agent_id: String, topic: String) -> mpsc::UnboundedReceiver<Message> {
        let (tx, rx) = mpsc::unbounded_channel();

        self.topics
            .entry(topic)
            .or_default()
            .insert(agent_id, tx);

        rx
    }

    pub fn unsubscribe(&self, agent_id: &str, topic: &str) {
        if let Some(subscribers) = self.topics.get(topic) {
            subscribers.remove(agent_id);
        }
    }

    pub fn unsubscribe_from_all(&self, agent_id: &str) {
        for subscribers in self.topics.iter_mut() {
            subscribers.remove(agent_id);
        }
    }

    pub fn broadcast(&self, message: Message) -> Result<usize, MessageError> {
        let validation = message.validate();
        if !validation.valid {
            return Err(MessageError::InvalidMessage(validation.errors.join(", ")));
        }

        if message.is_expired() {
            return Err(MessageError::Expired);
        }

        let topic = message.topic().ok_or_else(|| {
            MessageError::InvalidMessage("Broadcast message requires topic".to_string())
        })?;

        let subscribers = self
            .topics
            .get(topic)
            .ok_or_else(|| MessageError::TopicNotFound(topic.to_string()))?;

        let mut sent_count = 0;
        for subscriber in subscribers.iter() {
            if subscriber.value().send(message.clone()).is_ok() {
                sent_count += 1;
            }
        }

        Ok(sent_count)
    }

    pub fn get_subscribers(&self, topic: &str) -> Vec<String> {
        if let Some(subscribers) = self.topics.get(topic) {
            subscribers
                .iter()
                .map(|entry| entry.key().clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_topics(&self) -> Vec<String> {
        self.topics
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    pub fn has_subscribers(&self, topic: &str) -> bool {
        self.topics
            .get(topic)
            .map(|subscribers| !subscribers.is_empty())
            .unwrap_or(false)
    }
}

impl Default for BroadcastChannel {
    fn default() -> Self {
        Self::new()
    }
}
