use crate::models::*;
use gloo_net::http::Request;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::collections::HashMap;
use uuid::Uuid;

const API_BASE_URL: &str = "/api/v1";
const TOKEN_STORAGE_KEY: &str = "aetheris_auth_token";
const CACHE_DURATION_MS: u64 = 5000; // 5 seconds cache

pub struct ApiClient;

struct CacheEntry<T> {
    data: T,
    timestamp: u64,
}

impl ApiClient {
    fn api_url(path: &str) -> String {
        format!("{}{}", API_BASE_URL, path)
    }

    fn get_token() -> Option<String> {
        let window = web_sys::window().expect("Window not available");
        let local_storage = window.local_storage().expect("Local storage not available").expect("Local storage not available");
        local_storage.get_item(TOKEN_STORAGE_KEY).expect("Error getting token from local storage")
    }

    fn set_token(token: &str) {
        let window = web_sys::window().expect("Window not available");
        let local_storage = window.local_storage().expect("Local storage not available").expect("Local storage not available");
        local_storage.set_item(TOKEN_STORAGE_KEY, token).expect("Error setting token in local storage");
    }

    fn clear_token() {
        let window = web_sys::window().expect("Window not available");
        let local_storage = window.local_storage().expect("Local storage not available").expect("Local storage not available");
        local_storage.remove_item(TOKEN_STORAGE_KEY).expect("Error removing token from local storage");
    }

    // Simple in-memory cache for API responses
    fn get_current_timestamp() -> u64 {
        // 使用系统时间戳作为缓存时间
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time error")
            .as_millis() as u64
    }

    async fn handle_response<T: DeserializeOwned + Debug>(response: gloo_net::http::Response) -> Result<T, String> {
        if !response.ok() {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("HTTP error {}: {}", status, text));
        }

        let api_response: ApiResponse<T> = response.json().await.map_err(|e| format!("JSON parse error: {}", e))?;

        if api_response.success {
            api_response.data.ok_or_else(|| "No data in response".to_string())
        } else {
            Err(api_response.error.unwrap_or_else(|| "Unknown API error".to_string()))
        }
    }

    pub async fn get<T: DeserializeOwned + Debug>(path: &str) -> Result<T, String> {
        let mut request = Request::get(&Self::api_url(path))
            .header("Content-Type", "application/json");

        if let Some(token) = Self::get_token() {
            request = request.header("Authorization", &format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request error: {}", e))?;

        Self::handle_response(response).await
    }

    pub async fn post<T: DeserializeOwned + Debug, B: Serialize>(path: &str, body: &B) -> Result<T, String> {
        let mut request = Request::post(&Self::api_url(path))
            .header("Content-Type", "application/json");

        if let Some(token) = Self::get_token() {
            request = request.header("Authorization", &format!("Bearer {}", token));
        }

        let response = request
            .json(body)
            .map_err(|e| format!("JSON serialize error: {}", e))?
            .send()
            .await
            .map_err(|e| format!("Request error: {}", e))?;

        Self::handle_response(response).await
    }

    pub async fn put<T: DeserializeOwned + Debug, B: Serialize>(path: &str, body: &B) -> Result<T, String> {
        let mut request = Request::put(&Self::api_url(path))
            .header("Content-Type", "application/json");

        if let Some(token) = Self::get_token() {
            request = request.header("Authorization", &format!("Bearer {}", token));
        }

        let response = request
            .json(body)
            .map_err(|e| format!("JSON serialize error: {}", e))?
            .send()
            .await
            .map_err(|e| format!("Request error: {}", e))?;

        Self::handle_response(response).await
    }

    pub async fn delete<T: DeserializeOwned + Debug>(path: &str) -> Result<T, String> {
        let mut request = Request::delete(&Self::api_url(path))
            .header("Content-Type", "application/json");

        if let Some(token) = Self::get_token() {
            request = request.header("Authorization", &format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request error: {}", e))?;

        Self::handle_response(response).await
    }
}

impl ApiClient {
    pub async fn create_task(request: CreateTaskRequest) -> Result<Task, String> {
        Self::post("/tasks", &request).await
    }

    pub async fn get_tasks() -> Result<Vec<Task>, String> {
        Self::get("/tasks").await
    }

    pub async fn get_task(task_id: Uuid) -> Result<Task, String> {
        Self::get(&format!("/tasks/{}", task_id)).await
    }

    pub async fn update_task(task_id: Uuid, request: UpdateTaskRequest) -> Result<Task, String> {
        Self::put(&format!("/tasks/{}", task_id), &request).await
    }

    pub async fn pause_task(task_id: Uuid) -> Result<Task, String> {
        Self::post(&format!("/tasks/{}/pause", task_id), &()).await
    }

    pub async fn resume_task(task_id: Uuid) -> Result<Task, String> {
        Self::post(&format!("/tasks/{}/resume", task_id), &()).await
    }

    pub async fn cancel_task(task_id: Uuid) -> Result<Task, String> {
        Self::post(&format!("/tasks/{}/cancel", task_id), &()).await
    }

    pub async fn get_agents() -> Result<Vec<Agent>, String> {
        Self::get("/agents").await
    }

    pub async fn get_agent(agent_id: Uuid) -> Result<Agent, String> {
        Self::get(&format!("/agents/{}", agent_id)).await
    }

    pub async fn get_system_metrics() -> Result<SystemMetrics, String> {
        Self::get("/metrics/system").await
    }

    pub async fn get_task_metrics() -> Result<TaskMetrics, String> {
        Self::get("/metrics/tasks").await
    }

    pub async fn get_alerts(
        _severity: Option<&str>,
        _resolved: Option<bool>,
    ) -> Result<Vec<Alert>, String> {
        Self::get("/alerts").await
    }

    pub async fn resolve_alert(alert_id: Uuid) -> Result<Alert, String> {
        Self::post(&format!("/alerts/{}/resolve", alert_id), &()).await
    }

    pub async fn get_audit_logs(
        _level: Option<&str>,
        _event_type: Option<&str>,
        _task_id: Option<Uuid>,
        _agent_id: Option<Uuid>,
    ) -> Result<Vec<AuditLog>, String> {
        Self::get("/audit-logs").await
    }

    pub async fn get_health() -> Result<serde_json::Value, String> {
        Self::get("/health").await
    }

    pub async fn get_task_dag(task_id: Uuid) -> Result<TaskDAG, String> {
        Self::get(&format!("/tasks/{}/dag", task_id)).await
    }

    pub async fn create_pipeline(request: CreatePipelineRequest) -> Result<Pipeline, String> {
        Self::post("/pipelines", &request).await
    }

    pub async fn get_pipelines() -> Result<Vec<Pipeline>, String> {
        Self::get("/pipelines").await
    }

    pub async fn get_pipeline(pipeline_id: Uuid) -> Result<Pipeline, String> {
        Self::get(&format!("/pipelines/{}", pipeline_id)).await
    }

    pub async fn update_pipeline(pipeline_id: Uuid, request: UpdatePipelineRequest) -> Result<Pipeline, String> {
        Self::put(&format!("/pipelines/{}", pipeline_id), &request).await
    }

    pub async fn delete_pipeline(pipeline_id: Uuid) -> Result<bool, String> {
        Self::delete(&format!("/pipelines/{}", pipeline_id)).await
    }

    pub async fn start_pipeline(pipeline_id: Uuid) -> Result<Pipeline, String> {
        Self::post(&format!("/pipelines/{}/start", pipeline_id), &()).await
    }

    pub async fn stop_pipeline(pipeline_id: Uuid) -> Result<Pipeline, String> {
        Self::post(&format!("/pipelines/{}/stop", pipeline_id), &()).await
    }

    pub async fn get_pipeline_metrics(pipeline_id: Uuid) -> Result<PipelineMetrics, String> {
        Self::get(&format!("/pipelines/{}/metrics", pipeline_id)).await
    }

    pub async fn get_pipeline_logs(pipeline_id: Uuid) -> Result<PipelineLogsResponse, String> {
        Self::get(&format!("/pipelines/{}/logs", pipeline_id)).await
    }

    pub async fn login(request: LoginRequest) -> Result<LoginResponse, String> {
        let response: Result<LoginResponse, String> = Self::post("/auth/login", &request).await;
        if let Ok(login_response) = &response {
            Self::set_token(&login_response.token);
        }
        response
    }

    pub async fn logout() -> Result<bool, String> {
        let response = Self::post("/auth/logout", &()).await;
        Self::clear_token();
        response
    }

    pub async fn get_skills() -> Result<Vec<Skill>, String> {
        Self::get("/skills").await
    }

    pub async fn get_skill(skill_id: Uuid) -> Result<Skill, String> {
        Self::get(&format!("/skills/{}", skill_id)).await
    }

    pub async fn create_skill(request: CreateSkillRequest) -> Result<Skill, String> {
        Self::post("/skills", &request).await
    }

    pub async fn update_skill(skill_id: Uuid, request: UpdateSkillRequest) -> Result<Skill, String> {
        Self::put(&format!("/skills/{}", skill_id), &request).await
    }

    pub async fn delete_skill(skill_id: Uuid) -> Result<bool, String> {
        Self::delete(&format!("/skills/{}", skill_id)).await
    }

    pub async fn create_agent(request: CreateAgentRequest) -> Result<Agent, String> {
        Self::post("/agents", &request).await
    }

    pub async fn update_agent(agent_id: Uuid, request: UpdateAgentRequest) -> Result<Agent, String> {
        Self::put(&format!("/agents/{}", agent_id), &request).await
    }

    pub async fn delete_agent(agent_id: Uuid) -> Result<bool, String> {
        Self::delete(&format!("/agents/{}", agent_id)).await
    }

    pub async fn start_agent(agent_id: Uuid) -> Result<Agent, String> {
        Self::post(&format!("/agents/{}/start", agent_id), &()).await
    }

    pub async fn stop_agent(agent_id: Uuid) -> Result<Agent, String> {
        Self::post(&format!("/agents/{}/stop", agent_id), &()).await
    }
}
