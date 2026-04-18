pub mod auth;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod skill_marketplace;
pub mod websocket;

use axum::{
    Extension, Router,
    routing::{delete, get, post, put},
};
use std::sync::Arc;
use tower::ServiceBuilder;

use crate::agent::AgentRegistry;
use crate::agent::communication::AgentCommunicationBus;
use crate::ai::adaptive_learning::{
    ABTestManager, AutoRollbackManager, FeedbackManager, ModelPerformanceMonitor,
    ModelVersionManager,
};
use crate::ai::inference::{InferenceCache, InferenceMetrics, LocalInferenceEngine, ModelRegistry};
use crate::core::CommanderCore;
use crate::data_governance::LineageStore;
use crate::memory::ShortTermMemory;
use crate::observability::{OpenTelemetryManager, telemetry::Telemetry};
use crate::protocol::industrial::IndustrialProtocolManager;
use crate::security::SecurityManager;
use crate::security::rate_limit::RateLimiter;
use crate::skill::{AgentSkillsRegistry, SkillRegistry};
use crate::storage::timeseries::TimeSeriesManager;
use crate::streaming::StreamingRuntime;
use auth::AuthManager;
use middleware::{extract_claims, jwt_auth_middleware, rate_limit_middleware};
use websocket::WebSocketManager;

#[derive(Clone)]
pub struct AppState {
    pub commander: Arc<CommanderCore>,
    pub security: Arc<SecurityManager>,
    pub agents: Arc<AgentRegistry>,
    pub memory: Arc<ShortTermMemory>,
    pub telemetry: Arc<Telemetry>,
    pub auth: Arc<AuthManager>,
    pub ws_manager: Arc<WebSocketManager>,
    pub rate_limiter: Arc<RateLimiter>,
    pub opentelemetry: Option<Arc<OpenTelemetryManager>>,
    pub skill_registry: Arc<SkillRegistry>,
    pub agent_skills_registry: Arc<AgentSkillsRegistry>,
    pub skill_marketplace: Option<Arc<skill_marketplace::SkillMarketplaceState>>,
    pub industrial_protocol_manager: Option<Arc<IndustrialProtocolManager>>,
    pub timeseries_manager: Option<Arc<TimeSeriesManager>>,
    pub streaming_manager: Option<Arc<StreamingRuntime>>,
    pub model_registry: Arc<ModelRegistry>,
    pub inference_cache: Arc<InferenceCache>,
    pub inference_metrics: Arc<InferenceMetrics>,
    pub local_inference_engine: Arc<LocalInferenceEngine>,
    pub feedback_manager: Arc<FeedbackManager>,
    pub model_version_manager: Arc<ModelVersionManager>,
    pub ab_test_manager: Arc<ABTestManager>,
    pub performance_monitor: Arc<ModelPerformanceMonitor>,
    pub auto_rollback_manager: Arc<AutoRollbackManager>,
    pub communication_bus: Arc<AgentCommunicationBus>,
    pub lineage_store: Arc<LineageStore>,
}

/// AppState 构建器
/// 
/// 用于灵活构建 AppState 实例，支持链式调用和复用
/// 
/// # 示例
/// 
/// ```
/// let app_state = AppStateBuilder::new()
///     .commander(commander)
///     .security(security)
///     .agents(agents)
///     .memory(memory)
///     .telemetry(telemetry)
///     .auth(auth)
///     .ws_manager(ws_manager)
///     .rate_limiter(rate_limiter)
///     .skill_registry(skill_registry)
///     .agent_skills_registry(agent_skills_registry)
///     .build();
/// ```
pub struct AppStateBuilder {
    commander: Option<CommanderCore>,
    security: Option<SecurityManager>,
    agents: Option<AgentRegistry>,
    memory: Option<ShortTermMemory>,
    telemetry: Option<Telemetry>,
    auth: Option<AuthManager>,
    ws_manager: Option<WebSocketManager>,
    rate_limiter: Option<RateLimiter>,
    opentelemetry: Option<OpenTelemetryManager>,
    skill_registry: Option<SkillRegistry>,
    agent_skills_registry: Option<AgentSkillsRegistry>,
    skill_marketplace: Option<skill_marketplace::SkillMarketplaceState>,
    industrial_protocol_manager: Option<IndustrialProtocolManager>,
    timeseries_manager: Option<TimeSeriesManager>,
    streaming_manager: Option<StreamingRuntime>,
}

impl Default for AppStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppStateBuilder {
    /// 创建一个新的空构建器
    pub fn new() -> Self {
        Self {
            commander: None,
            security: None,
            agents: None,
            memory: None,
            telemetry: None,
            auth: None,
            ws_manager: None,
            rate_limiter: None,
            opentelemetry: None,
            skill_registry: None,
            agent_skills_registry: None,
            skill_marketplace: None,
            industrial_protocol_manager: None,
            timeseries_manager: None,
            streaming_manager: None,
        }
    }

    /// 设置 CommanderCore（消耗性，支持链式调用）
    pub fn commander(mut self, val: CommanderCore) -> Self {
        self.commander = Some(val);
        self
    }

    /// 设置 SecurityManager（消耗性，支持链式调用）
    pub fn security(mut self, val: SecurityManager) -> Self {
        self.security = Some(val);
        self
    }

    /// 设置 AgentRegistry（消耗性，支持链式调用）
    pub fn agents(mut self, val: AgentRegistry) -> Self {
        self.agents = Some(val);
        self
    }

    /// 设置 ShortTermMemory（消耗性，支持链式调用）
    pub fn memory(mut self, val: ShortTermMemory) -> Self {
        self.memory = Some(val);
        self
    }

    /// 设置 Telemetry（消耗性，支持链式调用）
    pub fn telemetry(mut self, val: Telemetry) -> Self {
        self.telemetry = Some(val);
        self
    }

    /// 设置 AuthManager（消耗性，支持链式调用）
    pub fn auth(mut self, val: AuthManager) -> Self {
        self.auth = Some(val);
        self
    }

    /// 设置 WebSocketManager（消耗性，支持链式调用）
    pub fn ws_manager(mut self, val: WebSocketManager) -> Self {
        self.ws_manager = Some(val);
        self
    }

    /// 设置 RateLimiter（消耗性，支持链式调用）
    pub fn rate_limiter(mut self, val: RateLimiter) -> Self {
        self.rate_limiter = Some(val);
        self
    }

    /// 设置 OpenTelemetryManager（消耗性，支持链式调用）
    pub fn opentelemetry(mut self, val: Option<OpenTelemetryManager>) -> Self {
        self.opentelemetry = val;
        self
    }

    /// 设置 SkillRegistry（消耗性，支持链式调用）
    pub fn skill_registry(mut self, val: SkillRegistry) -> Self {
        self.skill_registry = Some(val);
        self
    }

    /// 设置 AgentSkillsRegistry（消耗性，支持链式调用）
    pub fn agent_skills_registry(mut self, val: AgentSkillsRegistry) -> Self {
        self.agent_skills_registry = Some(val);
        self
    }

    /// 设置 SkillMarketplaceState（消耗性，支持链式调用）
    pub fn skill_marketplace(mut self, val: Option<skill_marketplace::SkillMarketplaceState>) -> Self {
        self.skill_marketplace = val;
        self
    }

    /// 设置 IndustrialProtocolManager（消耗性，支持链式调用）
    pub fn industrial_protocol_manager(mut self, val: Option<IndustrialProtocolManager>) -> Self {
        self.industrial_protocol_manager = val;
        self
    }

    /// 设置 TimeSeriesManager（消耗性，支持链式调用）
    pub fn timeseries_manager(mut self, val: Option<TimeSeriesManager>) -> Self {
        self.timeseries_manager = val;
        self
    }

    /// 设置 StreamingRuntime（消耗性，支持链式调用）
    pub fn streaming_manager(mut self, val: Option<StreamingRuntime>) -> Self {
        self.streaming_manager = val;
        self
    }

    /// 设置 CommanderCore（非消耗性，可链式调用，支持复用 Builder）
    pub fn set_commander(&mut self, val: CommanderCore) -> &mut Self {
        self.commander = Some(val);
        self
    }

    /// 设置 SecurityManager（非消耗性，可链式调用，支持复用 Builder）
    pub fn set_security(&mut self, val: SecurityManager) -> &mut Self {
        self.security = Some(val);
        self
    }

    /// 设置 AgentRegistry（非消耗性，可链式调用，支持复用 Builder）
    pub fn set_agents(&mut self, val: AgentRegistry) -> &mut Self {
        self.agents = Some(val);
        self
    }

    /// 设置 ShortTermMemory（非消耗性，可链式调用，支持复用 Builder）
    pub fn set_memory(&mut self, val: ShortTermMemory) -> &mut Self {
        self.memory = Some(val);
        self
    }

    /// 设置 Telemetry（非消耗性，可链式调用，支持复用 Builder）
    pub fn set_telemetry(&mut self, val: Telemetry) -> &mut Self {
        self.telemetry = Some(val);
        self
    }

    /// 设置 AuthManager（非消耗性，可链式调用，支持复用 Builder）
    pub fn set_auth(&mut self, val: AuthManager) -> &mut Self {
        self.auth = Some(val);
        self
    }

    /// 设置 WebSocketManager（非消耗性，可链式调用，支持复用 Builder）
    pub fn set_ws_manager(&mut self, val: WebSocketManager) -> &mut Self {
        self.ws_manager = Some(val);
        self
    }

    /// 设置 RateLimiter（非消耗性，可链式调用，支持复用 Builder）
    pub fn set_rate_limiter(&mut self, val: RateLimiter) -> &mut Self {
        self.rate_limiter = Some(val);
        self
    }

    /// 设置 OpenTelemetryManager（非消耗性，可链式调用，支持复用 Builder）
    pub fn set_opentelemetry(&mut self, val: Option<OpenTelemetryManager>) -> &mut Self {
        self.opentelemetry = val;
        self
    }

    /// 设置 SkillRegistry（非消耗性，可链式调用，支持复用 Builder）
    pub fn set_skill_registry(&mut self, val: SkillRegistry) -> &mut Self {
        self.skill_registry = Some(val);
        self
    }

    /// 设置 AgentSkillsRegistry（非消耗性，可链式调用，支持复用 Builder）
    pub fn set_agent_skills_registry(&mut self, val: AgentSkillsRegistry) -> &mut Self {
        self.agent_skills_registry = Some(val);
        self
    }

    /// 设置 SkillMarketplaceState（非消耗性，可链式调用，支持复用 Builder）
    pub fn set_skill_marketplace(&mut self, val: Option<skill_marketplace::SkillMarketplaceState>) -> &mut Self {
        self.skill_marketplace = val;
        self
    }

    /// 设置 IndustrialProtocolManager（非消耗性，可链式调用，支持复用 Builder）
    pub fn set_industrial_protocol_manager(&mut self, val: Option<IndustrialProtocolManager>) -> &mut Self {
        self.industrial_protocol_manager = val;
        self
    }

    /// 设置 TimeSeriesManager（非消耗性，可链式调用，支持复用 Builder）
    pub fn set_timeseries_manager(&mut self, val: Option<TimeSeriesManager>) -> &mut Self {
        self.timeseries_manager = val;
        self
    }

    /// 设置 StreamingRuntime（非消耗性，可链式调用，支持复用 Builder）
    pub fn set_streaming_manager(&mut self, val: Option<StreamingRuntime>) -> &mut Self {
        self.streaming_manager = val;
        self
    }

    pub fn build(self) -> AppState {
        let model_version_manager = ModelVersionManager::new();
        AppState {
            commander: Arc::new(self.commander.expect("commander is required")),
            security: Arc::new(self.security.expect("security is required")),
            agents: Arc::new(self.agents.expect("agents is required")),
            memory: Arc::new(self.memory.expect("memory is required")),
            telemetry: Arc::new(self.telemetry.expect("telemetry is required")),
            auth: Arc::new(self.auth.expect("auth is required")),
            ws_manager: Arc::new(self.ws_manager.expect("ws_manager is required")),
            rate_limiter: Arc::new(self.rate_limiter.expect("rate_limiter is required")),
            opentelemetry: self.opentelemetry.map(Arc::new),
            skill_registry: Arc::new(self.skill_registry.expect("skill_registry is required")),
            agent_skills_registry: Arc::new(self.agent_skills_registry.expect("agent_skills_registry is required")),
            skill_marketplace: self.skill_marketplace.map(Arc::new),
            industrial_protocol_manager: self.industrial_protocol_manager.map(Arc::new),
            timeseries_manager: self.timeseries_manager.map(Arc::new),
            streaming_manager: self.streaming_manager.map(Arc::new),
            model_registry: Arc::new(ModelRegistry::new()),
            inference_cache: Arc::new(InferenceCache::new()),
            inference_metrics: Arc::new(InferenceMetrics::new()),
            local_inference_engine: Arc::new(LocalInferenceEngine::new()),
            feedback_manager: Arc::new(FeedbackManager::new()),
            model_version_manager: Arc::new(model_version_manager),
            ab_test_manager: Arc::new(ABTestManager::new()),
            performance_monitor: Arc::new(ModelPerformanceMonitor::default()),
            auto_rollback_manager: Arc::new(AutoRollbackManager::new(ModelVersionManager::new())),
            communication_bus: Arc::new(AgentCommunicationBus::new()),
            lineage_store: Arc::new(LineageStore::new()),
        }
    }
}

impl AppState {
}

pub fn create_router(state: AppState) -> Router {
    let auth_manager = state.auth.clone();
    let rate_limiter = state.rate_limiter.clone();

    let public_routes = Router::new()
        .route("/api/v1/health", get(handlers::health_check))
        .route("/api/v1/auth/login", post(handlers::login))
        .route("/api/v1/ws", get(websocket::ws_handler))
        .route("/metrics", get(handlers::prometheus_metrics))
        .layer(ServiceBuilder::new().layer(Extension(rate_limiter.clone())))
        .route_layer(axum::middleware::from_fn(extract_claims))
        .route_layer(axum::middleware::from_fn(rate_limit_middleware));

    let protected_routes = Router::new()
        .route("/api/v1/tasks", post(handlers::submit_task))
        .route("/api/v1/tasks", get(handlers::list_tasks))
        .route("/api/v1/tasks/:id", get(handlers::get_task))
        .route("/api/v1/tasks/:id/pause", put(handlers::pause_task))
        .route("/api/v1/tasks/:id/resume", put(handlers::resume_task))
        .route("/api/v1/tasks/:id/cancel", delete(handlers::cancel_task))
        .route("/api/v1/agents", get(handlers::list_agents))
        .route("/api/v1/agents/:id", get(handlers::get_agent))
        .route("/api/v1/audit/events", get(handlers::list_audit_events))
        .route(
            "/api/v1/audit/events/:task_id",
            get(handlers::get_task_audit),
        )
        .route("/api/v1/telemetry/metrics", get(handlers::get_metrics))
        .route(
            "/api/v1/observability/system-metrics",
            get(handlers::get_system_metrics),
        )
        .route(
            "/api/v1/observability/task-metrics",
            get(handlers::list_task_metrics),
        )
        .route(
            "/api/v1/observability/task-metrics/:id",
            get(handlers::get_task_metrics),
        )
        .route("/api/v1/observability/alerts", get(handlers::list_alerts))
        .route("/api/v1/observability/alerts", post(handlers::create_alert))
        .route(
            "/api/v1/observability/alerts/:id/resolve",
            put(handlers::resolve_alert),
        )
        .route("/api/v1/pipelines", post(handlers::create_pipeline))
        .route("/api/v1/pipelines", get(handlers::list_pipelines))
        .route("/api/v1/pipelines/:id", get(handlers::get_pipeline))
        .route("/api/v1/pipelines/:id", put(handlers::update_pipeline))
        .route("/api/v1/pipelines/:id", delete(handlers::delete_pipeline))
        .route(
            "/api/v1/pipelines/:id/start",
            post(handlers::start_pipeline),
        )
        .route("/api/v1/pipelines/:id/stop", post(handlers::stop_pipeline))
        .route(
            "/api/v1/pipelines/:id/metrics",
            get(handlers::get_pipeline_metrics),
        )
        .route(
            "/api/v1/pipelines/:id/logs",
            get(handlers::get_pipeline_logs),
        )
        .route("/api/v1/users", get(handlers::list_users))
        .route("/api/v1/alert-rules", post(handlers::create_alert_rule))
        .route("/api/v1/alert-rules", get(handlers::list_alert_rules))
        .route("/api/v1/alert-rules/:id", get(handlers::get_alert_rule))
        .route("/api/v1/alert-rules/:id", put(handlers::update_alert_rule))
        .route(
            "/api/v1/alert-rules/:id",
            delete(handlers::delete_alert_rule),
        )
        .route(
            "/api/v1/alert-rules/:id/mute",
            post(handlers::mute_alert_rule),
        )
        .route(
            "/api/v1/alert-rules/:id/unmute",
            post(handlers::unmute_alert_rule),
        )
        .route("/api/v1/alert-history", get(handlers::get_alert_history))
        .route(
            "/api/v1/alert-history/:id/acknowledge",
            post(handlers::acknowledge_alert),
        )
        .route(
            "/api/v1/alert-history/:id/resolve",
            post(handlers::resolve_alert_history),
        )
        .route(
            "/api/v1/notification-channels",
            post(handlers::create_notification_channel),
        )
        .route(
            "/api/v1/notification-channels",
            get(handlers::list_notification_channels),
        )
        .route(
            "/api/v1/escalation-policies",
            post(handlers::create_escalation_policy),
        )
        .route(
            "/api/v1/escalation-policies",
            get(handlers::list_escalation_policies),
        )
        .route("/api/v1/models", post(handlers::register_model))
        .route("/api/v1/models", get(handlers::list_models))
        .route("/api/v1/models/:id", get(handlers::get_model))
        .route("/api/v1/models/:id", put(handlers::update_model))
        .route("/api/v1/models/:id", delete(handlers::delete_model))
        .route("/api/v1/inference", post(handlers::run_inference))
        .route(
            "/api/v1/inference/metrics",
            get(handlers::get_inference_metrics),
        )
        .route(
            "/api/v1/anomaly-detection/detect",
            post(handlers::detect_anomaly),
        )
        .route(
            "/api/v1/anomaly-detection/anomalies",
            get(handlers::list_anomalies),
        )
        .route(
            "/api/v1/anomaly-detection/anomalies/:id",
            get(handlers::get_anomaly),
        )
        .route(
            "/api/v1/anomaly-detection/visualization",
            get(handlers::get_anomaly_visualization),
        )
        .route(
            "/api/v1/anomaly-detection/fit",
            post(handlers::fit_anomaly_model),
        )
        .route("/api/v1/forecasting/forecast", post(handlers::forecast))
        .route(
            "/api/v1/forecasting/multi-step",
            post(handlers::multi_step_forecast),
        )
        .route(
            "/api/v1/forecasting/auto-select",
            post(handlers::auto_select_model),
        )
        .route(
            "/api/v1/forecasting/history",
            get(handlers::get_forecast_history),
        )
        .route(
            "/api/v1/knowledge-graph/entities",
            post(handlers::add_entity),
        )
        .route(
            "/api/v1/knowledge-graph/relationships",
            post(handlers::add_relationship),
        )
        .route(
            "/api/v1/knowledge-graph/search",
            post(handlers::search_knowledge_graph),
        )
        .route(
            "/api/v1/knowledge-graph/cases",
            post(handlers::get_maintenance_cases),
        )
        .route(
            "/api/v1/knowledge-graph/cases/add",
            post(handlers::add_maintenance_case),
        )
        .route(
            "/api/v1/knowledge-graph/visualization",
            get(handlers::get_graph_visualization),
        )
        .route(
            "/api/v1/adaptive-learning/feedback",
            post(handlers::submit_feedback),
        )
        .route(
            "/api/v1/adaptive-learning/feedback",
            get(handlers::list_feedback),
        )
        .route(
            "/api/v1/adaptive-learning/versions",
            post(handlers::create_model_version),
        )
        .route(
            "/api/v1/adaptive-learning/versions/:model_id/:version_id/rollback",
            post(handlers::rollback_model),
        )
        .route(
            "/api/v1/adaptive-learning/ab-tests",
            post(handlers::start_ab_test),
        )
        .route(
            "/api/v1/adaptive-learning/ab-tests/:id/result",
            get(handlers::get_ab_test_result),
        )
        .route(
            "/api/v1/agent-communication/send",
            post(handlers::send_message),
        )
        .route(
            "/api/v1/agent-communication/messages/:agent_id",
            get(handlers::get_messages),
        )
        .route(
            "/api/v1/agent-communication/subscribe",
            post(handlers::subscribe_topic),
        )
        .route(
            "/api/v1/agent-communication/unsubscribe",
            post(handlers::unsubscribe_topic),
        )
        .route(
            "/api/v1/agent-communication/agents/register",
            post(handlers::register_agent),
        )
        .route(
            "/api/v1/agent-communication/agents/:agent_id/unregister",
            delete(handlers::unregister_agent),
        )
        // .route(
        //     "/api/v1/agent-communication/agents",
        //     get(handlers::list_registered_agents),
        // )
        .route(
            "/api/v1/agent-communication/topics",
            get(handlers::list_topics),
        )
        .route(
            "/api/v1/agent-communication/topics/:topic/subscribers",
            get(handlers::get_topic_subscribers),
        )
        .route(
            "/api/v1/task-decomposer/decompose",
            post(handlers::decompose_task),
        )
        .route("/api/v1/agent-matcher/match", post(handlers::match_agent))
        .route(
            "/api/v1/data-governance/lineage/record",
            post(handlers::record_lineage),
        )
        .route(
            "/api/v1/data-governance/lineage/:id",
            get(handlers::get_lineage),
        )
        .route(
            "/api/v1/data-governance/lineage/query-upstream",
            post(handlers::query_upstream),
        )
        .route(
            "/api/v1/data-governance/lineage/query-downstream",
            post(handlers::query_downstream),
        )
        .route(
            "/api/v1/data-governance/lineage/:id/graph",
            get(handlers::get_lineage_graph),
        )
        .route(
            "/api/v1/data-governance/lineage/impact-analysis",
            post(handlers::get_impact_analysis),
        )
        .route(
            "/api/v1/data-governance/lineage/export",
            post(handlers::export_lineage),
        )
        .route(
            "/api/v1/data-governance/lineage/persist",
            post(handlers::persist_lineage),
        )
        .route(
            "/api/v1/data-governance/lineage/load",
            post(handlers::load_lineage),
        )
        .route(
            "/api/v1/data-governance/classification/classify",
            post(handlers::classify_data),
        )
        .route(
            "/api/v1/data-governance/masking/mask",
            post(handlers::mask_data),
        )
        // .route(
        //     "/api/v1/data-governance/compliance-reporting/generate",
        //     post(handlers::generate_compliance_report),
        // )
        // .route(
        //     "/api/v1/data-governance/compliance-reporting/reports",
        //     get(handlers::list_compliance_reports),
        // )
        // .route(
        //     "/api/v1/data-governance/compliance-reporting/reports/sign",
        //     post(handlers::sign_compliance_report),
        // )
        // .route(
        //     "/api/v1/data-governance/compliance-reporting/reports/export",
        //     post(handlers::export_compliance_report),
        // )
        // .route("/api/v1/edge/filter", post(handlers::filter_data))
        // .route(
        //     "/api/v1/edge/filter/statistics",
        //     get(handlers::get_filter_statistics),
        // )
        // .route(
        //     "/api/v1/edge/filter/configs",
        //     get(handlers::list_filter_configs),
        // )
        // .route(
        //     "/api/v1/edge/filter/config/:config_key",
        //     get(handlers::get_filter_config),
        // )
        // .route(
        //     "/api/v1/edge/filter/config",
        //     put(handlers::update_filter_config),
        // )
        .layer(ServiceBuilder::new().layer(Extension(auth_manager.clone())))
        .layer(ServiceBuilder::new().layer(Extension(rate_limiter.clone())))
        .route_layer(axum::middleware::from_fn(jwt_auth_middleware))
        .route_layer(axum::middleware::from_fn(rate_limit_middleware));

    let admin_routes = Router::new()
        .route("/api/v1/users", post(handlers::create_user))
        .route("/api/v1/users/:id/role", put(handlers::update_user_role))
        .layer(ServiceBuilder::new().layer(Extension(auth_manager.clone())))
        .layer(ServiceBuilder::new().layer(Extension(rate_limiter.clone())))
        .route_layer(axum::middleware::from_fn(jwt_auth_middleware))
        .route_layer(axum::middleware::from_fn(rate_limit_middleware));

    let router = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(admin_routes);

    // 暂时注释掉skill_marketplace的路由，让代码先编译通过
    // if let Some(skill_marketplace) = &state.skill_marketplace {
    //     router = router.nest(
    //         "/api/v1/skill-marketplace",
    //         skill_marketplace::create_marketplace_router(skill_marketplace.clone()),
    //     );
    // }

    router.with_state(state)
}
