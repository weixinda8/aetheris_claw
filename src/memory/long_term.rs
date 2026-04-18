use crate::memory::MemoryItem;
use crate::utils::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryType {
    Experience,
    Skill,
    Case,
    Pattern,
    Heuristic,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Experience => "experience",
            MemoryType::Skill => "skill",
            MemoryType::Case => "case",
            MemoryType::Pattern => "pattern",
            MemoryType::Heuristic => "heuristic",
        }
    }
}

impl std::str::FromStr for MemoryType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "skill" => MemoryType::Skill,
            "case" => MemoryType::Case,
            "pattern" => MemoryType::Pattern,
            "heuristic" => MemoryType::Heuristic,
            _ => MemoryType::Experience,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LongTermMemoryRecord {
    pub id: Uuid,
    pub memory_type: String,
    pub content: serde_json::Value,
    pub embedding: Option<Vec<f32>>,
    pub tags: Option<Vec<String>>,
    pub importance: f64,
    pub usage_count: i32,
    pub last_accessed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContent {
    pub name: String,
    pub description: String,
    pub version: String,
    pub parameters: serde_json::Value,
    pub implementation: Option<String>,
    pub examples: Vec<serde_json::Value>,
    pub success_rate: f64,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceContent {
    pub situation: String,
    pub action: String,
    pub outcome: String,
    pub lessons_learned: Vec<String>,
    pub context: serde_json::Value,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseContent {
    pub title: String,
    pub problem: String,
    pub solution: serde_json::Value,
    pub steps: Vec<String>,
    pub references: Vec<String>,
    pub tags: Vec<String>,
}

pub struct LongTermMemory {
    pool: Option<PgPool>,
}

impl LongTermMemory {
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
        let id = Uuid::parse_str(&item.id).unwrap_or_else(|_| Uuid::new_v4());
        self.store_internal(
            id,
            MemoryType::Experience,
            item.content,
            None,
            item.tags,
            item.importance,
        )
        .await?;
        Ok(())
    }

    async fn store_internal(
        &self,
        id: Uuid,
        memory_type: MemoryType,
        content: serde_json::Value,
        embedding: Option<Vec<f32>>,
        tags: Vec<String>,
        importance: f64,
    ) -> Result<Uuid> {
        if let Some(pool) = &self.pool {
            let now = chrono::Utc::now();
            sqlx::query(
                r#"
                INSERT INTO long_term_memory 
                (id, memory_type, content, embedding, tags, importance, usage_count, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (id) DO UPDATE SET
                    content = EXCLUDED.content,
                    embedding = EXCLUDED.embedding,
                    tags = EXCLUDED.tags,
                    importance = EXCLUDED.importance,
                    updated_at = EXCLUDED.updated_at
                "#
            )
            .bind(id)
            .bind(memory_type.as_str())
            .bind(&content)
            .bind(embedding.as_ref())
            .bind(&tags)
            .bind(importance)
            .bind(0)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        }
        Ok(id)
    }

    pub async fn store_skill(&self, skill: SkillContent, importance: f64) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let category = skill.category.clone();
        let content = serde_json::to_value(skill)?;
        let tags = vec!["skill".to_string(), category];
        self.store_internal(id, MemoryType::Skill, content, None, tags, importance)
            .await
    }

    pub async fn store_experience(
        &self,
        experience: ExperienceContent,
        importance: f64,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let success = experience.success;
        let content = serde_json::to_value(experience)?;
        let mut tags = vec!["experience".to_string()];
        if success {
            tags.push("success".to_string());
        } else {
            tags.push("failure".to_string());
        }
        self.store_internal(id, MemoryType::Experience, content, None, tags, importance)
            .await
    }

    pub async fn store_case(&self, case: CaseContent, importance: f64) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let content = serde_json::to_value(case.clone())?;
        let mut tags = vec!["case".to_string()];
        tags.extend(case.tags);
        self.store_internal(id, MemoryType::Case, content, None, tags, importance)
            .await
    }

    pub async fn retrieve(&self, query: &str) -> Result<Vec<MemoryItem>> {
        let mut results = Vec::new();
        if let Some(pool) = &self.pool {
            let records: Vec<LongTermMemoryRecord> = sqlx::query_as(
                r#"
                SELECT id, memory_type, content, embedding, tags, importance, 
                       usage_count, last_accessed_at, created_at, updated_at
                FROM long_term_memory
                WHERE content::text ILIKE $1 OR ($2 = ANY(tags))
                ORDER BY importance DESC, usage_count DESC
                LIMIT 100
                "#,
            )
            .bind(format!("%{}%", query))
            .bind(query)
            .fetch_all(pool)
            .await?;

            for record in records {
                self.increment_usage(record.id).await?;
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

    pub async fn get(&self, id: Uuid) -> Result<Option<LongTermMemoryRecord>> {
        if let Some(pool) = &self.pool {
            let record: Option<LongTermMemoryRecord> = sqlx::query_as(
                r#"
                SELECT id, memory_type, content, embedding, tags, importance, 
                       usage_count, last_accessed_at, created_at, updated_at
                FROM long_term_memory
                WHERE id = $1
                "#,
            )
            .bind(id)
            .fetch_optional(pool)
            .await?;

            if let Some(r) = &record {
                self.increment_usage(r.id).await?;
            }
            Ok(record)
        } else {
            Ok(None)
        }
    }

    pub async fn get_by_type(
        &self,
        memory_type: MemoryType,
        limit: usize,
    ) -> Result<Vec<LongTermMemoryRecord>> {
        if let Some(pool) = &self.pool {
            let records: Vec<LongTermMemoryRecord> = sqlx::query_as(
                r#"
                SELECT id, memory_type, content, embedding, tags, importance, 
                       usage_count, last_accessed_at, created_at, updated_at
                FROM long_term_memory
                WHERE memory_type = $1
                ORDER BY importance DESC, usage_count DESC
                LIMIT $2
                "#,
            )
            .bind(memory_type.as_str())
            .bind(limit as i64)
            .fetch_all(pool)
            .await?;
            Ok(records)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn get_by_tags(
        &self,
        tags: Vec<String>,
        limit: usize,
    ) -> Result<Vec<LongTermMemoryRecord>> {
        if let Some(pool) = &self.pool {
            let records: Vec<LongTermMemoryRecord> = sqlx::query_as(
                r#"
                SELECT id, memory_type, content, embedding, tags, importance, 
                       usage_count, last_accessed_at, created_at, updated_at
                FROM long_term_memory
                WHERE tags && $1
                ORDER BY importance DESC, usage_count DESC
                LIMIT $2
                "#,
            )
            .bind(&tags)
            .bind(limit as i64)
            .fetch_all(pool)
            .await?;
            Ok(records)
        } else {
            Ok(Vec::new())
        }
    }

    async fn increment_usage(&self, id: Uuid) -> Result<()> {
        if let Some(pool) = &self.pool {
            sqlx::query(
                r#"
                UPDATE long_term_memory
                SET usage_count = usage_count + 1,
                    last_accessed_at = $1,
                    updated_at = $1
                WHERE id = $2
                "#,
            )
            .bind(chrono::Utc::now())
            .bind(id)
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    pub async fn update_importance(&self, id: Uuid, importance: f64) -> Result<()> {
        if let Some(pool) = &self.pool {
            sqlx::query(
                r#"
                UPDATE long_term_memory
                SET importance = $1, updated_at = $2
                WHERE id = $3
                "#,
            )
            .bind(importance)
            .bind(chrono::Utc::now())
            .bind(id)
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    pub async fn update_embedding(&self, id: Uuid, embedding: Vec<f32>) -> Result<()> {
        if let Some(pool) = &self.pool {
            sqlx::query(
                r#"
                UPDATE long_term_memory
                SET embedding = $1, updated_at = $2
                WHERE id = $3
                "#,
            )
            .bind(&embedding)
            .bind(chrono::Utc::now())
            .bind(id)
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        if let Some(pool) = &self.pool {
            sqlx::query("DELETE FROM long_term_memory WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await?;
        }
        Ok(())
    }

    pub async fn get_most_used(&self, limit: usize) -> Result<Vec<LongTermMemoryRecord>> {
        if let Some(pool) = &self.pool {
            let records: Vec<LongTermMemoryRecord> = sqlx::query_as(
                r#"
                SELECT id, memory_type, content, embedding, tags, importance, 
                       usage_count, last_accessed_at, created_at, updated_at
                FROM long_term_memory
                ORDER BY usage_count DESC, importance DESC
                LIMIT $1
                "#,
            )
            .bind(limit as i64)
            .fetch_all(pool)
            .await?;
            Ok(records)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn get_recently_accessed(&self, limit: usize) -> Result<Vec<LongTermMemoryRecord>> {
        if let Some(pool) = &self.pool {
            let records: Vec<LongTermMemoryRecord> = sqlx::query_as(
                r#"
                SELECT id, memory_type, content, embedding, tags, importance, 
                       usage_count, last_accessed_at, created_at, updated_at
                FROM long_term_memory
                WHERE last_accessed_at IS NOT NULL
                ORDER BY last_accessed_at DESC
                LIMIT $1
                "#,
            )
            .bind(limit as i64)
            .fetch_all(pool)
            .await?;
            Ok(records)
        } else {
            Ok(Vec::new())
        }
    }
}

impl Default for LongTermMemory {
    fn default() -> Self {
        Self::new()
    }
}
