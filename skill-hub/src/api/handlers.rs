use crate::api::models::*;
use crate::api::AppState;
use crate::constants::*;
use crate::observability::metrics::gather_metrics;
use crate::utils::Result;
use crate::utils::SkillHubError;
use crate::utils::rating::*;
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::debug;

pub async fn metrics_handler() -> impl IntoResponse {
    match gather_metrics() {
        Ok(metrics) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
            metrics,
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to gather metrics: {}", e),
        ).into_response(),
    }
}

pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
    })
}

pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<impl IntoResponse> {
    let password_hash = hash(&request.password, DEFAULT_COST)?;
    let role = request.role.unwrap_or("user".to_string());

    let user_id = uuid::Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO users (id, username, email, password_hash, role)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        user_id,
        request.username,
        request.email,
        password_hash,
        role,
    )
    .execute(&state.db_pool)
    .await?;

    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, username, email, role, is_active, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
        user_id,
    )
    .fetch_one(&state.db_pool)
    .await?;

    let token = create_jwt_token(&user, &state.config.auth.jwt_secret_key, state.config.auth.jwt_expiration_hours)?;

    Ok(Json(LoginResponse { token, user }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, username, email, role, is_active, created_at, updated_at
        FROM users
        WHERE username = $1
        "#,
        request.username,
    )
    .fetch_optional(&state.db_pool)
    .await?
    .ok_or_else(|| SkillHubError::Authentication("Invalid username or password".to_string()))?;

    let db_user = sqlx::query!(
        "SELECT password_hash FROM users WHERE id = $1",
        user.id,
    )
    .fetch_one(&state.db_pool)
    .await?;

    if !verify(&request.password, &db_user.password_hash)? {
        return Err(SkillHubError::Authentication("Invalid username or password".to_string()));
    }

    let token = create_jwt_token(&user, &state.config.auth.jwt_secret_key, state.config.auth.jwt_expiration_hours)?;

    Ok(Json(LoginResponse { token, user }))
}

fn create_jwt_token(user: &User, secret_key: &str, expiration_hours: i64) -> Result<String> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(expiration_hours))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user.id,
        username: user.username.clone(),
        role: user.role.clone(),
        exp: expiration,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret_key.as_ref()),
    )?;

    Ok(token)
}

pub async fn list_skills(
    State(state): State<AppState>,
    Query(query): Query<SkillSearchQuery>,
) -> Result<impl IntoResponse> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(SKILL_DEFAULT_PAGE_SIZE as u32);
    let offset = (page - 1) * page_size;

    let mut conditions = Vec::new();
    let mut params = Vec::new();
    let mut param_index = 1;

    if let Some(status) = &query.status {
        conditions.push(format!("status = ${}", param_index));
        params.push(status.clone());
        param_index += 1;
    }

    if let Some(category) = &query.category {
        conditions.push(format!("category = ${}", param_index));
        params.push(category.clone());
        param_index += 1;
    }

    if let Some(search_query) = &query.query {
        conditions.push(format!(
            "(name ILIKE ${} OR description ILIKE ${} OR ${} = ANY(tags))",
            param_index,
            param_index + 1,
            param_index + 2
        ));
        let pattern = format!("%{}%", search_query);
        params.push(pattern.clone());
        params.push(pattern);
        params.push(search_query.clone());
        param_index += 3;
    }

    if let Some(tags) = &query.tags {
        if !tags.is_empty() {
            conditions.push(format!("tags && ${}", param_index));
            params.push(tags.clone());
            param_index += 1;
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        r#"
        SELECT id, name, description, version, author_id, category, tags, status,
               download_count, average_rating as "average_rating!", rating_count,
               success_rate as "success_rate!", execution_count,
               created_at, updated_at, published_at, deprecated_at
        FROM skills
        {}
        "#,
        where_clause
    );

    let mut query_builder = sqlx::query_as(&sql);

    for param in params.clone() {
        query_builder = query_builder.bind(param);
    }

    let skills: Vec<Skill> = query_builder.fetch_all(&state.db_pool).await?;

    let rating_config = RatingConfig::default();
    let sort_by = query.sort_by.as_ref().and_then(|s| SortBy::from_str(s)).unwrap_or(SortBy::Recommended);

    let mut skills_with_ratings = Vec::new();
    for skill in &skills {
        let seven_days_ago = Utc::now() - Duration::days(7);

        let recent_downloads = sqlx::query!(
            "SELECT COUNT(*) as count FROM skill_downloads WHERE skill_id = $1 AND downloaded_at >= $2",
            skill.id,
            seven_days_ago
        )
        .fetch_optional(&state.db_pool)
        .await?
        .map(|r| r.count.unwrap_or(0))
        .unwrap_or(0);

        let recent_executions = sqlx::query!(
            "SELECT COUNT(*) as count FROM skill_executions WHERE skill_id = $1 AND executed_at >= $2",
            skill.id,
            seven_days_ago
        )
        .fetch_optional(&state.db_pool)
        .await?
        .map(|r| r.count.unwrap_or(0))
        .unwrap_or(0);

        let audit_quality_score = if skill.status == "published" { 0.8 } else { 0.5 };

        let rating_input = SkillRatingInput {
            skill_id: skill.id,
            average_rating: skill.average_rating,
            rating_count: skill.rating_count,
            download_count: skill.download_count,
            success_rate: skill.success_rate / 100.0,
            execution_count: skill.execution_count,
            created_at: skill.created_at,
            updated_at: skill.updated_at,
            published_at: skill.published_at,
            recent_downloads,
            recent_executions,
            audit_quality_score,
        };

        let rating = calculate_skill_rating(&rating_input, &rating_config);
        skills_with_ratings.push((rating_input, rating));
    }

    let sorted_skills = sort_skills_with_ratings(skills_with_ratings, &sort_by);
    let sorted_skill_ids: Vec<Uuid> = sorted_skills.iter().map(|(_, r)| r.skill_id).collect();

    let start_idx = offset as usize;
    let end_idx = (start_idx + page_size as usize).min(sorted_skill_ids.len());
    let paged_skill_ids = &sorted_skill_ids[start_idx..end_idx];

    let mut skills_with_rating: Vec<SkillWithRating> = Vec::new();
    for skill_id in paged_skill_ids {
        if let Some(skill) = skills.iter().find(|s| s.id == *skill_id) {
            let mut skill_with_rating = SkillWithRating::from(skill.clone());
            
            if let Some((_, rating)) = sorted_skills.iter().find(|(_, r)| r.skill_id == *skill_id) {
                skill_with_rating.rating = Some(SkillRatingResponse {
                    skill_id: rating.skill_id,
                    bayesian_rating: rating.bayesian_rating,
                    normalized_downloads: rating.normalized_downloads,
                    smoothed_success_rate: rating.smoothed_success_rate,
                    activity_score: rating.activity_score,
                    overall_score: rating.overall_score,
                    audit_quality_score: rating.audit_quality_score,
                    trending_score: rating.trending_score,
                });
            }
            
            skills_with_rating.push(skill_with_rating);
        }
    }

    let count_sql = format!(
        "SELECT COUNT(*) FROM skills {}",
        where_clause
    );

    let mut count_query_builder = sqlx::query_as(&count_sql);
    for param in params {
        count_query_builder = count_query_builder.bind(param);
    }

    let total: (i64,) = count_query_builder.fetch_one(&state.db_pool).await?;

    Ok(Json(SkillSearchWithRatingResponse {
        skills: skills_with_rating,
        total: total.0 as u64,
        page,
        page_size,
    }))
}

pub async fn get_skill_rating(
    State(state): State<AppState>,
    Path(skill_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse> {
    let skill = sqlx::query_as!(
        Skill,
        r#"
        SELECT id, name, description, version, author_id, category, tags, status,
               download_count, average_rating as "average_rating!", rating_count,
               success_rate as "success_rate!", execution_count,
               created_at, updated_at, published_at, deprecated_at
        FROM skills
        WHERE id = $1
        "#,
        skill_id,
    )
    .fetch_one(&state.db_pool)
    .await?;

    let seven_days_ago = Utc::now() - Duration::days(7);

    let recent_downloads = sqlx::query!(
        "SELECT COUNT(*) as count FROM skill_downloads WHERE skill_id = $1 AND downloaded_at >= $2",
        skill.id,
        seven_days_ago
    )
    .fetch_optional(&state.db_pool)
    .await?
    .map(|r| r.count.unwrap_or(0))
    .unwrap_or(0);

    let recent_executions = sqlx::query!(
        "SELECT COUNT(*) as count FROM skill_executions WHERE skill_id = $1 AND executed_at >= $2",
        skill.id,
        seven_days_ago
    )
    .fetch_optional(&state.db_pool)
    .await?
    .map(|r| r.count.unwrap_or(0))
    .unwrap_or(0);

    let audit_quality_score = if skill.status == "published" { 0.8 } else { 0.5 };

    let rating_config = RatingConfig::default();
    let rating_input = SkillRatingInput {
        skill_id: skill.id,
        average_rating: skill.average_rating,
        rating_count: skill.rating_count,
        download_count: skill.download_count,
        success_rate: skill.success_rate / 100.0,
        execution_count: skill.execution_count,
        created_at: skill.created_at,
        updated_at: skill.updated_at,
        published_at: skill.published_at,
        recent_downloads,
        recent_executions,
        audit_quality_score,
    };

    let rating = calculate_skill_rating(&rating_input, &rating_config);

    Ok(Json(SkillRatingResponse {
        skill_id: rating.skill_id,
        bayesian_rating: rating.bayesian_rating,
        normalized_downloads: rating.normalized_downloads,
        smoothed_success_rate: rating.smoothed_success_rate,
        activity_score: rating.activity_score,
        overall_score: rating.overall_score,
        audit_quality_score: rating.audit_quality_score,
        trending_score: rating.trending_score,
    }))
}

pub async fn get_skill(
    State(state): State<AppState>,
    Path(skill_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse> {
    let skill = sqlx::query_as!(
        Skill,
        r#"
        SELECT id, name, description, version, author_id, category, tags, status,
               download_count, average_rating as "average_rating!", rating_count,
               success_rate as "success_rate!", execution_count,
               created_at, updated_at, published_at, deprecated_at
        FROM skills
        WHERE id = $1
        "#,
        skill_id,
    )
    .fetch_one(&state.db_pool)
    .await?;

    Ok(Json(skill))
}

pub async fn create_skill(
    State(state): State<AppState>,
    Json(request): Json<CreateSkillRequest>,
) -> Result<impl IntoResponse> {
    let author_id = uuid::Uuid::new_v4();

    let skill_id = uuid::Uuid::new_v4();
    let status = "draft".to_string();
    let call_mode = request.call_mode.unwrap_or("Text".to_string());
    let permission_level = request.permission_level.unwrap_or("Public".to_string());
    let priority = request.priority.unwrap_or("Medium".to_string());
    let categories = request.categories.unwrap_or_else(Vec::new);
    let tags = request.tags.unwrap_or_else(Vec::new);
    let required_permissions = request.required_permissions.unwrap_or_else(Vec::new);
    let dependencies = request.dependencies.unwrap_or_else(Vec::new);
    let metadata = request.metadata.unwrap_or_else(|| json!({}));

    sqlx::query!(
        r#"
        INSERT INTO skills (id, skill_id, name, description, long_description, version, author_id,
                           category, categories, tags, status, call_mode, permission_level,
                           priority, required_permissions, input_schema, output_schema,
                           example_input, example_output, dependencies, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
        "#,
        skill_id,
        request.skill_id,
        request.name,
        request.description,
        request.long_description,
        request.version.clone(),
        author_id,
        request.category,
        &categories[..],
        &tags[..],
        status,
        call_mode,
        permission_level,
        priority,
        &required_permissions[..],
        request.input_schema,
        request.output_schema,
        request.example_input,
        request.example_output,
        &dependencies[..],
        metadata,
    )
    .execute(&state.db_pool)
    .await?;

    let version_id = uuid::Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO skill_versions (id, skill_id, version, content, changelog)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        version_id,
        skill_id,
        request.version.clone(),
        request.content,
        request.changelog,
    )
    .execute(&state.db_pool)
    .await?;

    Ok(Json(CreateSkillResponse {
        skill_id,
        version: request.version,
    }))
}

pub async fn update_skill(
    State(state): State<AppState>,
    Path(skill_id): Path<uuid::Uuid>,
    Json(request): Json<UpdateSkillRequest>,
) -> Result<impl IntoResponse> {
    let mut updates = Vec::new();
    let mut params = Vec::new();
    let mut param_index = 1;

    if let Some(name) = &request.name {
        updates.push(format!("name = ${}", param_index));
        params.push(name.clone());
        param_index += 1;
    }

    if let Some(description) = &request.description {
        updates.push(format!("description = ${}", param_index));
        params.push(description.clone());
        param_index += 1;
    }

    if let Some(long_description) = &request.long_description {
        updates.push(format!("long_description = ${}", param_index));
        params.push(long_description.clone());
        param_index += 1;
    }

    if let Some(version) = &request.version {
        updates.push(format!("version = ${}", param_index));
        params.push(version.clone());
        param_index += 1;
    }

    if let Some(category) = &request.category {
        updates.push(format!("category = ${}", param_index));
        params.push(category.clone());
        param_index += 1;
    }

    if let Some(categories) = &request.categories {
        updates.push(format!("categories = ${}", param_index));
        params.push(categories.clone());
        param_index += 1;
    }

    if let Some(tags) = &request.tags {
        updates.push(format!("tags = ${}", param_index));
        params.push(tags.clone());
        param_index += 1;
    }

    if let Some(status) = &request.status {
        updates.push(format!("status = ${}", param_index));
        params.push(status.clone());
        param_index += 1;
    }

    if let Some(call_mode) = &request.call_mode {
        updates.push(format!("call_mode = ${}", param_index));
        params.push(call_mode.clone());
        param_index += 1;
    }

    if let Some(permission_level) = &request.permission_level {
        updates.push(format!("permission_level = ${}", param_index));
        params.push(permission_level.clone());
        param_index += 1;
    }

    if let Some(priority) = &request.priority {
        updates.push(format!("priority = ${}", param_index));
        params.push(priority.clone());
        param_index += 1;
    }

    if let Some(required_permissions) = &request.required_permissions {
        updates.push(format!("required_permissions = ${}", param_index));
        params.push(required_permissions.clone());
        param_index += 1;
    }

    if let Some(input_schema) = &request.input_schema {
        updates.push(format!("input_schema = ${}", param_index));
        params.push(input_schema.clone());
        param_index += 1;
    }

    if let Some(output_schema) = &request.output_schema {
        updates.push(format!("output_schema = ${}", param_index));
        params.push(output_schema.clone());
        param_index += 1;
    }

    if let Some(example_input) = &request.example_input {
        updates.push(format!("example_input = ${}", param_index));
        params.push(example_input.clone());
        param_index += 1;
    }

    if let Some(example_output) = &request.example_output {
        updates.push(format!("example_output = ${}", param_index));
        params.push(example_output.clone());
        param_index += 1;
    }

    if let Some(dependencies) = &request.dependencies {
        updates.push(format!("dependencies = ${}", param_index));
        params.push(dependencies.clone());
        param_index += 1;
    }

    if let Some(metadata) = &request.metadata {
        updates.push(format!("metadata = ${}", param_index));
        params.push(metadata.clone());
        param_index += 1;
    }

    if updates.is_empty() {
        return Err(SkillHubError::Validation("No fields to update".to_string()));
    }

    updates.push("updated_at = NOW()".to_string());
    let set_clause = updates.join(", ");

    let sql = format!(
        "UPDATE skills SET {} WHERE id = ${}",
        set_clause,
        param_index
    );

    let mut query_builder = sqlx::query(&sql);
    for param in params {
        query_builder = query_builder.bind(param);
    }
    query_builder = query_builder.bind(skill_id);

    let result = query_builder.execute(&state.db_pool).await?;

    if result.rows_affected() == 0 {
        return Err(SkillHubError::NotFound("Skill not found".to_string()));
    }

    if let Some(content) = request.content {
        let current_version = sqlx::query!(
            "SELECT version FROM skills WHERE id = $1",
            skill_id,
        )
        .fetch_one(&state.db_pool)
        .await?;

        let version_id = uuid::Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO skill_versions (id, skill_id, version, content, changelog)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            version_id,
            skill_id,
            current_version.version,
            content,
            request.changelog,
        )
        .execute(&state.db_pool)
        .await?;
    }

    Ok(StatusCode::OK)
}

pub async fn download_skill(
    State(state): State<AppState>,
    Path(skill_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse> {
    let skill_version = sqlx::query_as!(
        SkillVersion,
        r#"
        SELECT id, skill_id, version, content, changelog, created_at
        FROM skill_versions
        WHERE skill_id = $1
        ORDER BY created_at DESC
        LIMIT 1
        "#,
        skill_id,
    )
    .fetch_one(&state.db_pool)
    .await?;

    sqlx::query!(
        "UPDATE skills SET download_count = download_count + 1 WHERE id = $1",
        skill_id,
    )
    .execute(&state.db_pool)
    .await?;

    sqlx::query!(
        "INSERT INTO skill_downloads (skill_id, downloaded_at) VALUES ($1, $2)",
        skill_id,
        Utc::now(),
    )
    .execute(&state.db_pool)
    .await?;

    Ok(Json(DownloadSkillResponse {
        skill_id,
        version: skill_version.version.clone(),
        content: skill_version.content,
        downloaded_at: Utc::now(),
    }))
}

pub async fn record_execution(
    State(state): State<AppState>,
    Json(request): Json<RecordExecutionRequest>,
) -> Result<impl IntoResponse> {
    sqlx::query!(
        r#"
        INSERT INTO skill_executions (skill_id, version, success, executed_at, execution_time_ms, error_message)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        request.skill_id,
        request.version,
        request.success,
        Utc::now(),
        request.execution_time_ms,
        request.error_message,
    )
    .execute(&state.db_pool)
    .await?;

    let stats = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as total,
            SUM(CASE WHEN success THEN 1 ELSE 0 END) as successful
        FROM skill_executions
        WHERE skill_id = $1
        "#,
        request.skill_id,
    )
    .fetch_one(&state.db_pool)
    .await?;

    let total = stats.total.unwrap_or(0);
    let successful = stats.successful.unwrap_or(0);
    let success_rate = if total > 0 {
        (successful as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    sqlx::query!(
        r#"
        UPDATE skills
        SET execution_count = $1, success_rate = $2
        WHERE id = $3
        "#,
        total,
        success_rate,
        request.skill_id,
    )
    .execute(&state.db_pool)
    .await?;

    Ok(StatusCode::OK)
}

pub async fn list_reviews(
    State(state): State<AppState>,
    Path(skill_id): Path<uuid::Uuid>,
    Query(query): Query<ReviewListQuery>,
) -> Result<impl IntoResponse> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(SKILL_DEFAULT_PAGE_SIZE as u32);
    let offset = (page - 1) * page_size;

    let sort_by = query.sort_by.unwrap_or_else(|| "created_at".to_string());
    let order_clause = match sort_by.as_str() {
        "rating" => "ORDER BY rating DESC".to_string(),
        "helpful" => "ORDER BY helpful_count DESC".to_string(),
        _ => "ORDER BY created_at DESC".to_string(),
    };

    let sql = format!(
        r#"
        SELECT id, skill_id, user_id, rating, title, content,
               helpful_count, created_at, updated_at
        FROM skill_reviews
        WHERE skill_id = $1
        {}
        LIMIT $2 OFFSET $3
        "#,
        order_clause
    );

    let reviews = sqlx::query_as!(
        SkillReview,
        &sql,
        skill_id,
        page_size as i64,
        offset as i64,
    )
    .fetch_all(&state.db_pool)
    .await?;

    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM skill_reviews WHERE skill_id = $1",
    )
    .bind(skill_id)
    .fetch_one(&state.db_pool)
    .await?;

    Ok(Json(ReviewListResponse {
        reviews,
        total: total.0 as u64,
        page,
        page_size,
    }))
}

pub async fn create_review(
    State(state): State<AppState>,
    Path(skill_id): Path<uuid::Uuid>,
    Json(request): Json<CreateReviewRequest>,
) -> Result<impl IntoResponse> {
    if request.rating < 1 || request.rating > 5 {
        return Err(SkillHubError::Validation("Rating must be between 1 and 5".to_string()));
    }

    let user_id = uuid::Uuid::new_v4();
    let review_id = uuid::Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO skill_reviews (id, skill_id, user_id, rating, title, content)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (skill_id, user_id) DO UPDATE SET
            rating = EXCLUDED.rating,
            title = EXCLUDED.title,
            content = EXCLUDED.content,
            updated_at = NOW()
        "#,
        review_id,
        skill_id,
        user_id,
        request.rating,
        request.title,
        request.content,
    )
    .execute(&state.db_pool)
    .await?;

    let stats = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as count,
            AVG(rating) as avg
        FROM skill_reviews
        WHERE skill_id = $1
        "#,
        skill_id,
    )
    .fetch_one(&state.db_pool)
    .await?;

    let count = stats.count.unwrap_or(0);
    let avg = stats.avg.unwrap_or(0.0);

    sqlx::query!(
        r#"
        UPDATE skills
        SET rating_count = $1, average_rating = $2
        WHERE id = $3
        "#,
        count,
        avg,
        skill_id,
    )
    .execute(&state.db_pool)
    .await?;

    Ok(StatusCode::OK)
}

pub async fn get_stats(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let total_skills: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM skills")
        .fetch_one(&state.db_pool)
        .await?;

    let published_skills: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM skills WHERE status = 'published'")
        .fetch_one(&state.db_pool)
        .await?;

    let total_downloads: (i64,) = sqlx::query_as("SELECT SUM(download_count) FROM skills")
        .fetch_one(&state.db_pool)
        .await?;

    let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db_pool)
        .await?;

    let total_reviews: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM skill_reviews")
        .fetch_one(&state.db_pool)
        .await?;

    Ok(Json(SkillStats {
        total_skills: total_skills.0,
        published_skills: published_skills.0,
        total_downloads: total_downloads.0.unwrap_or(0),
        total_users: total_users.0,
        total_reviews: total_reviews.0,
    }))
}

pub async fn list_skill_versions(
    State(state): State<AppState>,
    Path(skill_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse> {
    let versions = sqlx::query_as!(
        SkillVersion,
        r#"
        SELECT id, skill_id, version, content, changelog, created_at
        FROM skill_versions
        WHERE skill_id = $1
        ORDER BY created_at DESC
        "#,
        skill_id,
    )
    .fetch_all(&state.db_pool)
    .await?;

    Ok(Json(versions))
}

pub async fn create_skill_version(
    State(state): State<AppState>,
    Path(skill_id): Path<uuid::Uuid>,
    Json(request): Json<CreateSkillVersionRequest>,
) -> Result<impl IntoResponse> {
    let version_id = uuid::Uuid::new_v4();
    
    sqlx::query!(
        r#"
        INSERT INTO skill_versions (id, skill_id, version, content, changelog)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        version_id,
        skill_id,
        request.version,
        request.content,
        request.changelog,
    )
    .execute(&state.db_pool)
    .await?;

    sqlx::query!(
        "UPDATE skills SET version = $1, updated_at = NOW() WHERE id = $2",
        request.version,
        skill_id,
    )
    .execute(&state.db_pool)
    .await?;

    Ok(Json(CreateSkillVersionResponse {
        version_id,
        skill_id,
        version: request.version,
    }))
}

pub async fn get_skill_version(
    State(state): State<AppState>,
    Path((skill_id, version)): Path<(uuid::Uuid, String)>,
) -> Result<impl IntoResponse> {
    let skill_version = sqlx::query_as!(
        SkillVersion,
        r#"
        SELECT id, skill_id, version, content, changelog, created_at
        FROM skill_versions
        WHERE skill_id = $1 AND version = $2
        "#,
        skill_id,
        version,
    )
    .fetch_one(&state.db_pool)
    .await?;

    Ok(Json(skill_version))
}
