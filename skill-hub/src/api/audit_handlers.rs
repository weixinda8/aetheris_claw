use crate::api::models::*;
use crate::api::AppState;
use crate::constants::*;
use crate::utils::Result;
use crate::utils::SkillHubError;
use crate::utils::audit::*;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use tracing::{info, instrument};

#[instrument(skip(state))]
pub async fn submit_for_audit(
    State(state): State<AppState>,
    Path(skill_id): Path<uuid::Uuid>,
    Json(request): Json<SubmitForAuditRequest>,
) -> Result<impl IntoResponse> {
    let skill = sqlx::query!(
        "SELECT status FROM skills WHERE id = $1",
        skill_id,
    )
    .fetch_optional(&state.db_pool)
    .await?
    .ok_or_else(|| SkillHubError::NotFound("Skill not found".to_string()))?;

    let current_status = SkillStatus::from(skill.status.as_str());
    if current_status != SkillStatus::Draft {
        return Err(SkillHubError::Validation(
            "Only draft skills can be submitted for audit".to_string(),
        ));
    }

    let mut tx = state.db_pool.begin().await?;

    sqlx::query!(
        "UPDATE skills SET status = $1, updated_at = NOW() WHERE id = $2",
        "pending",
        skill_id,
    )
    .execute(&mut *tx)
    .await?;

    let audit_record_id = uuid::Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO skill_audit_records (id, skill_id, stage, status, comments, started_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        audit_record_id,
        skill_id,
        "automated_scan",
        "in_progress",
        request.comments,
        Utc::now(),
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    info!("Skill {} submitted for audit", skill_id);

    Ok(StatusCode::OK)
}

#[instrument(skip(state))]
pub async fn get_audit_queue(
    State(state): State<AppState>,
    Query(query): Query<AuditQueueQuery>,
) -> Result<impl IntoResponse> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(SKILL_DEFAULT_PAGE_SIZE as u32);
    let offset = (page - 1) * page_size;

    let sql = r#"
        SELECT 
            s.id as skill_id,
            s.name as skill_name,
            s.author_id,
            u.username as author_name,
            sar.stage as current_stage,
            sar.status,
            s.priority,
            sar.started_at as submitted_at,
            EXTRACT(EPOCH FROM (NOW() - sar.started_at)) as waiting_time_seconds
        FROM skills s
        JOIN skill_audit_records sar ON s.id = sar.skill_id
        LEFT JOIN users u ON s.author_id = u.id
        WHERE s.status = 'pending'
        ORDER BY 
            CASE s.priority 
                WHEN 'High' THEN 1 
                WHEN 'Medium' THEN 2 
                WHEN 'Low' THEN 3 
            END,
            sar.started_at ASC
        LIMIT $1 OFFSET $2
    "#;

    let items: Vec<AuditQueueItem> = sqlx::query_as(sql)
        .bind(page_size as i64)
        .bind(offset as i64)
        .fetch_all(&state.db_pool)
        .await?;

    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM skills s JOIN skill_audit_records sar ON s.id = sar.skill_id WHERE s.status = 'pending'"
    )
        .fetch_one(&state.db_pool)
        .await?;

    Ok(Json(AuditQueueResponse {
        items,
        total: total.0 as u64,
        page,
        page_size,
    }))
}

#[instrument(skip(state))]
pub async fn perform_audit_action(
    State(state): State<AppState>,
    Path((skill_id, stage)): Path<(uuid::Uuid, String)>,
    Json(request): Json<AuditActionRequest>,
) -> Result<impl IntoResponse> {
    let audit_stage = AuditStage::from(stage.as_str());
    let action = request.action.as_str();

    let mut tx = state.db_pool.begin().await?;

    let current_audit = sqlx::query!(
        r#"
        SELECT id, stage, status FROM skill_audit_records 
        WHERE skill_id = $1 AND stage = $2
        ORDER BY created_at DESC LIMIT 1
        "#,
        skill_id,
        String::from(audit_stage.clone()),
    )
    .fetch_optional(&mut *tx)
    .await?;

    let audit_record = current_audit.ok_or_else(|| {
        SkillHubError::NotFound("Audit record not found for this stage".to_string())
    })?;

    if audit_record.status != "in_progress" {
        return Err(SkillHubError::Validation(
            "This audit stage is not in progress".to_string(),
        ));
    }

    let new_status = match action {
        "approve" => "approved",
        "reject" => "rejected",
        "request_changes" => "changes_requested",
        _ => {
            return Err(SkillHubError::Validation(
                "Invalid audit action".to_string(),
            ));
        }
    };

    sqlx::query!(
        r#"
        UPDATE skill_audit_records 
        SET status = $1, comments = $2, findings = $3, completed_at = NOW(), updated_at = NOW()
        WHERE id = $4
        "#,
        new_status,
        request.comments,
        request.findings,
        audit_record.id,
    )
    .execute(&mut *tx)
    .await?;

    if action == "approve" {
        if let Some(next_stage) = AuditWorkflow::next_audit_stage(&audit_stage) {
            let next_audit_id = uuid::Uuid::new_v4();
            sqlx::query!(
                r#"
                INSERT INTO skill_audit_records (id, skill_id, stage, status, started_at)
                VALUES ($1, $2, $3, $4, $5)
                "#,
                next_audit_id,
                skill_id,
                String::from(next_stage),
                "in_progress",
                Utc::now(),
            )
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query!(
                r#"
                UPDATE skills 
                SET status = 'published', published_at = NOW(), updated_at = NOW()
                WHERE id = $1
                "#,
                skill_id,
            )
            .execute(&mut *tx)
            .await?;
        }
    } else if action == "reject" {
        sqlx::query!(
            r#"
            UPDATE skills 
            SET status = 'blocked', updated_at = NOW()
            WHERE id = $1
            "#,
            skill_id,
        )
        .execute(&mut *tx)
        .await?;
    } else if action == "request_changes" {
        sqlx::query!(
            r#"
            UPDATE skills 
            SET status = 'draft', updated_at = NOW()
            WHERE id = $1
            "#,
            skill_id,
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    info!(
        "Audit action {} performed on skill {} at stage {}",
        action, skill_id, stage
    );

    Ok(StatusCode::OK)
}

#[instrument(skip(state))]
pub async fn get_skill_audit_history(
    State(state): State<AppState>,
    Path(skill_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse> {
    let records = sqlx::query_as!(
        AuditRecord,
        r#"
        SELECT id, skill_id, stage, reviewer_id, status, comments, findings, 
               started_at, completed_at, created_at, updated_at
        FROM skill_audit_records
        WHERE skill_id = $1
        ORDER BY created_at ASC
        "#,
        skill_id,
    )
    .fetch_all(&state.db_pool)
    .await?;

    Ok(Json(SkillAuditHistoryResponse {
        skill_id,
        records,
    }))
}

#[instrument(skip(state))]
pub async fn update_skill_status(
    State(state): State<AppState>,
    Path(skill_id): Path<uuid::Uuid>,
    Json(request): Json<UpdateSkillStatusRequest>,
) -> Result<impl IntoResponse> {
    let skill = sqlx::query!(
        "SELECT status FROM skills WHERE id = $1",
        skill_id,
    )
    .fetch_optional(&state.db_pool)
    .await?
    .ok_or_else(|| SkillHubError::NotFound("Skill not found".to_string()))?;

    let from_status = SkillStatus::from(skill.status.as_str());
    let to_status = SkillStatus::from(request.status.as_str());

    if !AuditWorkflow::can_transition(&from_status, &to_status) {
        return Err(SkillHubError::Validation(format!(
            "Cannot transition from {:?} to {:?}",
            from_status, to_status
        )));
    }

    let mut updates = vec!["status = $1".to_string()];
    let mut params = vec![request.status.clone()];

    if to_status == SkillStatus::Published {
        updates.push("published_at = NOW()".to_string());
    } else if to_status == SkillStatus::Deprecated {
        updates.push("deprecated_at = NOW()".to_string());
    }

    let set_clause = updates.join(", ");
    let sql = format!("UPDATE skills SET {} WHERE id = ${}", set_clause, params.len() + 1);

    let mut query_builder = sqlx::query(&sql);
    for param in params {
        query_builder = query_builder.bind(param);
    }
    query_builder = query_builder.bind(skill_id);

    let result = query_builder.execute(&state.db_pool).await?;

    if result.rows_affected() == 0 {
        return Err(SkillHubError::NotFound("Skill not found".to_string()));
    }

    info!("Skill {} status updated to {}", skill_id, request.status);

    Ok(StatusCode::OK)
}

#[instrument(skip(state))]
pub async fn update_skill_permission(
    State(state): State<AppState>,
    Path(skill_id): Path<uuid::Uuid>,
    Json(request): Json<UpdateSkillPermissionRequest>,
) -> Result<impl IntoResponse> {
    let result = sqlx::query!(
        "UPDATE skills SET permission_level = $1, updated_at = NOW() WHERE id = $2",
        request.permission_level,
        skill_id,
    )
    .execute(&state.db_pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(SkillHubError::NotFound("Skill not found".to_string()));
    }

    info!(
        "Skill {} permission level updated to {}",
        skill_id, request.permission_level
    );

    Ok(StatusCode::OK)
}

#[instrument(skip(state))]
pub async fn run_automated_scan(
    State(state): State<AppState>,
    Path(skill_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse> {
    let scan_result = AutomatedScanner::scan_skill(skill_id).await?;

    let audit_record = sqlx::query!(
        r#"
        SELECT id FROM skill_audit_records 
        WHERE skill_id = $1 AND stage = 'automated_scan' AND status = 'in_progress'
        ORDER BY created_at DESC LIMIT 1
        "#,
        skill_id,
    )
    .fetch_optional(&state.db_pool)
    .await?;

    if let Some(record) = audit_record {
        let status = if scan_result.passed { "approved" } else { "rejected" };
        sqlx::query!(
            r#"
            UPDATE skill_audit_records 
            SET status = $1, findings = $2, completed_at = NOW(), updated_at = NOW()
            WHERE id = $3
            "#,
            status,
            serde_json::to_value(&scan_result.findings)?,
            record.id,
        )
        .execute(&state.db_pool)
        .await?;

        if scan_result.passed {
            let next_audit_id = uuid::Uuid::new_v4();
            sqlx::query!(
                r#"
                INSERT INTO skill_audit_records (id, skill_id, stage, status, started_at)
                VALUES ($1, $2, $3, $4, $5)
                "#,
                next_audit_id,
                skill_id,
                "junior_review",
                "in_progress",
                Utc::now(),
            )
            .execute(&state.db_pool)
            .await?;
        } else {
            sqlx::query!(
                "UPDATE skills SET status = 'draft', updated_at = NOW() WHERE id = $1",
                skill_id,
            )
            .execute(&state.db_pool)
            .await?;
        }
    }

    Ok(Json(AutomatedScanResponse {
        skill_id,
        scan_result,
    }))
}

#[instrument(skip(state))]
pub async fn get_audit_stats(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let total_pending: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM skills WHERE status = 'pending'")
        .fetch_one(&state.db_pool)
        .await?;

    let in_automated_scan: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM skill_audit_records WHERE stage = 'automated_scan' AND status = 'in_progress'"
    )
        .fetch_one(&state.db_pool)
        .await?;

    let in_junior_review: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM skill_audit_records WHERE stage = 'junior_review' AND status = 'in_progress'"
    )
        .fetch_one(&state.db_pool)
        .await?;

    let in_senior_review: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM skill_audit_records WHERE stage = 'senior_review' AND status = 'in_progress'"
    )
        .fetch_one(&state.db_pool)
        .await?;

    let today = Utc::now().date_naive();
    let completed_today: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM skill_audit_records WHERE DATE(completed_at) = $1 AND status IN ('approved', 'rejected')"
    )
        .bind(today)
        .fetch_one(&state.db_pool)
        .await?;

    let avg_wait: (Option<f64>,) = sqlx::query_as(
        r#"
        SELECT AVG(EXTRACT(EPOCH FROM (COALESCE(completed_at, NOW()) - started_at)))
        FROM skill_audit_records
        WHERE status = 'in_progress'
        "#
    )
        .fetch_one(&state.db_pool)
        .await?;

    Ok(Json(AuditStatsResponse {
        total_pending: total_pending.0,
        in_automated_scan: in_automated_scan.0,
        in_junior_review: in_junior_review.0,
        in_senior_review: in_senior_review.0,
        completed_today: completed_today.0,
        average_wait_time_seconds: avg_wait.0.unwrap_or(0.0) as i64,
    }))
}
