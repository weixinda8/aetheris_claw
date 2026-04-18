pub mod handlers;
pub mod models;
pub mod audit_handlers;

use axum::{
    Router,
    routing::{get, post, put, delete},
};
use std::sync::Arc;
use qdrant_client::Qdrant;
use sqlx::PgPool;
use crate::config::AppConfig;

pub use models::*;
pub use handlers::*;
pub use audit_handlers::*;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub qdrant_client: Arc<Qdrant>,
    pub config: AppConfig,
}

impl AppState {
    pub fn new(db_pool: PgPool, qdrant_client: Qdrant, config: AppConfig) -> Self {
        Self {
            db_pool,
            qdrant_client: Arc::new(qdrant_client),
            config,
        }
    }
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/api/health", get(health_check))
        .route("/api/stats", get(get_stats))
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/skills", get(list_skills))
        .route("/api/skills", post(create_skill))
        .route("/api/skills/:id", get(get_skill))
        .route("/api/skills/:id", put(update_skill))
        .route("/api/skills/:id/download", get(download_skill))
        .route("/api/skills/executions", post(record_execution))
        .route("/api/skills/:id/reviews", get(list_reviews))
        .route("/api/skills/:id/reviews", post(create_review))
        .route("/api/skills/:id/rating", get(get_skill_rating))
        .route("/api/skills/:id/versions", get(list_skill_versions))
        .route("/api/skills/:id/versions", post(create_skill_version))
        .route("/api/skills/:id/versions/:version", get(get_skill_version))
        .route("/api/skills/:id/submit-audit", post(submit_for_audit))
        .route("/api/skills/:id/status", put(update_skill_status))
        .route("/api/skills/:id/permission", put(update_skill_permission))
        .route("/api/skills/:id/audit-history", get(get_skill_audit_history))
        .route("/api/skills/:id/automated-scan", post(run_automated_scan))
        .route("/api/audit/queue", get(get_audit_queue))
        .route("/api/audit/stats", get(get_audit_stats))
        .route("/api/audit/skills/:id/stages/:stage", post(perform_audit_action))
        .with_state(state)
}
