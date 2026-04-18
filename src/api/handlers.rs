use crate::api::AppState;
use crate::api::auth::LoginRequest;
use crate::api::models::*;
use crate::ai::inference::InferenceEngine;
use crate::ai::forecasting::TimeSeriesForecaster;
use crate::core::Task;
use crate::data_governance::LineageTracker;
use crate::agent::communication::protocol::CommunicationBus;
use crate::agent::task_decomposer::TaskDecomposer;
use crate::agent::matcher::AgentMatcher;
use dashmap::DashMap;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use tracing::{error, info};

pub async fn health_check() -> (StatusCode, Json<ApiResponse<HealthResponse>>) {
    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<crate::api::auth::LoginResponse>>,
) {
    info!("Login attempt for user: {}", request.username);

    match state.auth.login(request).await {
        Ok(response) => (StatusCode::OK, Json(ApiResponse::success(response))),
        Err(_) => {
            let error_message = "Invalid credentials".to_string();
            (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn submit_task(
    State(state): State<AppState>,
    Json(request): Json<SubmitTaskRequest>,
) -> (StatusCode, Json<ApiResponse<SubmitTaskResponse>>) {
    info!("Received task submission: {}", request.description);

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let mut task = Task::new(request.description, request.priority);

    if let Some(tags) = request.tags {
        task.tags = tags;
    }

    if let Some(metadata) = request.metadata {
        task.metadata = metadata;
    }

    let task_id = task.id.clone();
    state.memory.store_task(task.clone());

    let ws_manager = state.ws_manager.clone();
    let commander = state.commander.clone();
    let memory = state.memory.clone();
    let task_id_clone = task_id.clone();

    tokio::spawn(async move {
        info!("Starting task execution workflow: {}", task_id_clone);

        let mut task = match memory.get_task(&task_id_clone) {
            Some(t) => t,
            None => {
                error!("Task not found in memory: {}", task_id_clone);
                return;
            }
        };

        task.mark_running();
        memory.store_task(task.clone());
        ws_manager.broadcast_task_update(&task);

        let mut final_task = task.clone();

        match execute_task_with_commander(&commander, &memory, &ws_manager, task).await {
            Ok(completed_task) => {
                final_task = completed_task;
                final_task.mark_completed();
                info!("Task completed successfully: {}", task_id_clone);
            }
            Err(e) => {
                error!("Task execution failed: {} - Error: {}", task_id_clone, e);
                final_task.mark_failed();
            }
        }

        memory.store_task(final_task.clone());
        ws_manager.broadcast_task_update(&final_task);

        info!("Task execution workflow completed: {}", task_id_clone);
    });

    let response = SubmitTaskResponse {
        task_id: task_id.clone(),
        status: "submitted".to_string(),
        message: "Task submitted successfully".to_string(),
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

async fn execute_task_with_commander(
    commander: &crate::core::CommanderCore,
    _memory: &crate::memory::ShortTermMemory,
    _ws_manager: &crate::api::websocket::WebSocketManager,
    mut task: crate::core::Task,
) -> crate::utils::Result<crate::core::Task> {
    info!("Executing task workflow: {}", task.id);

    let raw_input = task.description.clone();

    let (intent, validation) = commander.process_intent(&raw_input).await?;
    info!(
        "Intent parsed: {:?}, Validation: {:?}",
        intent.confidence, validation
    );

    let plan = commander.create_plan_from_intent(intent).await?;
    info!("Execution plan created: {} nodes", plan.nodes.len());

    let context = commander.execute_plan(plan).await?;
    info!("Plan executed, context: {:?}", context.context_id);

    let report = commander.reflect_on_execution(&task).await?;
    info!("Execution report generated: success={}", report.success);

    task.result = Some(format!(
        "Execution completed successfully. Report: {:?}",
        report
    ));

    Ok(task)
}

pub async fn list_tasks(
    State(state): State<AppState>,
) -> (StatusCode, Json<ApiResponse<TaskListResponse>>) {
    let tasks = state.memory.get_all_tasks();
    let response = TaskListResponse {
        total: tasks.len(),
        tasks,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<TaskResponse>>) {
    if let Some(task) = state.memory.get_task(&task_id) {
        let response = TaskResponse { task };
        (StatusCode::OK, Json(ApiResponse::success(response)))
    } else {
        let error_message = "Task not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn pause_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<TaskResponse>>) {
    info!("Pausing task: {}", task_id);

    if let Some(mut task) = state.memory.get_task(&task_id) {
        task.mark_paused();
        state.memory.store_task(task.clone());

        let response = TaskResponse { task };
        (StatusCode::OK, Json(ApiResponse::success(response)))
    } else {
        let error_message = "Task not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn resume_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<TaskResponse>>) {
    info!("Resuming task: {}", task_id);

    if let Some(mut task) = state.memory.get_task(&task_id) {
        task.mark_running();
        state.memory.store_task(task.clone());

        let response = TaskResponse { task };
        (StatusCode::OK, Json(ApiResponse::success(response)))
    } else {
        let error_message = "Task not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn cancel_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<TaskResponse>>) {
    info!("Cancelling task: {}", task_id);

    if let Some(mut task) = state.memory.get_task(&task_id) {
        task.status = crate::core::TaskStatus::Cancelled;
        task.updated_at = chrono::Utc::now();
        state.memory.store_task(task.clone());

        let response = TaskResponse { task };
        (StatusCode::OK, Json(ApiResponse::success(response)))
    } else {
        let error_message = "Task not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn list_agents(
    State(state): State<AppState>,
) -> (StatusCode, Json<ApiResponse<AgentListResponse>>) {
    let agents = state.agents.list_all_agents();
    let response = AgentListResponse {
        total: agents.len(),
        agents,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<AgentResponse>>) {
    if let Some(agent) = state.agents.get_agent(&agent_id) {
        let response = AgentResponse {
            config: agent.config().clone(),
            state: agent.state().clone(),
        };
        (StatusCode::OK, Json(ApiResponse::success(response)))
    } else {
        let error_message = "Agent not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn list_audit_events(
    State(state): State<AppState>,
) -> (StatusCode, Json<ApiResponse<AuditEventListResponse>>) {
    let audit_log = state.security.audit_log().await;
    let events = audit_log.get_all_events().to_vec();
    let response = AuditEventListResponse {
        total: events.len(),
        events,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_task_audit(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<AuditEventListResponse>>) {
    let audit_log = state.security.audit_log().await;
    let events = audit_log.get_events(&task_id);
    let response = AuditEventListResponse {
        total: events.len(),
        events,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_metrics(
    State(state): State<AppState>,
) -> (StatusCode, Json<ApiResponse<MetricsResponse>>) {
    let audit_log = state.security.audit_log().await;
    let all_events = audit_log.get_all_events();
    let completed_tasks = all_events
        .iter()
        .filter(|e| {
            matches!(
                e.event_type,
                crate::security::audit::AuditEventType::TaskCompleted
            )
        })
        .count() as u64;
    let failed_tasks = all_events
        .iter()
        .filter(|e| {
            matches!(
                e.event_type,
                crate::security::audit::AuditEventType::TaskFailed
            )
        })
        .count() as u64;

    let response = MetricsResponse {
        total_tasks: completed_tasks + failed_tasks,
        completed_tasks,
        failed_tasks,
        active_agents: state.agents.list_all_agents().len(),
        uptime_seconds: state.telemetry.uptime_seconds(),
        timestamp: chrono::Utc::now(),
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_system_metrics(
    State(state): State<AppState>,
) -> (StatusCode, Json<ApiResponse<SystemMetricsResponse>>) {
    let active_agents = state.agents.list_all_agents().len();
    let uptime_seconds = state.telemetry.uptime_seconds();
    let system_metrics = state
        .telemetry
        .metrics
        .get_system_metrics(active_agents, uptime_seconds);

    let response = SystemMetricsResponse {
        metrics: system_metrics,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn list_task_metrics(
    State(state): State<AppState>,
) -> (StatusCode, Json<ApiResponse<TaskMetricsListResponse>>) {
    let tasks = state.telemetry.metrics.get_all_task_metrics();
    let response = TaskMetricsListResponse {
        total: tasks.len(),
        tasks,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_task_metrics(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> (
    StatusCode,
    Json<ApiResponse<crate::observability::TaskMetrics>>,
) {
    if let Some(task_metrics) = state.telemetry.metrics.get_task_metrics(&task_id) {
        (StatusCode::OK, Json(ApiResponse::success(task_metrics)))
    } else {
        let error_message = "Task metrics not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn list_alerts(
    State(state): State<AppState>,
) -> (StatusCode, Json<ApiResponse<AlertListResponse>>) {
    let alerts = state.telemetry.metrics.get_alerts(false);
    let response = AlertListResponse {
        total: alerts.len(),
        alerts,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn create_alert(
    State(state): State<AppState>,
    Json(request): Json<CreateAlertRequest>,
) -> (StatusCode, Json<ApiResponse<String>>) {
    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let severity = match request.severity.to_lowercase().as_str() {
        "info" => crate::observability::AlertSeverity::Info,
        "warning" => crate::observability::AlertSeverity::Warning,
        "error" => crate::observability::AlertSeverity::Error,
        "critical" => crate::observability::AlertSeverity::Critical,
        _ => crate::observability::AlertSeverity::Warning,
    };

    let alert_id = state.telemetry.metrics.create_alert(
        request.alert_type,
        severity,
        request.message,
        request.task_id,
        request.agent_id,
    );

    (StatusCode::OK, Json(ApiResponse::success(alert_id)))
}

pub async fn resolve_alert(
    State(state): State<AppState>,
    Path(alert_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    let success = state.telemetry.metrics.resolve_alert(&alert_id);
    (StatusCode::OK, Json(ApiResponse::success(success)))
}

pub async fn prometheus_metrics(State(_state): State<AppState>) -> (StatusCode, String) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "# Prometheus metrics not available".to_string(),
    )
}

lazy_static::lazy_static! {
    static ref PIPELINE_STORE: DashMap<String, Pipeline> = DashMap::new();
}

pub async fn create_pipeline(
    State(_state): State<AppState>,
    Json(request): Json<CreatePipelineRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<PipelineResponse>>,
) {
    info!("Creating pipeline: {}", request.name);

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let pipeline_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    let pipeline = Pipeline {
        id: pipeline_id.clone(),
        name: request.name,
        description: request.description,
        status: PipelineStatus::Stopped,
        created_at: now,
        updated_at: now,
        started_at: None,
    };

    PIPELINE_STORE.insert(pipeline_id, pipeline.clone());

    let response = PipelineResponse { pipeline };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn list_pipelines(
    State(_state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<PipelineListResponse>>,
) {
    let pipelines: Vec<Pipeline> = PIPELINE_STORE
        .iter()
        .map(|entry| entry.value().clone())
        .collect();
    let response = PipelineListResponse {
        total: pipelines.len(),
        pipelines,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_pipeline(
    State(_state): State<AppState>,
    Path(pipeline_id): Path<String>,
) -> (
    StatusCode,
    Json<ApiResponse<PipelineResponse>>,
) {
    if let Some(pipeline) = PIPELINE_STORE.get(&pipeline_id) {
        let response = PipelineResponse {
            pipeline: pipeline.value().clone(),
        };
        (StatusCode::OK, Json(ApiResponse::success(response)))
    } else {
        let error_message = "Pipeline not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn update_pipeline(
    State(_state): State<AppState>,
    Path(pipeline_id): Path<String>,
    Json(request): Json<UpdatePipelineRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<PipelineResponse>>,
) {
    info!("Updating pipeline: {}", pipeline_id);

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    if let Some(mut entry) = PIPELINE_STORE.get_mut(&pipeline_id) {
        let pipeline = entry.value_mut();

        if let Some(name) = request.name {
            pipeline.name = name;
        }
        if let Some(description) = request.description {
            pipeline.description = Some(description);
        }
        pipeline.updated_at = chrono::Utc::now();

        let response = PipelineResponse {
            pipeline: pipeline.clone(),
        };
        (StatusCode::OK, Json(ApiResponse::success(response)))
    } else {
        let error_message = "Pipeline not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn delete_pipeline(
    State(_state): State<AppState>,
    Path(pipeline_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    info!("Deleting pipeline: {}", pipeline_id);

    if PIPELINE_STORE.remove(&pipeline_id).is_some() {
        (StatusCode::OK, Json(ApiResponse::success(true)))
    } else {
        let error_message = "Pipeline not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn start_pipeline(
    State(_state): State<AppState>,
    Path(pipeline_id): Path<String>,
) -> (
    StatusCode,
    Json<ApiResponse<PipelineResponse>>,
) {
    info!("Starting pipeline: {}", pipeline_id);

    if let Some(mut entry) = PIPELINE_STORE.get_mut(&pipeline_id) {
        let pipeline = entry.value_mut();
        pipeline.status = PipelineStatus::Running;
        pipeline.started_at = Some(chrono::Utc::now());
        pipeline.updated_at = chrono::Utc::now();

        let response = PipelineResponse {
            pipeline: pipeline.clone(),
        };
        (StatusCode::OK, Json(ApiResponse::success(response)))
    } else {
        let error_message = "Pipeline not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn stop_pipeline(
    State(_state): State<AppState>,
    Path(pipeline_id): Path<String>,
) -> (
    StatusCode,
    Json<ApiResponse<PipelineResponse>>,
) {
    info!("Stopping pipeline: {}", pipeline_id);

    if let Some(mut entry) = PIPELINE_STORE.get_mut(&pipeline_id) {
        let pipeline = entry.value_mut();
        pipeline.status = PipelineStatus::Stopped;
        pipeline.updated_at = chrono::Utc::now();

        let response = PipelineResponse {
            pipeline: pipeline.clone(),
        };
        (StatusCode::OK, Json(ApiResponse::success(response)))
    } else {
        let error_message = "Pipeline not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn get_pipeline_metrics(
    State(_state): State<AppState>,
    Path(pipeline_id): Path<String>,
) -> (
    StatusCode,
    Json<ApiResponse<PipelineMetrics>>,
) {
    if !PIPELINE_STORE.contains_key(&pipeline_id) {
        let error_message = "Pipeline not found".to_string();
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        );
    }

    let metrics = PipelineMetrics {
        pipeline_id: pipeline_id.clone(),
        records_processed: 0,
        errors: 0,
        throughput_per_second: 0.0,
        latency_ms: 0.0,
        uptime_seconds: 0,
        timestamp: chrono::Utc::now(),
    };

    (StatusCode::OK, Json(ApiResponse::success(metrics)))
}

pub async fn get_pipeline_logs(
    State(_state): State<AppState>,
    Path(pipeline_id): Path<String>,
) -> (
    StatusCode,
    Json<ApiResponse<PipelineLogsResponse>>,
) {
    if !PIPELINE_STORE.contains_key(&pipeline_id) {
        let error_message = "Pipeline not found".to_string();
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        );
    }

    let response = PipelineLogsResponse::default();
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn create_user(
    State(state): State<AppState>,
    Json(request): Json<CreateUserRequest>,
) -> (StatusCode, Json<ApiResponse<UserResponse>>) {
    info!("Creating user: {}", request.username);

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    match state
        .auth
        .add_user(
            request.username.clone(),
            request.password,
            request.role.clone(),
        )
        .await
    {
        Ok(user_id) => {
            let response = UserResponse {
                user_id,
                username: request.username,
                role: request.role,
            };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(crate::api::auth::AuthError::UserAlreadyExists) => {
            let error_message = "User already exists".to_string();
            (
                StatusCode::CONFLICT,
                Json(ApiResponse::error(error_message)),
            )
        }
        Err(_) => {
            let error_message = "Failed to create user".to_string();
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn list_users(
    State(state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<UserListResponse>>,
) {
    let users = state.auth.list_users().await;
    let user_responses: Vec<UserResponse> = users
        .into_iter()
        .map(|user| UserResponse {
            user_id: user.user_id,
            username: user.username,
            role: user.role,
        })
        .collect();

    let response = UserListResponse {
        total: user_responses.len(),
        users: user_responses,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn update_user_role(
    State(state): State<AppState>,
    Path(user_id): Path<uuid::Uuid>,
    Json(request): Json<UpdateUserRoleRequest>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    info!("Updating user role: {}", user_id);

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    match state.auth.update_user_role(user_id, request.role).await {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success(true))),
        Err(crate::api::auth::AuthError::UserNotFound) => {
            let error_message = "User not found".to_string();
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(error_message)),
            )
        }
        Err(_) => {
            let error_message = "Failed to update user role".to_string();
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn create_alert_rule(
    State(state): State<AppState>,
    Json(request): Json<CreateAlertRuleRequest>,
) -> (StatusCode, Json<ApiResponse<String>>) {
    info!("Creating alert rule: {}", request.name);

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let now = chrono::Utc::now();
    let rule = crate::observability::AlertRule {
        id: uuid::Uuid::new_v4().to_string(),
        name: request.name,
        description: request.description,
        rule_type: request.rule_type,
        condition: request.condition,
        severity: request.severity,
        status: crate::observability::AlertRuleStatus::Enabled,
        channel_ids: request.channel_ids,
        escalation_policy_id: request.escalation_policy_id,
        evaluation_interval_seconds: request.evaluation_interval_seconds,
        last_evaluated_at: None,
        created_at: now,
        updated_at: now,
    };

    let rule_id = state.telemetry.alert_rule_engine.create_rule(rule);
    (StatusCode::OK, Json(ApiResponse::success(rule_id)))
}

pub async fn list_alert_rules(
    State(state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<AlertRuleListResponse>>,
) {
    let rules = state.telemetry.alert_rule_engine.list_rules();
    let response = AlertRuleListResponse {
        total: rules.len(),
        rules,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_alert_rule(
    State(state): State<AppState>,
    Path(rule_id): Path<String>,
) -> (
    StatusCode,
    Json<ApiResponse<crate::observability::AlertRule>>,
) {
    if let Some(rule) = state.telemetry.alert_rule_engine.get_rule(&rule_id) {
        (StatusCode::OK, Json(ApiResponse::success(rule)))
    } else {
        let error_message = "Alert rule not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn update_alert_rule(
    State(state): State<AppState>,
    Path(rule_id): Path<String>,
    Json(request): Json<UpdateAlertRuleRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<crate::observability::AlertRule>>,
) {
    info!("Updating alert rule: {}", rule_id);

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    if let Some(existing_rule) = state.telemetry.alert_rule_engine.get_rule(&rule_id) {
        let mut updated_rule = existing_rule;
        if let Some(name) = request.name {
            updated_rule.name = name;
        }
        if let Some(description) = request.description {
            updated_rule.description = Some(description);
        }
        if let Some(condition) = request.condition {
            updated_rule.condition = condition;
        }
        if let Some(severity) = request.severity {
            updated_rule.severity = severity;
        }
        if let Some(channel_ids) = request.channel_ids {
            updated_rule.channel_ids = channel_ids;
        }
        if request.escalation_policy_id.is_some() {
            updated_rule.escalation_policy_id = request.escalation_policy_id;
        }
        if let Some(interval) = request.evaluation_interval_seconds {
            updated_rule.evaluation_interval_seconds = interval;
        }
        if let Some(status) = request.status {
            updated_rule.status = status;
        }

        if let Some(rule) = state
            .telemetry
            .alert_rule_engine
            .update_rule(&rule_id, updated_rule)
        {
            (StatusCode::OK, Json(ApiResponse::success(rule)))
        } else {
            let error_message = "Failed to update alert rule".to_string();
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    } else {
        let error_message = "Alert rule not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn delete_alert_rule(
    State(state): State<AppState>,
    Path(rule_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    info!("Deleting alert rule: {}", rule_id);

    let success = state.telemetry.alert_rule_engine.delete_rule(&rule_id);
    (StatusCode::OK, Json(ApiResponse::success(success)))
}

pub async fn get_alert_history(
    State(state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<AlertHistoryListResponse>>,
) {
    let history = state.telemetry.alert_rule_engine.get_alert_history(None);
    let response = AlertHistoryListResponse {
        total: history.len(),
        history,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn mute_alert_rule(
    State(state): State<AppState>,
    Path(rule_id): Path<String>,
    Json(request): Json<MuteAlertRuleRequest>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    info!("Muting alert rule: {}", rule_id);

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let success = state.telemetry.alert_rule_engine.mute_rule(
        &rule_id,
        request.reason,
        request.muted_by,
        request.duration_seconds,
    );
    (StatusCode::OK, Json(ApiResponse::success(success)))
}

pub async fn unmute_alert_rule(
    State(state): State<AppState>,
    Path(rule_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    info!("Unmuting alert rule: {}", rule_id);

    let success = state.telemetry.alert_rule_engine.unmute_rule(&rule_id);
    (StatusCode::OK, Json(ApiResponse::success(success)))
}

pub async fn acknowledge_alert(
    State(state): State<AppState>,
    Path(history_id): Path<String>,
    Json(request): Json<AcknowledgeAlertRequest>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    info!("Acknowledging alert: {}", history_id);

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let success = state
        .telemetry
        .alert_rule_engine
        .acknowledge_alert(&history_id, request.acknowledged_by);
    (StatusCode::OK, Json(ApiResponse::success(success)))
}

pub async fn resolve_alert_history(
    State(state): State<AppState>,
    Path(history_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    info!("Resolving alert: {}", history_id);

    let success = state.telemetry.alert_rule_engine.resolve_alert(&history_id);
    (StatusCode::OK, Json(ApiResponse::success(success)))
}

pub async fn create_notification_channel(
    State(state): State<AppState>,
    Json(request): Json<CreateNotificationChannelRequest>,
) -> (StatusCode, Json<ApiResponse<String>>) {
    info!("Creating notification channel: {}", request.name);

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let now = chrono::Utc::now();
    let channel = crate::observability::NotificationChannel {
        id: uuid::Uuid::new_v4().to_string(),
        name: request.name,
        channel_type: request.channel_type,
        config: request.config,
        enabled: true,
        created_at: now,
        updated_at: now,
    };

    let channel_id = state.telemetry.alert_rule_engine.create_channel(channel);
    (
        StatusCode::OK,
        Json(ApiResponse::success(channel_id)),
    )
}

pub async fn list_notification_channels(
    State(state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<NotificationChannelListResponse>>,
) {
    let channels = state.telemetry.alert_rule_engine.list_channels();
    let response = NotificationChannelListResponse {
        total: channels.len(),
        channels,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn create_escalation_policy(
    State(state): State<AppState>,
    Json(request): Json<CreateEscalationPolicyRequest>,
) -> (StatusCode, Json<ApiResponse<String>>) {
    info!("Creating escalation policy: {}", request.name);

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let now = chrono::Utc::now();
    let policy = crate::observability::EscalationPolicy {
        id: uuid::Uuid::new_v4().to_string(),
        name: request.name,
        steps: request.steps,
        created_at: now,
        updated_at: now,
    };

    let policy_id = state
        .telemetry
        .alert_rule_engine
        .create_escalation_policy(policy);
    (
        StatusCode::OK,
        Json(ApiResponse::success(policy_id)),
    )
}

pub async fn list_escalation_policies(
    State(state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<EscalationPolicyListResponse>>,
) {
    let policies = state.telemetry.alert_rule_engine.list_escalation_policies();
    let response = EscalationPolicyListResponse {
        total: policies.len(),
        policies,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn register_model(
    State(state): State<AppState>,
    Json(request): Json<RegisterModelRequest>,
) -> (StatusCode, Json<ApiResponse<ModelResponse>>) {
    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let model_id = uuid::Uuid::new_v4().to_string();
    let model = crate::ai::inference::Model {
        id: model_id.clone(),
        name: request.name,
        format: request.format,
        version: request.version,
        path: request.path,
        description: request.description,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    match state.model_registry.register_model(model) {
        Ok(registered_model) => {
            let response = ModelResponse {
                model: registered_model,
            };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Failed to register model: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn list_models(
    State(state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<ModelListResponse>>,
) {
    let models = state.model_registry.list_models();
    let response = ModelListResponse {
        total: models.len(),
        models,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_model(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<ModelResponse>>) {
    if let Some(model) = state.model_registry.get_model(&model_id) {
        let response = ModelResponse { model };
        (StatusCode::OK, Json(ApiResponse::success(response)))
    } else {
        let error_message = "Model not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn update_model(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
    Json(request): Json<UpdateModelRequest>,
) -> (StatusCode, Json<ApiResponse<ModelResponse>>) {
    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    if let Some(existing_model) = state.model_registry.get_model(&model_id) {
        let mut updated_model = existing_model.clone();
        if let Some(name) = request.name {
            updated_model.name = name;
        }
        if let Some(format) = request.format {
            updated_model.format = format;
        }
        if let Some(version) = request.version {
            updated_model.version = version;
        }
        if let Some(path) = request.path {
            updated_model.path = path;
        }
        if request.description.is_some() {
            updated_model.description = request.description;
        }

        match state.model_registry.update_model(&model_id, updated_model) {
            Ok(model) => {
                let response = ModelResponse { model };
                (StatusCode::OK, Json(ApiResponse::success(response)))
            }
            Err(e) => {
                let error_message = format!("Failed to update model: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(error_message)),
                )
            }
        }
    } else {
        let error_message = "Model not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn delete_model(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    let success = state.model_registry.delete_model(&model_id);
    (StatusCode::OK, Json(ApiResponse::success(success)))
}

pub async fn run_inference(
    State(state): State<AppState>,
    Json(request): Json<InferenceRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<InferenceResponse>>,
) {
    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let use_cache = request.use_cache.unwrap_or(true);
    let cache_ttl = request.cache_ttl.unwrap_or(3600);
    let mut from_cache = false;

    let cache_key = format!("{}:{:?}", request.model_id, request.data);
    let mut output: Option<crate::ai::inference::InferenceOutput> = None;

    if use_cache {
        if let Some(cached_output) = state.inference_cache.cache_lookup(&cache_key) {
            output = Some(cached_output);
            from_cache = true;
        }
    }

    if output.is_none() {
        let inference_input = crate::ai::inference::InferenceInput {
            model_id: request.model_id.clone(),
            data: request.data,
            parameters: request.parameters,
        };

        match state
            .local_inference_engine
            .inference(inference_input)
            .await
        {
            Ok(result) => {
                output = Some(result.clone());
                state
                    .inference_cache
                    .cache_store(cache_key, result.clone(), cache_ttl);
                state
                    .inference_metrics
                    .record_inference(true, result.latency_ms, 0);
            }
            Err(e) => {
                let error_output = crate::ai::inference::InferenceOutput {
                    model_id: request.model_id.clone(),
                    data: serde_json::Value::Null,
                    latency_ms: 0,
                    success: false,
                    error_message: Some(format!("Inference failed: {}", e)),
                };
                output = Some(error_output);
                state.inference_metrics.record_inference(false, 0, 0);
            }
        }
    }

    let response = InferenceResponse {
        output: output.unwrap(),
        from_cache,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_inference_metrics(
    State(state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<InferenceMetricsResponse>>,
) {
    let metrics = state.inference_metrics.get_metrics();
    let cache_stats = state.inference_cache.get_cache_stats();

    let response = InferenceMetricsResponse {
        metrics,
        cache_stats: Some(cache_stats),
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

lazy_static::lazy_static! {
    static ref ANOMALY_DETECTORS: DashMap<
        crate::ai::anomaly_detection::AnomalyDetectionMethod,
        Box<dyn crate::ai::anomaly_detection::AnomalyDetector + Send + Sync>,
    > = DashMap::new();
    static ref ANOMALY_STORE: DashMap<String, crate::ai::anomaly_detection::Anomaly> =
        DashMap::new();
    static ref VISUALIZATION_DATA: DashMap<
        crate::ai::anomaly_detection::AnomalyDetectionMethod,
        crate::ai::anomaly_detection::AnomalyVisualizationData,
    > = DashMap::new();
}

fn get_detector(
    method: &crate::ai::anomaly_detection::AnomalyDetectionMethod,
) -> Box<dyn crate::ai::anomaly_detection::AnomalyDetector + Send + Sync> {
    match method {
        crate::ai::anomaly_detection::AnomalyDetectionMethod::Statistical3Sigma => {
            Box::new(crate::ai::anomaly_detection::Statistical3SigmaDetector::new())
        }
        crate::ai::anomaly_detection::AnomalyDetectionMethod::StatisticalIQR => {
            Box::new(crate::ai::anomaly_detection::StatisticalIQRDetector::new())
        }
        crate::ai::anomaly_detection::AnomalyDetectionMethod::IsolationForest => {
            Box::new(crate::ai::anomaly_detection::IsolationForestDetector::new())
        }
        crate::ai::anomaly_detection::AnomalyDetectionMethod::LOF => {
            Box::new(crate::ai::anomaly_detection::LOFDetector::new())
        }
        crate::ai::anomaly_detection::AnomalyDetectionMethod::Autoencoder => Box::new(
            crate::ai::anomaly_detection::AutoencoderDetector::new(10, 5),
        ),
    }
}

pub async fn detect_anomaly(
    State(_state): State<AppState>,
    Json(request): Json<DetectAnomalyRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<DetectAnomalyResponse>>,
) {
    info!("Detecting anomaly with features: {:?}", request.features);

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let method = request
        .method
        .unwrap_or(crate::ai::anomaly_detection::AnomalyDetectionMethod::Statistical3Sigma);

    let mut detector_entry = ANOMALY_DETECTORS
        .entry(method.clone())
        .or_insert_with(|| get_detector(&method));

    let detector = detector_entry.value_mut();

    if !detector.is_fitted() {
        let initial_data = vec![request.features.clone()];
        if let Err(e) = detector.fit(&initial_data).await {
            let error_message = format!("Failed to fit detector: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            );
        }
    }

    match detector.detect(&request.features).await {
        Ok(anomaly) => {
            ANOMALY_STORE.insert(anomaly.id.clone(), anomaly.clone());

            let mut viz_data = VISUALIZATION_DATA.entry(method.clone()).or_insert_with(|| {
                crate::ai::anomaly_detection::AnomalyVisualizationData::new(
                    detector.name().to_string(),
                    format!("{:?}", method),
                )
            });

            viz_data.add_anomaly(&anomaly);

            for (key, &value) in &request.features {
                viz_data.add_time_series_point(anomaly.timestamp, value, key.clone());
            }

            viz_data.compute_feature_stats();

            let response = DetectAnomalyResponse { anomaly };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Detection failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn list_anomalies(
    State(_state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<AnomalyListResponse>>,
) {
    let anomalies: Vec<crate::ai::anomaly_detection::Anomaly> = ANOMALY_STORE
        .iter()
        .map(|entry| entry.value().clone())
        .collect();

    let response = AnomalyListResponse {
        total: anomalies.len(),
        anomalies,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_anomaly(
    State(_state): State<AppState>,
    Path(anomaly_id): Path<String>,
) -> (
    StatusCode,
    Json<ApiResponse<DetectAnomalyResponse>>,
) {
    if let Some(anomaly) = ANOMALY_STORE.get(&anomaly_id) {
        let response = DetectAnomalyResponse {
            anomaly: anomaly.value().clone(),
        };
        (StatusCode::OK, Json(ApiResponse::success(response)))
    } else {
        let error_message = "Anomaly not found".to_string();
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn get_anomaly_visualization(
    State(_state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<AnomalyVisualizationResponse>>,
) {
    let method = crate::ai::anomaly_detection::AnomalyDetectionMethod::Statistical3Sigma;

    let viz_data = VISUALIZATION_DATA.entry(method.clone()).or_insert_with(|| {
        crate::ai::anomaly_detection::AnomalyVisualizationData::new(
            "Statistical 3-Sigma Detector".to_string(),
            "Statistical3Sigma".to_string(),
        )
    });

    let response = AnomalyVisualizationResponse {
        visualization_data: viz_data.value().clone(),
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn fit_anomaly_model(
    State(_state): State<AppState>,
    Json(request): Json<FitModelRequest>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    info!("Fitting anomaly detection model");

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let mut detector = get_detector(&request.method);

    match detector.fit(&request.data).await {
        Ok(_) => {
            ANOMALY_DETECTORS.insert(request.method, detector);
            (StatusCode::OK, Json(ApiResponse::success(true)))
        }
        Err(e) => {
            let error_message = format!("Failed to fit model: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

lazy_static::lazy_static! {
    static ref FORECASTERS: DashMap<
        crate::ai::forecasting::ForecastingMethod,
        Box<dyn crate::ai::forecasting::TimeSeriesForecaster + Send + Sync>,
    > = DashMap::new();
    static ref FORECAST_HISTORY: DashMap<String, crate::ai::forecasting::Forecast> = DashMap::new();
}

fn get_forecaster(
    method: &crate::ai::forecasting::ForecastingMethod,
) -> Box<dyn crate::ai::forecasting::TimeSeriesForecaster + Send + Sync> {
    match method {
        crate::ai::forecasting::ForecastingMethod::ARIMA => {
            Box::new(crate::ai::forecasting::ARIMAForecaster::new(1, 1, 1))
        }
        crate::ai::forecasting::ForecastingMethod::ETS => {
            Box::new(crate::ai::forecasting::ETSForecaster::new(
                "additive".to_string(),
                "additive".to_string(),
                "additive".to_string(),
                24,
            ))
        }
        crate::ai::forecasting::ForecastingMethod::XGBoost => {
            Box::new(crate::ai::forecasting::XGBoostForecaster::new(100, 6, 0.1))
        }
        crate::ai::forecasting::ForecastingMethod::LightGBM => Box::new(
            crate::ai::forecasting::LightGBMForecaster::new(100, 31, 0.1),
        ),
        crate::ai::forecasting::ForecastingMethod::LSTM => {
            Box::new(crate::ai::forecasting::LSTMForecaster::new(64, 2, 0.2, 24))
        }
        crate::ai::forecasting::ForecastingMethod::Transformer => Box::new(
            crate::ai::forecasting::TransformerForecaster::new(64, 8, 2, 256, 24),
        ),
    }
}

pub async fn forecast(
    State(_state): State<AppState>,
    Json(request): Json<ForecastRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<ForecastResponse>>,
) {
    info!("Performing time series forecast");

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let method = request
        .method
        .unwrap_or(crate::ai::forecasting::ForecastingMethod::ARIMA);

    let mut forecaster_entry = FORECASTERS
        .entry(method.clone())
        .or_insert_with(|| get_forecaster(&method));
    let forecaster = forecaster_entry.value_mut();

    if !forecaster.is_fitted() {
        if let Err(e) = forecaster.fit(&request.timestamps, &request.values).await {
            let error_message = format!("Failed to fit forecaster: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            );
        }
    }

    match forecaster.predict(request.horizon).await {
        Ok(mut forecast) => {
            if !request.with_confidence.unwrap_or(true) {
                forecast.confidence_intervals = None;
            }

            FORECAST_HISTORY.insert(forecast.id.clone(), forecast.clone());

            let response = ForecastResponse { forecast };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Forecast failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn multi_step_forecast(
    State(_state): State<AppState>,
    Json(request): Json<MultiStepForecastRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<ForecastResponse>>,
) {
    info!("Performing multi-step time series forecast");

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let method = request
        .method
        .unwrap_or(crate::ai::forecasting::ForecastingMethod::LSTM);
    let mut forecaster = get_forecaster(&method);

    if let Err(e) = forecaster.fit(&request.timestamps, &request.values).await {
        let error_message = format!("Failed to fit forecaster: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(error_message)),
        );
    }

    let forecast = forecaster.predict(request.horizon).await;

    match forecast {
        Ok(forecast) => {
            FORECAST_HISTORY.insert(forecast.id.clone(), forecast.clone());

            let response = ForecastResponse { forecast };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Multi-step forecast failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn auto_select_model(
    State(_state): State<AppState>,
    Json(request): Json<ModelSelectionRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<ModelSelectionResponse>>,
) {
    info!("Auto-selecting best forecasting model");

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let mut auto_forecaster = crate::ai::forecasting::AutoForecaster::new(request.criteria);

    let candidates = request.candidate_methods.unwrap_or_else(|| {
        vec![
            crate::ai::forecasting::ForecastingMethod::ARIMA,
            crate::ai::forecasting::ForecastingMethod::ETS,
            crate::ai::forecasting::ForecastingMethod::XGBoost,
        ]
    });

    for method in candidates {
        auto_forecaster.add_candidate(get_forecaster(&method));
    }

    match auto_forecaster
        .fit(&request.timestamps, &request.values)
        .await
    {
        Ok(_) => match auto_forecaster.predict(request.horizon).await {
            Ok(forecast) => {
                let best_method = auto_forecaster.method();
                let best_model_name = auto_forecaster
                    .get_best_model_name()
                    .unwrap_or("Unknown".to_string());
                let performance_history = auto_forecaster.get_performance_history().to_vec();

                FORECAST_HISTORY.insert(forecast.id.clone(), forecast.clone());

                let response = ModelSelectionResponse {
                    best_method,
                    best_model_name,
                    forecast,
                    performance_history,
                };

                (StatusCode::OK, Json(ApiResponse::success(response)))
            }
            Err(e) => {
                let error_message = format!("Prediction failed: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(error_message)),
                )
            }
        },
        Err(e) => {
            let error_message = format!("Model selection failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn get_forecast_history(
    State(_state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<ForecastHistoryResponse>>,
) {
    info!("Retrieving forecast history");

    let forecasts: Vec<crate::ai::forecasting::Forecast> = FORECAST_HISTORY
        .iter()
        .map(|entry| entry.value().clone())
        .collect();

    let response = ForecastHistoryResponse {
        total: forecasts.len(),
        forecasts,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

lazy_static::lazy_static! {
    static ref KNOWLEDGE_GRAPH: DashMap<String, crate::ai::knowledge_graph::Entity> =
        DashMap::new();
    static ref RELATIONSHIPS: DashMap<String, crate::ai::knowledge_graph::Relationship> =
        DashMap::new();
    static ref RELATIONSHIPS_FROM: DashMap<String, Vec<String>> = DashMap::new();
    static ref RELATIONSHIPS_TO: DashMap<String, Vec<String>> = DashMap::new();
    static ref CASE_LIBRARY: parking_lot::RwLock<crate::ai::knowledge_graph::CaseLibrary> =
        parking_lot::RwLock::new(crate::ai::knowledge_graph::CaseLibrary::new());
}

pub async fn add_entity(
    State(_state): State<AppState>,
    Json(request): Json<AddEntityRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<AddEntityResponse>>,
) {
    info!("Adding entity to knowledge graph: {}", request.name);

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let mut entity = crate::ai::knowledge_graph::Entity::new(
        request.entity_type,
        request.name,
        request.description,
    );

    if let Some(properties) = request.properties {
        for (key, value) in properties {
            entity = entity.with_property(key, value);
        }
    }

    let entity_id = entity.id.clone();
    KNOWLEDGE_GRAPH.insert(entity_id.clone(), entity);

    let response = AddEntityResponse { entity_id };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn add_relationship(
    State(_state): State<AppState>,
    Json(request): Json<AddRelationshipRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<AddRelationshipResponse>>,
) {
    info!("Adding relationship to knowledge graph");

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let mut relationship = crate::ai::knowledge_graph::Relationship::new(
        request.relationship_type,
        request.source_id,
        request.target_id,
    );

    if let Some(properties) = request.properties {
        for (key, value) in properties {
            relationship = relationship.with_property(key, value);
        }
    }

    let rel_id = relationship.id.clone();
    let source_id = relationship.source_id.clone();
    let target_id = relationship.target_id.clone();

    RELATIONSHIPS.insert(rel_id.clone(), relationship);

    RELATIONSHIPS_FROM
        .entry(source_id)
        .or_default()
        .push(rel_id.clone());

    RELATIONSHIPS_TO
        .entry(target_id)
        .or_default()
        .push(rel_id.clone());

    let response = AddRelationshipResponse {
        relationship_id: rel_id,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn search_knowledge_graph(
    State(_state): State<AppState>,
    Json(request): Json<SearchRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<SearchResponse>>,
) {
    info!("Searching knowledge graph");

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let mut entities: Vec<crate::ai::knowledge_graph::Entity> = KNOWLEDGE_GRAPH
        .iter()
        .map(|entry| entry.value().clone())
        .collect();

    if let Some(entity_type) = request.entity_type {
        entities.retain(|e| e.entity_type == entity_type);
    }

    if let Some(keywords) = request.keywords {
        let keywords_lower: Vec<_> = keywords.iter().map(|k| k.to_lowercase()).collect();
        entities.retain(|e| {
            keywords_lower.iter().any(|k| {
                e.name.to_lowercase().contains(k)
                    || e.description
                        .as_ref()
                        .is_some_and(|d| d.to_lowercase().contains(k))
            })
        });
    }

    if let Some(max_results) = request.max_results {
        entities.truncate(max_results);
    }

    let response = SearchResponse {
        total: entities.len(),
        entities,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_maintenance_cases(
    State(_state): State<AppState>,
    Json(request): Json<CaseQueryRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<CaseListResponse>>,
) {
    info!("Retrieving maintenance cases");

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let cases = if let Some(search_text) = request.search_text {
        CASE_LIBRARY.read().search_cases_by_text(&search_text)
    } else if let Some(tags) = request.tags {
        CASE_LIBRARY.read().search_cases_by_tags(&tags)
    } else {
        CASE_LIBRARY.read().list_cases(request.limit)
    };

    let response = CaseListResponse {
        total: cases.len(),
        cases,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn add_maintenance_case(
    State(_state): State<AppState>,
    Json(request): Json<AddCaseRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<AddCaseResponse>>,
) {
    info!("Adding maintenance case");

    if let Err(validation_error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Validation failed: {}",
                validation_error
            ))),
        );
    }

    let mut case =
        crate::ai::knowledge_graph::MaintenanceCase::new(request.title, request.description);

    if let Some(device_ids) = request.device_ids {
        for device_id in device_ids {
            case = case.with_device(device_id);
        }
    }

    if let Some(fault_ids) = request.fault_ids {
        for fault_id in fault_ids {
            case = case.with_fault(fault_id);
        }
    }

    if let Some(solution_ids) = request.solution_ids {
        for solution_id in solution_ids {
            case = case.with_solution(solution_id);
        }
    }

    if let Some(tags) = request.tags {
        for tag in tags {
            case = case.with_tag(tag);
        }
    }

    case.resolution_summary = request.resolution_summary;
    case.root_cause = request.root_cause;
    case.duration_minutes = request.duration_minutes;

    let case_id = case.id.clone();
    match CASE_LIBRARY.write().add_case(case) {
        Ok(_) => {
            let response = AddCaseResponse { case_id };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Failed to add case: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn get_graph_visualization(
    State(_state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<GraphVisualizationResponse>>,
) {
    info!("Generating graph visualization data");

    let mut viz_data = crate::ai::knowledge_graph::GraphVisualizationData::new();

    for entity in KNOWLEDGE_GRAPH.iter() {
        viz_data.add_entity(entity.value().clone());
    }

    for relationship in RELATIONSHIPS.iter() {
        viz_data.add_relationship(relationship.value().clone());
    }

    viz_data.metadata.insert(
        "node_count".to_string(),
        serde_json::json!(viz_data.nodes.len()),
    );
    viz_data.metadata.insert(
        "edge_count".to_string(),
        serde_json::json!(viz_data.edges.len()),
    );

    let response = GraphVisualizationResponse {
        visualization: viz_data.to_cytoscape_format(),
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn submit_feedback(
    State(state): State<AppState>,
    Json(request): Json<SubmitFeedbackRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<SubmitFeedbackResponse>>,
) {
    info!(
        "Submitting feedback for prediction: {}",
        request.prediction_id
    );

    let feedback_type = match request.feedback_type.to_lowercase().as_str() {
        "positive" => crate::ai::adaptive_learning::FeedbackType::Positive,
        "negative" => crate::ai::adaptive_learning::FeedbackType::Negative,
        _ => {
            let error_message = "Invalid feedback type. Use 'positive' or 'negative'".to_string();
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(error_message)),
            );
        }
    };

    let feedback = crate::ai::adaptive_learning::Feedback::new(
        request.model_id,
        request.prediction_id,
        feedback_type,
        request.comment,
        request.metadata,
        request.created_by,
    );

    let feedback_id = state.feedback_manager.submit_feedback(feedback);

    let response = SubmitFeedbackResponse { feedback_id };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn list_feedback(
    State(state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<FeedbackListResponse>>,
) {
    info!("Listing all feedback");

    let feedbacks = state.feedback_manager.list_feedback(None);

    let response = FeedbackListResponse {
        total: feedbacks.len(),
        feedbacks,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn create_model_version(
    State(state): State<AppState>,
    Json(request): Json<CreateModelVersionRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<CreateModelVersionResponse>>,
) {
    info!("Creating model version: {}", request.version);

    let version = crate::ai::adaptive_learning::ModelVersion::new(
        request.model_id,
        request.version,
        request.description,
        request.checksum,
        request.path,
        request.metadata,
    );

    let version_id = state.model_version_manager.create_version(version);

    let response = CreateModelVersionResponse { version_id };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn rollback_model(
    State(state): State<AppState>,
    Path((model_id, version_id)): Path<(String, String)>,
    Json(request): Json<RollbackModelRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<RollbackModelResponse>>,
) {
    info!("Rolling back model {} to version {}", model_id, version_id);

    let rollback_event =
        state
            .auto_rollback_manager
            .manual_rollback(&model_id, &version_id, request.reason);

    let response = RollbackModelResponse { rollback_event };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn start_ab_test(
    State(state): State<AppState>,
    Json(request): Json<StartABTestRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<StartABTestResponse>>,
) {
    info!("Starting A/B test: {}", request.name);

    let test = crate::ai::adaptive_learning::ABTest::new(
        request.name,
        request.description,
        request.model_id,
        request.version_a,
        request.version_b,
        request.traffic_split,
    );

    let test_id = state.ab_test_manager.create_test(test);
    state.ab_test_manager.start_test(&test_id);

    let response = StartABTestResponse { test_id };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_ab_test_result(
    State(state): State<AppState>,
    Path(test_id): Path<String>,
) -> (
    StatusCode,
    Json<ApiResponse<ABTestResultResponse>>,
) {
    info!("Getting A/B test result: {}", test_id);

    let result = state.ab_test_manager.compute_test_result(&test_id);

    match result {
        Some(result) => {
            let response = ABTestResultResponse { result };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        None => {
            let error_message = "Test not found".to_string();
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn send_message(
    State(state): State<AppState>,
    Json(request): Json<SendMessageRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<MessageResponse>>,
) {
    info!("Sending message from: {:?}", request.message_type);

    let msg_type = match request.message_type.to_lowercase().as_str() {
        "request" => crate::agent::communication::MessageType::Request,
        "response" => crate::agent::communication::MessageType::Response,
        "event" => crate::agent::communication::MessageType::Event,
        "command" => crate::agent::communication::MessageType::Command,
        "notification" => crate::agent::communication::MessageType::Notification,
        _ => {
            let error_message = "Invalid message type".to_string();
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(error_message)),
            );
        }
    };

    let mut message = crate::agent::communication::Message::new(
        msg_type,
        request.sender_id,
        request.receiver_id,
        request.topic,
        request.payload,
    );

    if let Some(priority) = request.priority {
        message.header.priority = priority;
    }

    if let Some(ttl) = request.ttl {
        message.header.ttl = Some(ttl);
    }

    if let Some(metadata) = request.metadata {
        message.header.metadata = metadata;
    }

    match state.communication_bus.send_direct(message.clone()) {
        Ok(_) => {
            let response = MessageResponse {
                message_id: message.id().to_string(),
                success: true,
                message: "Message sent successfully".to_string(),
            };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Failed to send message: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn get_messages(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> (
    StatusCode,
    Json<ApiResponse<MessageListResponse>>,
) {
    info!("Getting messages for agent: {}", agent_id);

    let queue = state.communication_bus.get_or_create_queue(&agent_id);
    let messages = queue.get_all();

    let response = MessageListResponse {
        total: messages.len(),
        messages,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn subscribe_topic(
    State(state): State<AppState>,
    Json(request): Json<SubscribeTopicRequest>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    info!(
        "Subscribing agent {} to topic: {}",
        request.agent_id, request.topic
    );

    state
        .communication_bus
        .subscribe_agent(&request.agent_id, &request.topic);

    (StatusCode::OK, Json(ApiResponse::success(true)))
}

pub async fn unsubscribe_topic(
    State(state): State<AppState>,
    Json(request): Json<UnsubscribeTopicRequest>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    info!(
        "Unsubscribing agent {} from topic: {}",
        request.agent_id, request.topic
    );

    state
        .communication_bus
        .unsubscribe_agent(&request.agent_id, &request.topic);

    (StatusCode::OK, Json(ApiResponse::success(true)))
}

pub async fn register_agent(
    State(state): State<AppState>,
    Json(request): Json<RegisterAgentRequest>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    info!("Registering agent: {}", request.agent_id);

    let metadata = request.metadata.unwrap_or_default();

    match state
        .communication_bus
        .register_agent_with_metadata(&request.agent_id, metadata)
    {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success(true))),
        Err(e) => {
            let error_message = format!("Failed to register agent: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn unregister_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    info!("Unregistering agent: {}", agent_id);

    match state.communication_bus.unregister_agent(&agent_id).await {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success(true))),
        Err(e) => {
            let error_message = format!("Failed to unregister agent: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

/*
pub async fn list_registered_agents(
    State(state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<AgentListResponse>>,
) {
    info!("Listing registered agents");

    let agents = state.communication_bus.list_registered_agents();

    let response = AgentListResponse {
        total: agents.len(),
        agents,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}
*/

pub async fn list_topics(
    State(state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<TopicListResponse>>,
) {
    info!("Listing topics");

    let topics = state.communication_bus.get_all_topics();

    let response = TopicListResponse {
        total: topics.len(),
        topics,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_topic_subscribers(
    State(state): State<AppState>,
    Path(topic): Path<String>,
) -> (
    StatusCode,
    Json<ApiResponse<TopicSubscribersResponse>>,
) {
    info!("Getting subscribers for topic: {}", topic);

    let subscribers = state.communication_bus.get_topic_subscribers(&topic);

    let response = TopicSubscribersResponse {
        topic,
        total: subscribers.len(),
        subscribers,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}



pub async fn decompose_task(
    State(_state): State<AppState>,
    Json(request): Json<DecomposeTaskRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<DecomposeTaskResponse>>,
) {
    info!("Decomposing task: {}", request.task_description);

    let decomposer = crate::agent::task_decomposer::LlmTaskDecomposer::new();
    let options = request.options.unwrap_or_default();

    match decomposer
        .decompose_task(&request.task_description, options)
        .await
    {
        Ok(decomposed_task) => {
            let validation = decomposer
                .validate_decomposition(&decomposed_task)
                .await
                .ok();
            let response = DecomposeTaskResponse {
                decomposed_task,
                validation,
            };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Failed to decompose task: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn match_agent(
    State(_state): State<AppState>,
    Json(request): Json<MatchAgentRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<MatchAgentResponse>>,
) {
    info!("Matching agents for requirement");

    let matcher = crate::agent::matcher::SmartAgentMatcher::new();

    match matcher
        .match_agents(request.requirement, &request.agents)
        .await
    {
        Ok(result) => {
            let response = MatchAgentResponse { result };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Failed to match agents: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}



pub async fn record_lineage(
    State(state): State<AppState>,
    Json(request): Json<RecordLineageRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<RecordLineageResponse>>,
) {
    info!("Recording lineage data");

    let mut node_ids = Vec::new();
    let mut edge_ids = Vec::new();

    for node in request.nodes {
        match state.lineage_store.record_node(node).await {
            Ok(id) => node_ids.push(id.0),
            Err(e) => {
                let error_message = format!("Failed to record node: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(error_message)),
                );
            }
        }
    }

    for edge in request.edges {
        match state.lineage_store.record_edge(edge).await {
            Ok(id) => edge_ids.push(id.0),
            Err(e) => {
                let error_message = format!("Failed to record edge: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(error_message)),
                );
            }
        }
    }

    let response = RecordLineageResponse {
        node_ids,
        edge_ids,
        success: true,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_lineage(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> (
    StatusCode,
    Json<ApiResponse<LineageResponse>>,
) {
    info!("Getting lineage for node: {}", id);

    let node_id = crate::data_governance::LineageNodeId::from_string(id);

    match state.lineage_store.get_lineage(&node_id).await {
        Ok(lineage) => {
            let response = LineageResponse { lineage };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Failed to get lineage: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn query_upstream(
    State(state): State<AppState>,
    Json(request): Json<LineageQueryRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<LineageQueryResponse>>,
) {
    info!("Querying upstream for node: {}", request.node_id);

    let node_id = crate::data_governance::LineageNodeId::from_string(request.node_id);

    match state
        .lineage_store
        .query_upstream(&node_id, request.depth)
        .await
    {
        Ok(nodes) => {
            let response = LineageQueryResponse { nodes };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Failed to query upstream: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn query_downstream(
    State(state): State<AppState>,
    Json(request): Json<LineageQueryRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<LineageQueryResponse>>,
) {
    info!("Querying downstream for node: {}", request.node_id);

    let node_id = crate::data_governance::LineageNodeId::from_string(request.node_id);

    match state
        .lineage_store
        .query_downstream(&node_id, request.depth)
        .await
    {
        Ok(nodes) => {
            let response = LineageQueryResponse { nodes };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Failed to query downstream: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn get_lineage_graph(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> (
    StatusCode,
    Json<ApiResponse<LineageGraphResponse>>,
) {
    info!("Getting lineage graph for node: {}", id);

    let node_id = crate::data_governance::LineageNodeId::from_string(id);

    match state.lineage_store.get_lineage(&node_id).await {
        Ok(lineage) => {
            let viz_data = crate::data_governance::LineageVisualizationData::from_lineage(
                &lineage,
                crate::data_governance::VisualizationFormat::CytoscapeJs,
            );
            let graph_data = viz_data.export();
            let response = LineageGraphResponse { graph_data };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Failed to get lineage graph: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

/*
lazy_static::lazy_static! {
    static ref FILTER_CONFIGS: crate::edge::config::FilterConfigManager =
        crate::edge::config::FilterConfigManager::new();
    static ref FILTER_PIPELINES: DashMap<String, crate::edge::EdgeFilterPipeline> = DashMap::new();
    static ref PIPELINE_START_TIME: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
}
*/

/*
pub async fn filter_data(
    State(_state): State<AppState>,
    Json(request): Json<FilterDataRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<FilterDataResponse>>,
) {
    info!("Filtering edge data: {} records", request.data.len());

    let start = std::time::Instant::now();
    let config_key = request.config_key.unwrap_or_else(|| "default".to_string());

    let mut pipeline = if let Some(pipeline_entry) = FILTER_PIPELINES.get(&config_key) {
        pipeline_entry.value().clone()
    } else {
        let config = FILTER_CONFIGS.get_config(&config_key).unwrap_or_default();
        let default_stream_config = crate::edge::config::StreamConfig {
            stream_id: request
                .stream_id
                .clone()
                .unwrap_or_else(|| "default".to_string()),
            ..Default::default()
        };
        let pipeline = crate::edge::EdgeFilterPipeline::from_stream_config(
            config_key.clone(),
            config,
            &default_stream_config,
        );
        FILTER_PIPELINES.insert(config_key.clone(), pipeline.clone());
        pipeline
    };

    match pipeline.process_batch(request.data.clone()).await {
        Ok(filtered_data) => {
            let duration = start.elapsed();
            let processing_time_ms = duration.as_secs_f64() * 1000.0;

            let response = FilterDataResponse {
                filtered_data: filtered_data.clone(),
                original_count: request.data.len(),
                filtered_count: filtered_data.len(),
                processing_time_ms,
                compression_ratio: pipeline.get_compression_ratio(),
            };

            FILTER_PIPELINES.insert(config_key, pipeline);

            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Failed to filter data: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn get_filter_statistics(
    State(_state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<FilterStatisticsResponse>>,
) {
    info!("Getting filter statistics");

    let mut total_records = 0;
    let mut filtered_records = 0;
    let mut total_processing_time = 0.0;
    let mut count = 0;
    let mut compression_ratios = Vec::new();
    let mut last_record_time = None;

    for entry in FILTER_PIPELINES.iter() {
        let stats = entry.value().get_stats();
        total_records += stats.total_records;
        filtered_records += stats.filtered_records;
        total_processing_time += stats.average_processing_time_ms;
        count += 1;

        if let Some(time) = stats.last_record_time {
            if last_record_time.map(|t| time > t).unwrap_or(true) {
                last_record_time = Some(time);
            }
        }

        if let Some(ratio) = entry.value().get_compression_ratio() {
            compression_ratios.push(ratio);
        }
    }

    let average_processing_time_ms = if count > 0 {
        total_processing_time / count as f64
    } else {
        0.0
    };
    let average_compression_ratio = if !compression_ratios.is_empty() {
        Some(compression_ratios.iter().sum::<f64>() / compression_ratios.len() as f64)
    } else {
        None
    };

    let uptime_seconds = (chrono::Utc::now() - *PIPELINE_START_TIME).num_seconds();

    let response = FilterStatisticsResponse {
        total_records,
        filtered_records,
        uptime_seconds,
        average_processing_time_ms,
        average_compression_ratio,
        last_record_time,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn get_filter_config(
    State(_state): State<AppState>,
    Path(config_key): Path<String>,
) -> (
    StatusCode,
    Json<ApiResponse<GetFilterConfigResponse>>,
) {
    info!("Getting filter config: {}", config_key);

    if let Some(config) = FILTER_CONFIGS.get_config(&config_key) {
        let response = GetFilterConfigResponse { config_key, config };
        (StatusCode::OK, Json(ApiResponse::success(response)))
    } else {
        let error_message = format!("Config not found: {}", config_key);
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(error_message)),
        )
    }
}

pub async fn update_filter_config(
    State(_state): State<AppState>,
    Json(request): Json<UpdateFilterConfigRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<UpdateFilterConfigResponse>>,
) {
    info!("Updating filter config");

    let config_key = uuid::Uuid::new_v4().to_string();
    FILTER_CONFIGS.create_config(config_key.clone(), request.config);
    FILTER_PIPELINES.remove(&config_key);

    let response = UpdateFilterConfigResponse {
        success: true,
        config_key,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn list_filter_configs(
    State(_state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<FilterConfigListResponse>>,
) {
    info!("Listing filter configs");

    let configs = FILTER_CONFIGS.list_configs();
    let response = FilterConfigListResponse {
        total: configs.len(),
        configs,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}
*/

pub async fn get_impact_analysis(
    State(state): State<AppState>,
    Json(request): Json<ImpactAnalysisRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<ImpactAnalysisResponse>>,
) {
    info!("Performing impact analysis for node: {}", request.node_id);

    let node_id = crate::data_governance::LineageNodeId::from_string(request.node_id);

    match state.lineage_store.query_downstream(&node_id, None).await {
        Ok(affected_nodes) => {
            let mut node_type_counts = std::collections::HashMap::new();
            for node in &affected_nodes {
                *node_type_counts
                    .entry(format!("{:?}", node.node_type))
                    .or_insert(0) += 1;
            }

            let response = ImpactAnalysisResponse {
                affected_nodes: affected_nodes.len(),
                node_type_counts,
                affected_nodes_list: affected_nodes,
            };

            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Failed to perform impact analysis: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn export_lineage(
    State(state): State<AppState>,
    Json(request): Json<ExportLineageRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<ExportLineageResponse>>,
) {
    info!("Exporting lineage data in format: {}", request.format);

    let content: String;
    let filename: String;

    match request.format.to_lowercase().as_str() {
        "json" => {
            content = match state.lineage_store.export_to_json() {
                Ok(c) => c,
                Err(e) => {
                    let error_message = format!("Failed to export to JSON: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error(error_message)),
                    );
                }
            };
            filename = "lineage_export.json".to_string();
        }
        "graphml" => {
            content = match state.lineage_store.export_to_graphml() {
                Ok(c) => c,
                Err(e) => {
                    let error_message = format!("Failed to export to GraphML: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error(error_message)),
                    );
                }
            };
            filename = "lineage_export.graphml".to_string();
        }
        _ => {
            let error_message = format!("Unsupported export format: {}", request.format);
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(error_message)),
            );
        }
    }

    let response = ExportLineageResponse {
        format: request.format,
        content,
        filename,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn persist_lineage(
    State(state): State<AppState>,
    Json(request): Json<PersistLineageRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<PersistLineageResponse>>,
) {
    info!("Persisting lineage data to: {}", request.path);

    let path = std::path::PathBuf::from(request.path.clone());

    unsafe {
        let store_ptr = std::ptr::addr_of!(*state.lineage_store)
            as *mut crate::data_governance::lineage::LineageStore;
        (*store_ptr).set_storage_path(path);
    }

    match state.lineage_store.save_to_disk() {
        Ok(_) => {
            let response = PersistLineageResponse {
                success: true,
                path: request.path,
            };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Failed to persist lineage: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn load_lineage(
    State(state): State<AppState>,
    Json(request): Json<LoadLineageRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<LoadLineageResponse>>,
) {
    info!("Loading lineage data from: {}", request.path);

    let path = std::path::PathBuf::from(request.path.clone());

    unsafe {
        let store_ptr = std::ptr::addr_of!(*state.lineage_store)
            as *mut crate::data_governance::lineage::LineageStore;
        (*store_ptr).set_storage_path(path);
    }

    match state.lineage_store.load_from_disk() {
        Ok(loaded) => {
            let response = LoadLineageResponse {
                success: true,
                loaded,
            };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Failed to load lineage: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn classify_data(
    State(_state): State<AppState>,
    Json(request): Json<ClassifyDataRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<ClassifyDataResponse>>,
) {
    info!("Classifying data");

    let manager = crate::data_governance::ClassificationManager::new();

    match manager
        .classify(
            &request.data,
            request.metadata.as_ref(),
            request.strategy_id,
        )
        .await
    {
        Ok(result) => {
            let response = ClassifyDataResponse { result };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Failed to classify data: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn mask_data(
    State(_state): State<AppState>,
    Json(request): Json<MaskDataRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<MaskDataResponse>>,
) {
    info!("Masking data");

    let manager = crate::data_governance::MaskingManager::new();

    let rule = request
        .rule_id
        .and_then(|rid| manager.get_rule(&rid).cloned());

    let result = if request.is_static {
        manager.mask_static(&request.data, rule.as_ref()).await
    } else {
        if let Some(r) = rule {
            manager
                .mask_dynamic(&request.data, &r, request.user_id.as_deref())
                .await
        } else {
            manager.mask_static(&request.data, None).await
        }
    };

    match result {
        Ok(result) => {
            let response = MaskDataResponse { result };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_message = format!("Failed to mask data: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

/*
pub async fn generate_compliance_report(
    State(_state): State<AppState>,
    Json(request): Json<GenerateComplianceReportRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<GenerateComplianceReportResponse>>,
) {
    info!("Generating compliance report: {}", request.name);

    let mut generator = crate::data_governance::ComplianceReportGenerator::new();

    let report = generator.generate_report(
        request.name,
        request.description,
        request.standard,
        request.format,
        request.template_id,
        request.generated_by,
        request.period_start,
        request.period_end,
    );

    let response = GenerateComplianceReportResponse { report };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn list_compliance_reports(
    State(_state): State<AppState>,
) -> (
    StatusCode,
    Json<ApiResponse<ListComplianceReportsResponse>>,
) {
    info!("Listing compliance reports");

    let generator = crate::data_governance::ComplianceReportGenerator::new();
    let reports: Vec<crate::data_governance::ComplianceReport> =
        generator.list_reports();

    let response = ListComplianceReportsResponse {
        total: reports.len(),
        reports,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

pub async fn sign_compliance_report(
    State(_state): State<AppState>,
    Json(request): Json<SignComplianceReportRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<SignComplianceReportResponse>>,
) {
    info!("Signing compliance report: {}", request.report_id);

    let mut generator = crate::data_governance::ComplianceReportGenerator::new();

    match generator.sign_report(request.report_id, request.signer, request.signature) {
        Some(report) => {
            let response = SignComplianceReportResponse { report };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        None => {
            let error_message = "Report not found".to_string();
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}

pub async fn export_compliance_report(
    State(_state): State<AppState>,
    Json(request): Json<ExportComplianceReportRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<ExportComplianceReportResponse>>,
) {
    info!("Exporting compliance report: {}", request.report_id);

    let generator = crate::data_governance::ComplianceReportGenerator::new();

    match generator.export_report(&request.report_id) {
        Some((format, content)) => {
            let response = ExportComplianceReportResponse { format, content };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        None => {
            let error_message = "Report not found".to_string();
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(error_message)),
            )
        }
    }
}
*/
