use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InterventionType {
    Pause,
    Resume,
    Cancel,
    Approve,
    Reject,
    Review,
    Escalate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionRequest {
    pub request_id: String,
    pub task_id: String,
    pub intervention_type: InterventionType,
    pub reason: String,
    pub requested_by: Option<String>,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl InterventionRequest {
    pub fn new(
        task_id: String,
        intervention_type: InterventionType,
        reason: String,
        requested_by: Option<String>,
    ) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            task_id,
            intervention_type,
            reason,
            requested_by,
            requested_at: chrono::Utc::now(),
            expires_at: None,
        }
    }

    pub fn with_expiry(mut self, duration: chrono::Duration) -> Self {
        self.expires_at = Some(chrono::Utc::now() + duration);
        self
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionResponse {
    pub request_id: String,
    pub task_id: String,
    pub approved: bool,
    pub decision: Option<String>,
    pub decided_by: Option<String>,
    pub decided_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    state: CircuitBreakerState,
    failure_count: u32,
    success_count: u32,
    failure_threshold: u32,
    recovery_timeout: chrono::Duration,
    last_failure_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout_seconds: u64) -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            success_count: 0,
            failure_threshold,
            recovery_timeout: chrono::Duration::seconds(recovery_timeout_seconds as i64),
            last_failure_time: None,
        }
    }

    pub fn allow_request(&self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    chrono::Utc::now() - last_failure > self.recovery_timeout
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        if self.state == CircuitBreakerState::HalfOpen {
            self.success_count += 1;
            if self.success_count >= 3 {
                self.state = CircuitBreakerState::Closed;
                self.success_count = 0;
            }
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(chrono::Utc::now());

        match self.state {
            CircuitBreakerState::Closed => {
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitBreakerState::Open;
                    warn!("Circuit breaker opened due to too many failures");
                }
            }
            CircuitBreakerState::HalfOpen => {
                self.state = CircuitBreakerState::Open;
                self.success_count = 0;
                warn!("Circuit breaker re-opened after failure in half-open state");
            }
            _ => {}
        }
    }

    pub fn transition_to_half_open(&mut self) {
        if self.state == CircuitBreakerState::Open {
            self.state = CircuitBreakerState::HalfOpen;
            self.success_count = 0;
            info!("Circuit breaker transitioning to half-open state");
        }
    }

    pub fn state(&self) -> CircuitBreakerState {
        self.state.clone()
    }
}

pub struct HumanInterventionManager {
    sender: broadcast::Sender<InterventionRequest>,
    pending_requests: DashMap<String, InterventionRequest>,
    responses: DashMap<String, InterventionResponse>,
    circuit_breakers: DashMap<String, CircuitBreaker>,
    default_circuit_breaker: CircuitBreaker,
}

impl HumanInterventionManager {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self {
            sender,
            pending_requests: DashMap::new(),
            responses: DashMap::new(),
            circuit_breakers: DashMap::new(),
            default_circuit_breaker: CircuitBreaker::new(5, 60),
        }
    }

    pub async fn request_intervention(&self, request: InterventionRequest) -> Result<()> {
        info!(
            "Requesting intervention for task {}: {:?}",
            request.task_id, request.intervention_type
        );

        self.pending_requests
            .insert(request.request_id.clone(), request.clone());
        self.sender
            .send(request)
            .map_err(|e| AetherisError::Security(e.to_string()))?;

        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<InterventionRequest> {
        self.sender.subscribe()
    }

    pub async fn respond_to_intervention(&self, response: InterventionResponse) -> Result<()> {
        info!(
            "Responding to intervention request {} for task {}: approved={}",
            response.request_id, response.task_id, response.approved
        );

        self.pending_requests.remove(&response.request_id);
        self.responses.insert(response.request_id.clone(), response);

        Ok(())
    }

    pub fn get_pending_requests(&self) -> Vec<InterventionRequest> {
        self.pending_requests
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_pending_requests_for_task(&self, task_id: &str) -> Vec<InterventionRequest> {
        self.pending_requests
            .iter()
            .filter(|entry| entry.value().task_id == task_id)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_response(&self, request_id: &str) -> Option<InterventionResponse> {
        self.responses.get(request_id).map(|r| r.clone())
    }

    pub fn get_circuit_breaker_state(&self, key: &str) -> CircuitBreakerState {
        if let Some(breaker) = self.circuit_breakers.get(key) {
            breaker.state()
        } else {
            self.default_circuit_breaker.state()
        }
    }

    pub fn check_circuit_breaker(&self, key: &str) -> bool {
        if let Some(breaker) = self.circuit_breakers.get(key) {
            breaker.allow_request()
        } else {
            self.default_circuit_breaker.allow_request()
        }
    }

    pub fn record_circuit_success(&self, key: &str) {
        if let Some(mut breaker) = self.circuit_breakers.get_mut(key) {
            breaker.record_success();
        }
    }

    pub fn record_circuit_failure(&self, key: &str) {
        if let Some(mut breaker) = self.circuit_breakers.get_mut(key) {
            breaker.record_failure();
        }
    }

    pub async fn pause_task(
        &self,
        task_id: String,
        reason: String,
        requested_by: Option<String>,
    ) -> Result<()> {
        let request =
            InterventionRequest::new(task_id, InterventionType::Pause, reason, requested_by);
        self.request_intervention(request).await
    }

    pub async fn resume_task(
        &self,
        task_id: String,
        reason: String,
        requested_by: Option<String>,
    ) -> Result<()> {
        let request =
            InterventionRequest::new(task_id, InterventionType::Resume, reason, requested_by);
        self.request_intervention(request).await
    }

    pub async fn cancel_task(
        &self,
        task_id: String,
        reason: String,
        requested_by: Option<String>,
    ) -> Result<()> {
        let request =
            InterventionRequest::new(task_id, InterventionType::Cancel, reason, requested_by);
        self.request_intervention(request).await
    }

    pub async fn approve_task(
        &self,
        task_id: String,
        reason: String,
        requested_by: Option<String>,
    ) -> Result<()> {
        let request =
            InterventionRequest::new(task_id, InterventionType::Approve, reason, requested_by);
        self.request_intervention(request).await
    }

    pub async fn reject_task(
        &self,
        task_id: String,
        reason: String,
        requested_by: Option<String>,
    ) -> Result<()> {
        let request =
            InterventionRequest::new(task_id, InterventionType::Reject, reason, requested_by);
        self.request_intervention(request).await
    }
}

impl Default for HumanInterventionManager {
    fn default() -> Self {
        Self::new()
    }
}
