use crate::memory::MemoryItem;
use crate::utils::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MidTermMemoryRecord {
    pub id: Uuid,
    pub session_id: Uuid,
    pub content: serde_json::Value,
    pub tags: Option<Vec<String>>,
    pub importance: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskChainRecord {
    pub id: Uuid,
    pub session_id: Uuid,
    pub task_chain: serde_json::Value,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: TaskStatus,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        }
    }
}

impl std::str::FromStr for TaskStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "in_progress" => TaskStatus::InProgress,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            _ => TaskStatus::Pending,
        })
    }
}

pub struct MidTermMemory {
    pool: Option<PgPool>,
}

impl MidTermMemory {
    pub fn new() -> Self {
        Self { pool: None }
    }

    pub async fn with_pool(pool: PgPool) -> Result<Self> {
        Ok(Self { pool: Some(pool) })
    }

    pub fn set_pool(&mut self, pool: PgPool) {
        self.pool = Some(pool);
    }

    pub async fn store(&self, item: MemoryItem) -> Result<()> {
        if let Some(pool) = &self.pool {
            let id = Uuid::parse_str(&item.id).unwrap_or_else(|_| Uuid::new_v4());
            sqlx::query(
                r#"
                INSERT INTO mid_term_memory (id, session_id, content, tags, importance, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (id) DO UPDATE SET
                    content = EXCLUDED.content,
                    tags = EXCLUDED.tags,
                    importance = EXCLUDED.importance,
                    updated_at = EXCLUDED.updated_at
                "#
            )
            .bind(id)
            .bind(Uuid::new_v4())
            .bind(&item.content)
            .bind(&item.tags)
            .bind(item.importance)
            .bind(item.timestamp)
            .bind(chrono::Utc::now())
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    pub async fn store_with_session(&self, session_id: Uuid, item: MemoryItem) -> Result<Uuid> {
        let id = Uuid::parse_str(&item.id).unwrap_or_else(|_| Uuid::new_v4());
        if let Some(pool) = &self.pool {
            sqlx::query(
                r#"
                INSERT INTO mid_term_memory (id, session_id, content, tags, importance, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (id) DO UPDATE SET
                    content = EXCLUDED.content,
                    tags = EXCLUDED.tags,
                    importance = EXCLUDED.importance,
                    updated_at = EXCLUDED.updated_at
                "#
            )
            .bind(id)
            .bind(session_id)
            .bind(&item.content)
            .bind(&item.tags)
            .bind(item.importance)
            .bind(item.timestamp)
            .bind(chrono::Utc::now())
            .execute(pool)
            .await?;
        }
        Ok(id)
    }

    pub async fn retrieve(&self, query: &str) -> Result<Vec<MemoryItem>> {
        let mut results = Vec::new();
        if let Some(pool) = &self.pool {
            let records: Vec<MidTermMemoryRecord> = sqlx::query_as(
                r#"
                SELECT id, session_id, content, tags, importance, created_at, updated_at
                FROM mid_term_memory
                WHERE content::text ILIKE $1 OR ($2 = ANY(tags))
                ORDER BY created_at DESC
                "#,
            )
            .bind(format!("%{}%", query))
            .bind(query)
            .fetch_all(pool)
            .await?;

            for record in records {
                results.push(MemoryItem {
                    id: record.id.to_string(),
                    content: record.content,
                    timestamp: record.created_at,
                    tags: record.tags.unwrap_or_default(),
                    importance: record.importance,
                });
            }
        }
        Ok(results)
    }

    pub async fn get_by_session(&self, session_id: Uuid) -> Result<Vec<MemoryItem>> {
        let mut results = Vec::new();
        if let Some(pool) = &self.pool {
            let records: Vec<MidTermMemoryRecord> = sqlx::query_as(
                r#"
                SELECT id, session_id, content, tags, importance, created_at, updated_at
                FROM mid_term_memory
                WHERE session_id = $1
                ORDER BY created_at DESC
                "#,
            )
            .bind(session_id)
            .fetch_all(pool)
            .await?;

            for record in records {
                results.push(MemoryItem {
                    id: record.id.to_string(),
                    content: record.content,
                    timestamp: record.created_at,
                    tags: record.tags.unwrap_or_default(),
                    importance: record.importance,
                });
            }
        }
        Ok(results)
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<MemoryItem>> {
        if let Some(pool) = &self.pool {
            let record: Option<MidTermMemoryRecord> = sqlx::query_as(
                r#"
                SELECT id, session_id, content, tags, importance, created_at, updated_at
                FROM mid_term_memory
                WHERE id = $1
                "#,
            )
            .bind(id)
            .fetch_optional(pool)
            .await?;

            Ok(record.map(|r| MemoryItem {
                id: r.id.to_string(),
                content: r.content,
                timestamp: r.created_at,
                tags: r.tags.unwrap_or_default(),
                importance: r.importance,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn create_task_chain(
        &self,
        session_id: Uuid,
        task_chain: serde_json::Value,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        if let Some(pool) = &self.pool {
            sqlx::query(
                r#"
                INSERT INTO task_chains (id, session_id, task_chain, status, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(id)
            .bind(session_id)
            .bind(&task_chain)
            .bind(TaskStatus::Pending.as_str())
            .bind(chrono::Utc::now())
            .bind(chrono::Utc::now())
            .execute(pool)
            .await?;
        }
        Ok(id)
    }

    pub async fn get_task_chain(&self, id: Uuid) -> Result<Option<TaskChainRecord>> {
        if let Some(pool) = &self.pool {
            let record: Option<TaskChainRecord> = sqlx::query_as(
                r#"
                SELECT id, session_id, task_chain, status, created_at, updated_at
                FROM task_chains
                WHERE id = $1
                "#,
            )
            .bind(id)
            .fetch_optional(pool)
            .await?;
            Ok(record)
        } else {
            Ok(None)
        }
    }

    pub async fn get_task_chains_by_session(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<TaskChainRecord>> {
        if let Some(pool) = &self.pool {
            let records: Vec<TaskChainRecord> = sqlx::query_as(
                r#"
                SELECT id, session_id, task_chain, status, created_at, updated_at
                FROM task_chains
                WHERE session_id = $1
                ORDER BY created_at DESC
                "#,
            )
            .bind(session_id)
            .fetch_all(pool)
            .await?;
            Ok(records)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn update_task_chain_status(&self, id: Uuid, status: TaskStatus) -> Result<()> {
        if let Some(pool) = &self.pool {
            sqlx::query(
                r#"
                UPDATE task_chains
                SET status = $1, updated_at = $2
                WHERE id = $3
                "#,
            )
            .bind(status.as_str())
            .bind(chrono::Utc::now())
            .bind(id)
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    pub async fn update_task_chain(&self, id: Uuid, task_chain: serde_json::Value) -> Result<()> {
        if let Some(pool) = &self.pool {
            sqlx::query(
                r#"
                UPDATE task_chains
                SET task_chain = $1, updated_at = $2
                WHERE id = $3
                "#,
            )
            .bind(&task_chain)
            .bind(chrono::Utc::now())
            .bind(id)
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        if let Some(pool) = &self.pool {
            sqlx::query("DELETE FROM mid_term_memory WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await?;
        }
        Ok(())
    }

    pub async fn delete_by_session(&self, session_id: Uuid) -> Result<()> {
        if let Some(pool) = &self.pool {
            sqlx::query("DELETE FROM mid_term_memory WHERE session_id = $1")
                .bind(session_id)
                .execute(pool)
                .await?;
        }
        Ok(())
    }
}

impl Default for MidTermMemory {
    fn default() -> Self {
        Self::new()
    }
}
