use crate::utils::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[derive(Default)]
pub enum AuditEventType {
    TaskSubmitted,
    TaskValidated,
    TaskStarted,
    TaskCompleted,
    TaskFailed,
    TaskCancelled,
    TaskPaused,
    TaskResumed,
    SecurityViolation,
    RuleTriggered,
    HumanIntervention,
    AgentRegistered,
    AgentUnregistered,
    SkillExecuted,
    SandboxCreated,
    SandboxDestroyed,
    #[default]
    ConfigChanged,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub event_type: AuditEventType,
    pub task_id: Option<String>,
    pub agent_id: Option<String>,
    pub user_id: Option<String>,
    pub allowed: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub details: serde_json::Value,
    pub signature: Option<String>,
}

impl AuditEvent {
    pub fn new(
        event_type: AuditEventType,
        task_id: Option<String>,
        agent_id: Option<String>,
        user_id: Option<String>,
        allowed: bool,
        details: serde_json::Value,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type,
            task_id,
            agent_id,
            user_id,
            allowed,
            timestamp: chrono::Utc::now(),
            details,
            signature: None,
        }
    }

    pub fn with_signature(mut self, signature: String) -> Self {
        self.signature = Some(signature);
        self
    }
}

impl Default for AuditEvent {
    fn default() -> Self {
        Self::new(
            AuditEventType::default(),
            None,
            None,
            None,
            true,
            serde_json::json!({}),
        )
    }
}

pub struct AuditLog {
    events: Vec<AuditEvent>,
    task_events: DashMap<String, Vec<String>>,
    agent_events: DashMap<String, Vec<String>>,
    event_index: DashMap<String, AuditEvent>,
    max_events: usize,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            task_events: DashMap::new(),
            agent_events: DashMap::new(),
            event_index: DashMap::new(),
            max_events: 10000,
        }
    }

    pub fn with_max_events(mut self, max_events: usize) -> Self {
        self.max_events = max_events;
        self
    }

    pub async fn log(&mut self, event: AuditEvent) -> Result<()> {
        info!(
            "Logging audit event: {} - Task: {:?} - Agent: {:?}",
            event.event_id, event.task_id, event.agent_id
        );

        let event_id = event.event_id.clone();

        if let Some(task_id) = &event.task_id {
            self.task_events
                .entry(task_id.clone())
                .or_default()
                .push(event_id.clone());
        }

        if let Some(agent_id) = &event.agent_id {
            self.agent_events
                .entry(agent_id.clone())
                .or_default()
                .push(event_id.clone());
        }

        self.event_index.insert(event_id.clone(), event.clone());
        self.events.push(event);

        if self.events.len() > self.max_events {
            let excess = self.events.len() - self.max_events;
            self.events.drain(0..excess);
        }

        Ok(())
    }

    pub fn get_events(&self, task_id: &str) -> Vec<AuditEvent> {
        self.task_events
            .get(task_id)
            .map(|event_ids| {
                event_ids
                    .iter()
                    .filter_map(|id| self.event_index.get(id).map(|e| e.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_agent_events(&self, agent_id: &str) -> Vec<AuditEvent> {
        self.agent_events
            .get(agent_id)
            .map(|event_ids| {
                event_ids
                    .iter()
                    .filter_map(|id| self.event_index.get(id).map(|e| e.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_all_events(&self) -> Vec<AuditEvent> {
        self.events.clone()
    }

    pub fn get_event(&self, event_id: &str) -> Option<AuditEvent> {
        self.event_index.get(event_id).map(|e| e.clone())
    }

    pub fn get_events_by_type(&self, event_type: &AuditEventType) -> Vec<AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.event_type == *event_type)
            .cloned()
            .collect()
    }

    pub fn get_events_in_time_range(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Vec<AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .cloned()
            .collect()
    }

    pub fn get_security_violations(&self) -> Vec<AuditEvent> {
        self.get_events_by_type(&AuditEventType::SecurityViolation)
    }

    pub fn get_task_history(&self, task_id: &str) -> Vec<AuditEvent> {
        let mut events = self.get_events(task_id);
        events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        events
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.task_events.clear();
        self.agent_events.clear();
        self.event_index.clear();
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}
