use crate::core::Task;
use crate::utils::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LoadingStrategy {
    Immediate,
    Lazy,
    OnDemand,
    Batch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BudgetStatus {
    WithinBudget,
    Warning(f64),
    Exceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub estimated_cost_usd: f64,
}

impl TokenUsage {
    pub fn new() -> Self {
        Self {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            estimated_cost_usd: 0.0,
        }
    }

    pub fn add(&mut self, other: &TokenUsage) {
        self.prompt_tokens += other.prompt_tokens;
        self.completion_tokens += other.completion_tokens;
        self.total_tokens += other.total_tokens;
        self.estimated_cost_usd += other.estimated_cost_usd;
    }

    pub fn estimate_cost(&mut self, cost_per_1k_tokens: f64) {
        self.estimated_cost_usd = (self.total_tokens as f64 / 1000.0) * cost_per_1k_tokens;
    }
}

impl Default for TokenUsage {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingContext {
    pub context_id: String,
    pub task_id: String,
    pub loading_strategy: LoadingStrategy,
    pub current_depth: u32,
    pub max_depth: u32,
    pub loaded_chunks: Vec<String>,
    pub pending_chunks: Vec<String>,
    pub token_usage: TokenUsage,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_loaded_at: chrono::DateTime<chrono::Utc>,
    pub batch_size: usize,
    pub budget_warning_threshold: f64,
}

impl LoadingContext {
    pub fn new(task_id: String, strategy: LoadingStrategy, max_depth: u32) -> Self {
        let now = chrono::Utc::now();
        Self {
            context_id: uuid::Uuid::new_v4().to_string(),
            task_id,
            loading_strategy: strategy,
            current_depth: 0,
            max_depth,
            loaded_chunks: Vec::new(),
            pending_chunks: Vec::new(),
            token_usage: TokenUsage::new(),
            created_at: now,
            last_loaded_at: now,
            batch_size: 5,
            budget_warning_threshold: 0.8,
        }
    }

    pub fn should_load_next_chunk(&self) -> bool {
        match self.loading_strategy {
            LoadingStrategy::Immediate => true,
            LoadingStrategy::Lazy => self.loaded_chunks.is_empty(),
            LoadingStrategy::OnDemand => false,
            LoadingStrategy::Batch => self.loaded_chunks.len() < self.batch_size,
        }
    }

    pub fn get_budget_status(&self, total_budget: i64, total_cost_budget: f64) -> BudgetStatus {
        let token_ratio = if total_budget > 0 {
            self.token_usage.total_tokens as f64 / total_budget as f64
        } else {
            0.0
        };

        let cost_ratio = if total_cost_budget > 0.0 {
            self.token_usage.estimated_cost_usd / total_cost_budget
        } else {
            0.0
        };

        let max_ratio = token_ratio.max(cost_ratio);

        if max_ratio >= 1.0 {
            BudgetStatus::Exceeded
        } else if max_ratio >= self.budget_warning_threshold {
            BudgetStatus::Warning(max_ratio)
        } else {
            BudgetStatus::WithinBudget
        }
    }

    pub fn record_token_usage(&mut self, usage: &TokenUsage) {
        self.token_usage.add(usage);
        self.last_loaded_at = chrono::Utc::now();
    }

    pub fn mark_chunk_loaded(&mut self, chunk_id: String) {
        self.loaded_chunks.push(chunk_id.clone());
        if let Some(pos) = self.pending_chunks.iter().position(|x| x == &chunk_id) {
            self.pending_chunks.remove(pos);
        }
    }

    pub fn add_pending_chunk(&mut self, chunk_id: String) {
        if !self.loaded_chunks.contains(&chunk_id) && !self.pending_chunks.contains(&chunk_id) {
            self.pending_chunks.push(chunk_id);
        }
    }

    pub fn can_enter_next_depth(&self) -> bool {
        self.current_depth < self.max_depth
    }

    pub fn enter_next_depth(&mut self) -> bool {
        if self.can_enter_next_depth() {
            self.current_depth += 1;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentChunk {
    pub chunk_id: String,
    pub content: String,
    pub chunk_type: ChunkType,
    pub priority: u8,
    pub estimated_tokens: i64,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChunkType {
    TaskDescription,
    Context,
    Examples,
    Tools,
    Constraints,
}

pub struct ProgressiveLoader {
    contexts: Arc<RwLock<HashMap<String, LoadingContext>>>,
    chunks: Arc<RwLock<HashMap<String, ContentChunk>>>,
    token_budget: i64,
    cost_budget_usd: f64,
    cost_per_1k_tokens: f64,
    storage_path: PathBuf,
}

impl ProgressiveLoader {
    pub fn new(
        token_budget: i64,
        cost_budget_usd: f64,
        cost_per_1k_tokens: f64,
        storage_path: PathBuf,
    ) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;

        let instance = Self {
            contexts: Arc::new(RwLock::new(HashMap::new())),
            chunks: Arc::new(RwLock::new(HashMap::new())),
            token_budget,
            cost_budget_usd,
            cost_per_1k_tokens,
            storage_path,
        };

        instance.load()?;

        Ok(instance)
    }

    pub fn check_budget(&self, task_id: &str) -> BudgetStatus {
        if let Some(context) = self.contexts.blocking_read().get(task_id) {
            context.get_budget_status(self.token_budget, self.cost_budget_usd)
        } else {
            BudgetStatus::WithinBudget
        }
    }

    pub async fn save(&self) -> Result<()> {
        let contexts_path = self.storage_path.join("contexts.json");
        let chunks_path = self.storage_path.join("chunks.json");

        let contexts = self.contexts.read().await.clone();
        let contexts_vec: Vec<(String, LoadingContext)> = contexts.into_iter().collect();
        std::fs::write(contexts_path, serde_json::to_string_pretty(&contexts_vec)?)?;

        let chunks = self.chunks.read().await.clone();
        let chunks_vec: Vec<(String, ContentChunk)> = chunks.into_iter().collect();
        std::fs::write(chunks_path, serde_json::to_string_pretty(&chunks_vec)?)?;

        info!("ProgressiveLoader saved to: {:?}", self.storage_path);

        Ok(())
    }

    fn load(&self) -> Result<()> {
        let contexts_path = self.storage_path.join("contexts.json");
        let chunks_path = self.storage_path.join("chunks.json");

        if contexts_path.exists() {
            let content = std::fs::read_to_string(contexts_path)?;
            let contexts_vec: Vec<(String, LoadingContext)> = serde_json::from_str(&content)?;
            let mut contexts = self.contexts.blocking_write();
            for (task_id, context) in contexts_vec {
                contexts.insert(task_id, context);
            }
        }

        if chunks_path.exists() {
            let content = std::fs::read_to_string(chunks_path)?;
            let chunks_vec: Vec<(String, ContentChunk)> = serde_json::from_str(&content)?;
            let mut chunks = self.chunks.blocking_write();
            for (chunk_id, chunk) in chunks_vec {
                chunks.insert(chunk_id, chunk);
            }
        }

        info!("ProgressiveLoader loaded from: {:?}", self.storage_path);

        Ok(())
    }

    pub async fn create_context(
        &self,
        task: &Task,
        strategy: LoadingStrategy,
        max_depth: u32,
    ) -> Result<LoadingContext> {
        info!("Creating progressive loading context for task: {}", task.id);

        let context = LoadingContext::new(task.id.clone(), strategy, max_depth);

        self.contexts
            .write()
            .await
            .insert(task.id.clone(), context.clone());

        self.save().await?;

        Ok(context)
    }

    pub async fn register_chunk(&self, chunk: ContentChunk) -> Result<()> {
        debug!("Registering content chunk: {}", chunk.chunk_id);
        self.chunks
            .write()
            .await
            .insert(chunk.chunk_id.clone(), chunk);
        self.save().await?;
        Ok(())
    }

    pub async fn get_context(&self, task_id: &str) -> Option<LoadingContext> {
        self.contexts.read().await.get(task_id).cloned()
    }

    pub async fn load_chunk(&self, task_id: &str, chunk_id: &str) -> Result<Option<ContentChunk>> {
        let contexts = self.contexts.read().await;
        let context = contexts.get(task_id);

        if context.is_none() {
            return Ok(None);
        }

        let context = context.unwrap();

        if !context.should_load_next_chunk() {
            debug!("Skipping chunk load due to strategy: {}", chunk_id);
            return Ok(None);
        }

        let budget_status = context.get_budget_status(self.token_budget, self.cost_budget_usd);
        match budget_status {
            BudgetStatus::Exceeded => {
                info!("Budget exceeded, skipping chunk load: {}", chunk_id);
                return Ok(None);
            }
            BudgetStatus::Warning(ratio) => {
                info!("Budget warning: {}% of budget used", ratio * 100.0);
            }
            BudgetStatus::WithinBudget => {}
        }

        drop(contexts);

        let chunks = self.chunks.read().await;
        let chunk = chunks.get(chunk_id).cloned();

        if let Some(chunk) = &chunk {
            let mut contexts = self.contexts.write().await;
            if let Some(context) = contexts.get_mut(task_id) {
                let mut usage = TokenUsage {
                    prompt_tokens: chunk.estimated_tokens,
                    completion_tokens: 0,
                    total_tokens: chunk.estimated_tokens,
                    estimated_cost_usd: 0.0,
                };
                usage.estimate_cost(self.cost_per_1k_tokens);
                context.record_token_usage(&usage);
                context.mark_chunk_loaded(chunk_id.to_string());
            }
            drop(contexts);
            self.save().await?;
        }

        Ok(chunk)
    }

    pub async fn load_chunks_for_depth(
        &self,
        task_id: &str,
        depth: u32,
    ) -> Result<Vec<ContentChunk>> {
        debug!("Loading chunks for depth {} of task: {}", depth, task_id);

        let mut contexts = self.contexts.write().await;
        let context = contexts.get_mut(task_id);

        if context.is_none() {
            return Ok(Vec::new());
        }

        let context = context.unwrap();

        if context.current_depth != depth {
            return Ok(Vec::new());
        }

        let pending_chunks = context.pending_chunks.clone();
        let strategy = context.loading_strategy.clone();
        let batch_size = context.batch_size;
        drop(contexts);

        let chunks = self.chunks.read().await;
        let mut loaded_chunks = Vec::new();

        for chunk_id in pending_chunks {
            if let Some(chunk) = chunks.get(&chunk_id) {
                loaded_chunks.push(chunk.clone());
            }
        }

        loaded_chunks.sort_by(|a, b| b.priority.cmp(&a.priority));

        if strategy == LoadingStrategy::Batch {
            loaded_chunks.truncate(batch_size);
        }

        let mut contexts = self.contexts.write().await;
        if let Some(context) = contexts.get_mut(task_id) {
            for chunk in &loaded_chunks {
                let mut usage = TokenUsage {
                    prompt_tokens: chunk.estimated_tokens,
                    completion_tokens: 0,
                    total_tokens: chunk.estimated_tokens,
                    estimated_cost_usd: 0.0,
                };
                usage.estimate_cost(self.cost_per_1k_tokens);
                context.record_token_usage(&usage);
                context.mark_chunk_loaded(chunk.chunk_id.clone());
            }
        }
        drop(contexts);
        self.save().await?;

        Ok(loaded_chunks)
    }

    pub async fn get_token_usage(&self, task_id: &str) -> Option<TokenUsage> {
        self.contexts
            .read()
            .await
            .get(task_id)
            .map(|ctx| ctx.token_usage.clone())
    }

    pub async fn is_within_budget(&self, task_id: &str) -> bool {
        if let Some(usage) = self.get_token_usage(task_id).await {
            usage.total_tokens <= self.token_budget
        } else {
            true
        }
    }

    pub async fn optimize_prompt(&self, task_id: &str, base_prompt: String) -> Result<String> {
        let contexts = self.contexts.read().await;
        let context = contexts.get(task_id);

        if context.is_none() {
            return Ok(base_prompt);
        }

        let context = context.unwrap();

        let mut optimized_prompt = base_prompt;

        for chunk_id in &context.loaded_chunks {
            let chunks = self.chunks.read().await;
            if let Some(chunk) = chunks.get(chunk_id) {
                optimized_prompt.push_str("\n\n");
                optimized_prompt.push_str(&chunk.content);
            }
        }

        Ok(optimized_prompt)
    }

    pub async fn get_loading_summary(&self, task_id: &str) -> Option<LoadingSummary> {
        let contexts = self.contexts.read().await;
        let context = contexts.get(task_id)?;

        Some(LoadingSummary {
            task_id: task_id.to_string(),
            loaded_chunks_count: context.loaded_chunks.len(),
            pending_chunks_count: context.pending_chunks.len(),
            current_depth: context.current_depth,
            max_depth: context.max_depth,
            total_tokens_used: context.token_usage.total_tokens,
            estimated_cost: context.token_usage.estimated_cost_usd,
            loading_strategy: context.loading_strategy.clone(),
        })
    }
}

impl Default for ProgressiveLoader {
    fn default() -> Self {
        let storage_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".aetheris")
            .join("progressive_loading");

        Self::new(100000, 10.0, 0.01, storage_path).unwrap_or_else(|_| {
            let temp_dir = tempfile::tempdir().unwrap();
            Self::new(100000, 10.0, 0.01, temp_dir.path().to_path_buf()).unwrap()
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingSummary {
    pub task_id: String,
    pub loaded_chunks_count: usize,
    pub pending_chunks_count: usize,
    pub current_depth: u32,
    pub max_depth: u32,
    pub total_tokens_used: i64,
    pub estimated_cost: f64,
    pub loading_strategy: LoadingStrategy,
}
