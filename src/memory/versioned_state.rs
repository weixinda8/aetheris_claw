use crate::utils::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StateVersion {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub entity_type: String,
    pub version: i64,
    pub state: serde_json::Value,
    pub change_message: Option<String>,
    pub created_by: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub entity_id: Uuid,
    pub entity_type: String,
    pub version: i64,
    pub state: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct VersionedState {
    pool: Option<PgPool>,
}

impl VersionedState {
    pub fn new() -> Self {
        Self { pool: None }
    }

    pub async fn with_pool(pool: PgPool) -> Result<Self> {
        Ok(Self { pool: Some(pool) })
    }

    pub fn set_pool(&mut self, pool: PgPool) {
        self.pool = Some(pool);
    }

    pub async fn commit(
        &self,
        entity_id: Uuid,
        entity_type: String,
        state: serde_json::Value,
        message: Option<String>,
        created_by: Option<String>,
    ) -> Result<i64> {
        if let Some(pool) = &self.pool {
            let next_version = self.get_next_version(entity_id, &entity_type).await?;
            let now = chrono::Utc::now();
            let id = Uuid::new_v4();

            sqlx::query(
                r#"
                INSERT INTO state_versions 
                (id, entity_id, entity_type, version, state, change_message, created_by, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
            )
            .bind(id)
            .bind(entity_id)
            .bind(&entity_type)
            .bind(next_version)
            .bind(&state)
            .bind(message)
            .bind(created_by)
            .bind(now)
            .execute(pool)
            .await?;

            Ok(next_version)
        } else {
            Ok(1)
        }
    }

    async fn get_next_version(&self, entity_id: Uuid, entity_type: &str) -> Result<i64> {
        if let Some(pool) = &self.pool {
            let result: Option<(i64,)> = sqlx::query_as(
                r#"
                SELECT COALESCE(MAX(version), 0) as max_version
                FROM state_versions
                WHERE entity_id = $1 AND entity_type = $2
                "#,
            )
            .bind(entity_id)
            .bind(entity_type)
            .fetch_optional(pool)
            .await?;

            Ok(result.map(|(v,)| v + 1).unwrap_or(1))
        } else {
            Ok(1)
        }
    }

    pub async fn get_version(
        &self,
        entity_id: Uuid,
        entity_type: &str,
        version: i64,
    ) -> Result<Option<StateVersion>> {
        if let Some(pool) = &self.pool {
            let record: Option<StateVersion> = sqlx::query_as(
                r#"
                SELECT id, entity_id, entity_type, version, state, change_message, created_by, created_at
                FROM state_versions
                WHERE entity_id = $1 AND entity_type = $2 AND version = $3
                "#
            )
            .bind(entity_id)
            .bind(entity_type)
            .bind(version)
            .fetch_optional(pool)
            .await?;

            Ok(record)
        } else {
            Ok(None)
        }
    }

    pub async fn get_latest(
        &self,
        entity_id: Uuid,
        entity_type: &str,
    ) -> Result<Option<StateVersion>> {
        if let Some(pool) = &self.pool {
            let record: Option<StateVersion> = sqlx::query_as(
                r#"
                SELECT id, entity_id, entity_type, version, state, change_message, created_by, created_at
                FROM state_versions
                WHERE entity_id = $1 AND entity_type = $2
                ORDER BY version DESC
                LIMIT 1
                "#
            )
            .bind(entity_id)
            .bind(entity_type)
            .fetch_optional(pool)
            .await?;

            Ok(record)
        } else {
            Ok(None)
        }
    }

    pub async fn get_history(
        &self,
        entity_id: Uuid,
        entity_type: &str,
        limit: Option<usize>,
    ) -> Result<Vec<StateVersion>> {
        if let Some(pool) = &self.pool {
            let query = if let Some(l) = limit {
                sqlx::query_as(
                    r#"
                    SELECT id, entity_id, entity_type, version, state, change_message, created_by, created_at
                    FROM state_versions
                    WHERE entity_id = $1 AND entity_type = $2
                    ORDER BY version DESC
                    LIMIT $3
                    "#
                )
                .bind(entity_id)
                .bind(entity_type)
                .bind(l as i64)
            } else {
                sqlx::query_as(
                    r#"
                    SELECT id, entity_id, entity_type, version, state, change_message, created_by, created_at
                    FROM state_versions
                    WHERE entity_id = $1 AND entity_type = $2
                    ORDER BY version DESC
                    "#
                )
                .bind(entity_id)
                .bind(entity_type)
            };

            let records: Vec<StateVersion> = query.fetch_all(pool).await?;
            Ok(records)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn revert_to(
        &self,
        entity_id: Uuid,
        entity_type: String,
        version: i64,
        message: Option<String>,
        created_by: Option<String>,
    ) -> Result<()> {
        if let Some(target_version) = self.get_version(entity_id, &entity_type, version).await? {
            self.commit(
                entity_id,
                entity_type,
                target_version.state,
                message.or_else(|| Some(format!("Reverted to version {}", version))),
                created_by,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn create_snapshot(
        &self,
        entity_id: Uuid,
        entity_type: String,
    ) -> Result<StateSnapshot> {
        if let Some(latest) = self.get_latest(entity_id, &entity_type).await? {
            Ok(StateSnapshot {
                entity_id,
                entity_type,
                version: latest.version,
                state: latest.state,
                timestamp: latest.created_at,
            })
        } else {
            Err(crate::utils::AetherisError::Memory(
                "No state found for entity".to_string(),
            ))
        }
    }

    pub async fn restore_snapshot(
        &self,
        snapshot: StateSnapshot,
        message: Option<String>,
        created_by: Option<String>,
    ) -> Result<i64> {
        self.commit(
            snapshot.entity_id,
            snapshot.entity_type,
            snapshot.state,
            message.or_else(|| {
                Some(format!(
                    "Restored from snapshot at version {}",
                    snapshot.version
                ))
            }),
            created_by,
        )
        .await
    }

    pub async fn compare_versions(
        &self,
        entity_id: Uuid,
        entity_type: &str,
        version1: i64,
        version2: i64,
    ) -> Result<Option<(serde_json::Value, serde_json::Value)>> {
        let v1 = self.get_version(entity_id, entity_type, version1).await?;
        let v2 = self.get_version(entity_id, entity_type, version2).await?;

        match (v1, v2) {
            (Some(a), Some(b)) => Ok(Some((a.state, b.state))),
            _ => Ok(None),
        }
    }

    pub async fn get_audit_trail(
        &self,
        entity_id: Uuid,
        entity_type: &str,
        from_version: Option<i64>,
        to_version: Option<i64>,
    ) -> Result<Vec<StateVersion>> {
        if let Some(pool) = &self.pool {
            let mut query = r#"
                SELECT id, entity_id, entity_type, version, state, change_message, created_by, created_at
                FROM state_versions
                WHERE entity_id = $1 AND entity_type = $2
            "#
            .to_string();

            let mut params = vec![entity_id.to_string(), entity_type.to_string()];
            let mut param_idx = 3;

            if let Some(from) = from_version {
                query.push_str(&format!(" AND version >= ${}", param_idx));
                params.push(from.to_string());
                param_idx += 1;
            }

            if let Some(to) = to_version {
                query.push_str(&format!(" AND version <= ${}", param_idx));
                params.push(to.to_string());
            }

            query.push_str(" ORDER BY version ASC");

            let records: Vec<StateVersion> = sqlx::query_as(&query)
                .bind(entity_id)
                .bind(entity_type)
                .fetch_all(pool)
                .await?;

            Ok(records)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn delete_versions_before(
        &self,
        entity_id: Uuid,
        entity_type: &str,
        version: i64,
    ) -> Result<u64> {
        if let Some(pool) = &self.pool {
            let result = sqlx::query(
                r#"
                DELETE FROM state_versions
                WHERE entity_id = $1 AND entity_type = $2 AND version < $3
                "#,
            )
            .bind(entity_id)
            .bind(entity_type)
            .bind(version)
            .execute(pool)
            .await?;

            Ok(result.rows_affected())
        } else {
            Ok(0)
        }
    }

    pub async fn count_versions(&self, entity_id: Uuid, entity_type: &str) -> Result<i64> {
        if let Some(pool) = &self.pool {
            let result: Option<(i64,)> = sqlx::query_as(
                r#"
                SELECT COUNT(*) as count
                FROM state_versions
                WHERE entity_id = $1 AND entity_type = $2
                "#,
            )
            .bind(entity_id)
            .bind(entity_type)
            .fetch_optional(pool)
            .await?;

            Ok(result.map(|(c,)| c).unwrap_or(0))
        } else {
            Ok(0)
        }
    }
}

impl Default for VersionedState {
    fn default() -> Self {
        Self::new()
    }
}
