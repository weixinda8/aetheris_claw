use super::broadcast::BroadcastChannel;
use super::point_to_point::PointToPointChannel;
use super::protocol::{CommunicationBus, Message, MessageError, MessageType};
use super::queue::{MessageQueue, QueuePriority};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;

pub struct AgentCommunicationBus {
    point_to_point: PointToPointChannel,
    broadcast: BroadcastChannel,
    queues: DashMap<String, Arc<MessageQueue>>,
    registered_agents: DashMap<String, AgentInfo>,
}

#[derive(Debug, Clone)]
pub struct AgentInfo {
    agent_id: String,
    registered_at: chrono::DateTime<chrono::Utc>,
    last_seen: chrono::DateTime<chrono::Utc>,
    metadata: std::collections::HashMap<String, String>,
}

impl AgentInfo {
    fn new(agent_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            agent_id,
            registered_at: now,
            last_seen: now,
            metadata: std::collections::HashMap::new(),
        }
    }
}

impl AgentCommunicationBus {
    pub fn new() -> Self {
        Self {
            point_to_point: PointToPointChannel::default(),
            broadcast: BroadcastChannel::default(),
            queues: DashMap::new(),
            registered_agents: DashMap::new(),
        }
    }

    pub fn get_or_create_queue(&self, agent_id: &str) -> Arc<MessageQueue> {
        self.queues
            .entry(agent_id.to_string())
            .or_insert_with(|| Arc::new(MessageQueue::new()))
            .clone()
    }

    pub fn send_direct(&self, message: Message) -> Result<(), MessageError> {
        match message.message_type() {
            MessageType::Event => {
                if message.topic().is_some() {
                    self.broadcast.broadcast(message)?;
                } else {
                    return Err(MessageError::InvalidMessage(
                        "Event message requires topic".to_string(),
                    ));
                }
            }
            _ => {
                self.point_to_point.send(message.clone())?;

                if let Some(receiver_id) = message.receiver_id() {
                    let queue = self.get_or_create_queue(receiver_id);
                    queue.enqueue(message, QueuePriority::Normal, None)?;
                }
            }
        }
        Ok(())
    }

    pub fn broadcast_message(&self, message: Message) -> Result<usize, MessageError> {
        self.broadcast.broadcast(message)
    }

    pub fn register_agent_with_metadata(
        &self,
        agent_id: &str,
        metadata: std::collections::HashMap<String, String>,
    ) -> Result<(), MessageError> {
        let mut info = AgentInfo::new(agent_id.to_string());
        info.metadata = metadata;
        self.registered_agents.insert(agent_id.to_string(), info);
        self.point_to_point.register_agent(agent_id.to_string());
        Ok(())
    }

    pub fn update_agent_metadata(
        &self,
        agent_id: &str,
        key: String,
        value: String,
    ) -> Result<(), MessageError> {
        if let Some(mut info) = self.registered_agents.get_mut(agent_id) {
            info.metadata.insert(key, value);
            info.last_seen = chrono::Utc::now();
            Ok(())
        } else {
            Err(MessageError::AgentNotFound(agent_id.to_string()))
        }
    }

    pub fn get_agent_info(&self, agent_id: &str) -> Option<AgentInfo> {
        self.registered_agents
            .get(agent_id)
            .map(|entry| entry.clone())
    }

    pub fn list_registered_agents(&self) -> Vec<String> {
        self.registered_agents
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    pub fn get_agent_queue(&self, agent_id: &str) -> Option<Arc<MessageQueue>> {
        self.queues.get(agent_id).map(|entry| entry.clone())
    }

    pub fn subscribe_agent(&self, agent_id: &str, topic: &str) {
        self.broadcast
            .subscribe(agent_id.to_string(), topic.to_string());
    }

    pub fn unsubscribe_agent(&self, agent_id: &str, topic: &str) {
        self.broadcast.unsubscribe(agent_id, topic);
    }

    pub fn get_subscribed_topics(&self, agent_id: &str) -> Vec<String> {
        let mut topics = Vec::new();
        for topic in self.broadcast.get_topics() {
            if self
                .broadcast
                .get_subscribers(&topic)
                .contains(&agent_id.to_string())
            {
                topics.push(topic);
            }
        }
        topics
    }

    pub fn get_topic_subscribers(&self, topic: &str) -> Vec<String> {
        self.broadcast.get_subscribers(topic)
    }

    pub fn get_all_topics(&self) -> Vec<String> {
        self.broadcast.get_topics()
    }

    pub fn heartbeat(&self, agent_id: &str) -> Result<(), MessageError> {
        if let Some(mut info) = self.registered_agents.get_mut(agent_id) {
            info.last_seen = chrono::Utc::now();
            Ok(())
        } else {
            Err(MessageError::AgentNotFound(agent_id.to_string()))
        }
    }

    pub fn get_inactive_agents(&self, timeout: Duration) -> Vec<String> {
        let now = chrono::Utc::now();
        self.registered_agents
            .iter()
            .filter(|entry| {
                let elapsed = now.signed_duration_since(entry.last_seen);
                elapsed
                    > chrono::Duration::from_std(timeout).unwrap_or(chrono::Duration::seconds(0))
            })
            .map(|entry| entry.key().clone())
            .collect()
    }
}

#[async_trait::async_trait]
impl CommunicationBus for AgentCommunicationBus {
    async fn send(&self, message: Message) -> Result<(), MessageError> {
        self.send_direct(message)
    }

    async fn receive(&self, agent_id: &str) -> Result<Option<Message>, MessageError> {
        let queue = self.get_or_create_queue(agent_id);
        Ok(queue.dequeue())
    }

    async fn subscribe(&self, agent_id: &str, topic: &str) -> Result<(), MessageError> {
        self.subscribe_agent(agent_id, topic);
        Ok(())
    }

    async fn unsubscribe(&self, agent_id: &str, topic: &str) -> Result<(), MessageError> {
        self.unsubscribe_agent(agent_id, topic);
        Ok(())
    }

    async fn register_agent(&self, agent_id: &str) -> Result<(), MessageError> {
        self.register_agent_with_metadata(agent_id, std::collections::HashMap::new())
    }

    async fn unregister_agent(&self, agent_id: &str) -> Result<(), MessageError> {
        self.registered_agents.remove(agent_id);
        self.point_to_point.unregister_agent(agent_id);
        self.broadcast.unsubscribe_from_all(agent_id);
        self.queues.remove(agent_id);
        Ok(())
    }
}

impl Default for AgentCommunicationBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for AgentCommunicationBus {
    fn clone(&self) -> Self {
        Self {
            point_to_point: PointToPointChannel::default(),
            broadcast: BroadcastChannel::default(),
            queues: DashMap::new(),
            registered_agents: DashMap::new(),
        }
    }
}
