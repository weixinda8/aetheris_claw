use super::protocol::{Message, MessageError};
use dashmap::DashMap;
use std::collections::BinaryHeap;
use std::sync::Mutex;
use tokio::sync::Notify;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueuePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl From<u8> for QueuePriority {
    fn from(value: u8) -> Self {
        match value {
            0 => QueuePriority::Low,
            1 => QueuePriority::Normal,
            2 => QueuePriority::High,
            _ => QueuePriority::Critical,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageQueueEntry {
    pub message: Message,
    pub priority: QueuePriority,
    pub deliver_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl PartialEq for MessageQueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.message.id() == other.message.id()
    }
}

impl Eq for MessageQueueEntry {}

impl PartialOrd for MessageQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MessageQueueEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let priority_order = self.priority.cmp(&other.priority).reverse();
        if priority_order != std::cmp::Ordering::Equal {
            return priority_order;
        }

        match (self.deliver_at, other.deliver_at) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (None, None) => self.created_at.cmp(&other.created_at),
        }
    }
}

pub struct MessageQueue {
    heap: Mutex<BinaryHeap<MessageQueueEntry>>,
    message_index: DashMap<uuid::Uuid, MessageQueueEntry>,
    notify: Notify,
}

impl MessageQueue {
    pub fn new() -> Self {
        Self {
            heap: Mutex::new(BinaryHeap::new()),
            message_index: DashMap::new(),
            notify: Notify::new(),
        }
    }

    pub fn enqueue(
        &self,
        message: Message,
        priority: QueuePriority,
        deliver_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), MessageError> {
        let validation = message.validate();
        if !validation.valid {
            return Err(MessageError::InvalidMessage(validation.errors.join(", ")));
        }

        let entry = MessageQueueEntry {
            message: message.clone(),
            priority,
            deliver_at,
            created_at: chrono::Utc::now(),
        };

        self.message_index.insert(message.id(), entry.clone());

        let mut heap = self.heap.lock().unwrap();
        heap.push(entry);
        self.notify.notify_one();

        Ok(())
    }

    pub fn dequeue(&self) -> Option<Message> {
        let now = chrono::Utc::now();
        let mut heap = self.heap.lock().unwrap();

        while let Some(entry) = heap.peek() {
            if entry.message.is_expired() {
                let expired = heap.pop().unwrap();
                self.message_index.remove(&expired.message.id());
                continue;
            }

            if let Some(deliver_at) = entry.deliver_at {
                if deliver_at > now {
                    break;
                }
            }

            let entry = heap.pop().unwrap();
            self.message_index.remove(&entry.message.id());
            return Some(entry.message);
        }

        None
    }

    pub async fn dequeue_async(&self) -> Message {
        loop {
            if let Some(msg) = self.dequeue() {
                return msg;
            }
            self.notify.notified().await;
        }
    }

    pub fn peek(&self) -> Option<Message> {
        let heap = self.heap.lock().unwrap();
        heap.peek().map(|entry| entry.message.clone())
    }

    pub fn get(&self, message_id: uuid::Uuid) -> Option<Message> {
        self.message_index
            .get(&message_id)
            .map(|entry| entry.message.clone())
    }

    pub fn remove(&self, message_id: uuid::Uuid) -> Option<Message> {
        if let Some((_, entry)) = self.message_index.remove(&message_id) {
            let mut heap = self.heap.lock().unwrap();
            let new_heap: BinaryHeap<_> = heap
                .drain()
                .filter(|e| e.message.id() != message_id)
                .collect();
            *heap = new_heap;
            Some(entry.message)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.heap.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.lock().unwrap().is_empty()
    }

    pub fn clear(&self) {
        self.heap.lock().unwrap().clear();
        self.message_index.clear();
    }

    pub fn get_all(&self) -> Vec<Message> {
        let heap = self.heap.lock().unwrap();
        heap.iter().map(|entry| entry.message.clone()).collect()
    }

    pub fn get_by_priority(&self, priority: QueuePriority) -> Vec<Message> {
        let heap = self.heap.lock().unwrap();
        heap.iter()
            .filter(|entry| entry.priority == priority)
            .map(|entry| entry.message.clone())
            .collect()
    }
}

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new()
    }
}
