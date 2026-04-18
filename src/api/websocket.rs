use crate::api::AppState;
use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use dashmap::DashMap;
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WebSocketMessage {
    TaskUpdate {
        task_id: String,
        status: String,
        progress: Option<f32>,
    },
    AgentUpdate {
        agent_id: String,
        state: String,
    },
    SystemMetric {
        cpu_usage: f64,
        memory_usage: f64,
    },
    Alert {
        alert_id: Uuid,
        severity: String,
        message: String,
    },
    Log {
        level: String,
        message: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    Subscribe { task_ids: Vec<Uuid> },
    Unsubscribe { task_ids: Vec<Uuid> },
    Ping,
}

pub struct WebSocketManager {
    clients: DashMap<Uuid, broadcast::Sender<WebSocketMessage>>,
    task_subscriptions: DashMap<Uuid, Vec<Uuid>>,
    global_tx: broadcast::Sender<WebSocketMessage>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        let (global_tx, _) = broadcast::channel(100);
        Self {
            clients: DashMap::new(),
            task_subscriptions: DashMap::new(),
            global_tx,
        }
    }

    pub fn register_client(&self, client_id: Uuid) -> broadcast::Receiver<WebSocketMessage> {
        let (tx, rx) = broadcast::channel(50);
        self.clients.insert(client_id, tx);
        rx
    }

    pub fn unregister_client(&self, client_id: Uuid) {
        self.clients.remove(&client_id);
        for mut task_sub in self.task_subscriptions.iter_mut() {
            task_sub.retain(|&id| id != client_id);
        }
    }

    pub fn subscribe_to_tasks(&self, client_id: Uuid, task_ids: Vec<Uuid>) {
        for task_id in task_ids {
            self.task_subscriptions
                .entry(task_id)
                .or_default()
                .push(client_id);
        }
    }

    pub fn unsubscribe_from_tasks(&self, client_id: Uuid, task_ids: Vec<Uuid>) {
        for task_id in task_ids {
            if let Some(mut subscribers) = self.task_subscriptions.get_mut(&task_id) {
                subscribers.retain(|&id| id != client_id);
            }
        }
    }

    pub fn send_task_update(&self, task_id: String, status: String, progress: Option<f32>) {
        let message = WebSocketMessage::TaskUpdate {
            task_id: task_id.clone(),
            status,
            progress,
        };

        if let Ok(task_uuid) = Uuid::parse_str(&task_id) {
            if let Some(subscribers) = self.task_subscriptions.get(&task_uuid) {
                for client_id in subscribers.iter() {
                    if let Some(tx) = self.clients.get(client_id) {
                        let _ = tx.send(message.clone());
                    }
                }
            }
        }

        let _ = self.global_tx.send(message);
    }

    pub fn broadcast_task_update(&self, task: &crate::core::Task) {
        let status = format!("{:?}", task.status);
        self.send_task_update(task.id.clone(), status, None);
    }

    pub fn send_agent_update(&self, agent_id: String, state: String) {
        let message = WebSocketMessage::AgentUpdate { agent_id, state };
        let _ = self.global_tx.send(message);
    }

    pub fn send_system_metric(&self, cpu_usage: f64, memory_usage: f64) {
        let message = WebSocketMessage::SystemMetric {
            cpu_usage,
            memory_usage,
        };
        let _ = self.global_tx.send(message);
    }

    pub fn send_alert(&self, alert_id: Uuid, severity: String, message: String) {
        let msg = WebSocketMessage::Alert {
            alert_id,
            severity,
            message,
        };
        let _ = self.global_tx.send(msg);
    }

    pub fn send_log(
        &self,
        level: String,
        message: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) {
        let msg = WebSocketMessage::Log {
            level,
            message,
            timestamp,
        };
        let _ = self.global_tx.send(msg);
    }
}

impl WebSocketManager {
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    pub fn task_subscription_count(&self) -> usize {
        self.task_subscriptions.len()
    }

    pub fn has_client(&self, client_id: &Uuid) -> bool {
        self.clients.contains_key(client_id)
    }

    pub fn has_task_subscription(&self, task_id: &Uuid) -> bool {
        self.task_subscriptions.contains_key(task_id)
    }

    pub fn is_subscribed_to_task(&self, client_id: &Uuid, task_id: &Uuid) -> bool {
        self.task_subscriptions
            .get(task_id)
            .map(|subs| subs.contains(client_id))
            .unwrap_or(false)
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    let ws_manager = state.ws_manager.clone();
    ws.on_upgrade(move |socket| handle_socket(socket, ws_manager))
}

async fn handle_socket(socket: WebSocket, ws_manager: Arc<WebSocketManager>) {
    let client_id = Uuid::new_v4();
    let mut rx = ws_manager.register_client(client_id);

    let (mut sender, mut receiver) = socket.split();

    let ws_manager_clone1 = ws_manager.clone();
    let ws_manager_clone3 = ws_manager.clone();

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(text) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    let receive_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                        match client_msg {
                            ClientMessage::Subscribe { task_ids } => {
                                ws_manager_clone1.subscribe_to_tasks(client_id, task_ids);
                            }
                            ClientMessage::Unsubscribe { task_ids } => {
                                ws_manager_clone1.unsubscribe_from_tasks(client_id, task_ids);
                            }
                            ClientMessage::Ping => {
                                let _ = ws_manager_clone1.global_tx.send(WebSocketMessage::Log {
                                    level: "debug".to_string(),
                                    message: "Ping received".to_string(),
                                    timestamp: chrono::Utc::now(),
                                });
                            }
                        }
                    }
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = receive_task => {},
    }

    ws_manager_clone3.unregister_client(client_id);
}
