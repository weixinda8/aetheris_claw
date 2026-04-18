pub mod long_term;
pub mod mid_term;
pub mod semantic_search;
pub mod short_term;
pub mod vector_db;
pub mod versioned_state;

pub use long_term::{
    CaseContent, ExperienceContent, LongTermMemory, LongTermMemoryRecord, MemoryType, SkillContent,
};
pub use mid_term::{MidTermMemory, MidTermMemoryRecord, TaskChainRecord, TaskStatus, TaskStep};
pub use short_term::{ExecutionContext, LruCache, ShortTermMemory};
pub use vector_db::{SearchResult, VectorDatabase, VectorDocument};
pub use versioned_state::{StateSnapshot, StateVersion, VersionedState};

use crate::utils::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub content: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
    pub importance: f64,
}

impl MemoryItem {
    pub fn new(content: serde_json::Value, tags: Vec<String>, importance: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            timestamp: chrono::Utc::now(),
            tags,
            importance,
        }
    }
}

pub struct MemorySystem {
    short_term: ShortTermMemory,
    mid_term: MidTermMemory,
    long_term: LongTermMemory,
    vector_db: VectorDatabase,
    versioned_state: VersionedState,
}

impl MemorySystem {
    pub fn new() -> Self {
        Self {
            short_term: ShortTermMemory::new(),
            mid_term: MidTermMemory::new(),
            long_term: LongTermMemory::new(),
            vector_db: VectorDatabase::new(),
            versioned_state: VersionedState::new(),
        }
    }

    pub async fn with_pg_pool(pool: PgPool) -> Result<Self> {
        Ok(Self {
            short_term: ShortTermMemory::new(),
            mid_term: MidTermMemory::with_pool(pool.clone()).await?,
            long_term: LongTermMemory::with_pool(pool.clone()).await?,
            vector_db: VectorDatabase::new(),
            versioned_state: VersionedState::with_pool(pool).await?,
        })
    }

    pub fn set_pg_pool(&mut self, pool: PgPool) {
        self.mid_term.set_pool(pool.clone());
        self.long_term.set_pool(pool.clone());
        self.versioned_state.set_pool(pool);
    }

    pub fn short_term(&self) -> &ShortTermMemory {
        &self.short_term
    }

    pub fn mid_term(&self) -> &MidTermMemory {
        &self.mid_term
    }

    pub fn long_term(&self) -> &LongTermMemory {
        &self.long_term
    }

    pub fn vector_db(&self) -> &VectorDatabase {
        &self.vector_db
    }

    pub fn versioned_state(&self) -> &VersionedState {
        &self.versioned_state
    }

    pub async fn store(&self, item: MemoryItem) -> Result<()> {
        self.short_term.store(item.clone()).await?;
        self.mid_term.store(item.clone()).await?;
        self.long_term.store(item).await?;
        Ok(())
    }

    pub async fn store_with_session(&self, session_id: Uuid, item: MemoryItem) -> Result<Uuid> {
        self.short_term.store(item.clone()).await?;
        let id = self
            .mid_term
            .store_with_session(session_id, item.clone())
            .await?;
        self.long_term.store(item).await?;
        Ok(id)
    }

    pub async fn retrieve(&self, query: &str) -> Result<Vec<MemoryItem>> {
        let mut results = Vec::new();
        results.extend(self.short_term.retrieve(query).await?);
        results.extend(self.mid_term.retrieve(query).await?);
        results.extend(self.long_term.retrieve(query).await?);
        Ok(results)
    }

    pub async fn retrieve_by_session(&self, session_id: Uuid) -> Result<Vec<MemoryItem>> {
        let mut results = Vec::new();
        results.extend(self.short_term.retrieve(&session_id.to_string()).await?);
        results.extend(self.mid_term.get_by_session(session_id).await?);
        Ok(results)
    }
}

impl Default for MemorySystem {
    fn default() -> Self {
        Self::new()
    }
}
