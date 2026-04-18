use super::protocol::{Message, MessageError};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcknowledgementType {
    Ack,
    Nack,
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub timeout: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            timeout: Duration::from_secs(60),
        }
    }
}

#[derive(Clone)]
struct PendingMessage {
    message: Message,
    retry_count: u32,
    last_sent: Instant,
    next_retry: Instant,
    ack_channel: mpsc::Sender<AcknowledgementType>,
}

pub struct ReliableMessageChannel {
    inner: Arc<dyn super::protocol::CommunicationBus>,
    pending: DashMap<uuid::Uuid, PendingMessage>,
    retry_config: RetryConfig,
    ack_channels: DashMap<uuid::Uuid, mpsc::Sender<AcknowledgementType>>,
}

impl ReliableMessageChannel {
    pub fn new(
        inner: Arc<dyn super::protocol::CommunicationBus>,
        retry_config: RetryConfig,
    ) -> Self {
        Self {
            inner,
            pending: DashMap::new(),
            retry_config,
            ack_channels: DashMap::new(),
        }
    }

    pub async fn send_with_retry(&self, message: Message) -> Result<(), MessageError> {
        let (ack_tx, mut ack_rx) = mpsc::channel(1);
        let message_id = message.id();

        self.ack_channels.insert(message_id, ack_tx);

        let pending = PendingMessage {
            message: message.clone(),
            retry_count: 0,
            last_sent: Instant::now(),
            next_retry: Instant::now(),
            ack_channel: self.ack_channels.get(&message_id).unwrap().clone(),
        };

        self.pending.insert(message_id, pending);

        self.inner.send(message).await?;

        let timeout = sleep(self.retry_config.timeout);
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                _ = &mut timeout => {
                    self.pending.remove(&message_id);
                    self.ack_channels.remove(&message_id);
                    return Err(MessageError::ChannelError("Message delivery timeout".to_string()));
                }
                ack = ack_rx.recv() => {
                    match ack {
                        Some(AcknowledgementType::Ack) => {
                            self.pending.remove(&message_id);
                            self.ack_channels.remove(&message_id);
                            return Ok(());
                        }
                        Some(AcknowledgementType::Nack) => {
                            if let Some(mut pending) = self.pending.get_mut(&message_id) {
                                pending.retry_count += 1;
                                if pending.retry_count > self.retry_config.max_retries {
                                    self.pending.remove(&message_id);
                                    self.ack_channels.remove(&message_id);
                                    return Err(MessageError::ChannelError("Max retries exceeded".to_string()));
                                }

                                let delay = self.calculate_delay(pending.retry_count);
                                pending.next_retry = Instant::now() + delay;
                            }
                        }
                        None => {
                            self.pending.remove(&message_id);
                            self.ack_channels.remove(&message_id);
                            return Err(MessageError::ChannelError("Acknowledgement channel closed".to_string()));
                        }
                    }
                }
            }
        }
    }

    pub fn acknowledge(
        &self,
        message_id: uuid::Uuid,
        ack_type: AcknowledgementType,
    ) -> Result<(), MessageError> {
        if let Some(ack_tx) = self.ack_channels.get(&message_id) {
            let _ = ack_tx.try_send(ack_type);
        }
        Ok(())
    }

    pub async fn process_retries(&self) {
        let now = Instant::now();
        let mut to_retry = Vec::new();

        for entry in self.pending.iter() {
            if entry.next_retry <= now {
                to_retry.push(*entry.key());
            }
        }

        for message_id in to_retry {
            if let Some(mut pending) = self.pending.get_mut(&message_id) {
                let msg = pending.message.clone();
                if let Err(e) = self.inner.send(msg).await {
                    tracing::warn!("Failed to retry message {}: {}", message_id, e);
                }
                pending.last_sent = Instant::now();
                pending.retry_count += 1;

                if pending.retry_count > self.retry_config.max_retries {
                    self.pending.remove(&message_id);
                    self.ack_channels.remove(&message_id);
                } else {
                    let delay = self.calculate_delay(pending.retry_count);
                    pending.next_retry = Instant::now() + delay;
                }
            }
        }
    }

    fn calculate_delay(&self, retry_count: u32) -> Duration {
        let delay_ms = self.retry_config.initial_delay.as_millis() as f64
            * self
                .retry_config
                .backoff_multiplier
                .powi(retry_count as i32);
        let delay_ms = delay_ms.min(self.retry_config.max_delay.as_millis() as f64);
        Duration::from_millis(delay_ms as u64)
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn get_pending_messages(&self) -> Vec<Message> {
        self.pending
            .iter()
            .map(|entry| entry.message.clone())
            .collect()
    }
}

impl Clone for ReliableMessageChannel {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            pending: self.pending.clone(),
            retry_config: self.retry_config.clone(),
            ack_channels: self.ack_channels.clone(),
        }
    }
}
