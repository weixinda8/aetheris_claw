use crate::memory::MemoryItem;
use crate::utils::Result;
use dashmap::DashMap;
use hashlink::LinkedHashMap;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionContext {
    pub id: Uuid,
    pub session_id: Uuid,
    pub task_id: Option<Uuid>,
    pub variables: serde_json::Map<String, serde_json::Value>,
    pub call_stack: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            task_id: None,
            variables: serde_json::Map::new(),
            call_stack: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

pub struct LruCache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
{
    map: LinkedHashMap<K, V>,
    capacity: usize,
}

impl<K, V> LruCache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            map: LinkedHashMap::new(),
            capacity,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            self.map.to_back(key);
        }
        self.map.get(key)
    }

    pub fn insert(&mut self, key: K, value: V) {
        if self.map.contains_key(&key) {
            self.map.to_back(&key);
            self.map.insert(key, value);
        } else {
            if self.map.len() >= self.capacity {
                self.map.pop_front();
            }
            self.map.insert(key, value);
        }
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.map.remove(key)
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

pub struct ShortTermMemory {
    items: DashMap<String, MemoryItem>,
    tasks: DashMap<String, crate::core::Task>,
    contexts: DashMap<String, Arc<RwLock<ExecutionContext>>>,
    recent_items: RwLock<LruCache<String, MemoryItem>>,
    message_queue: RwLock<VecDeque<MemoryItem>>,
    max_queue_size: usize,
}

impl ShortTermMemory {
    pub fn new() -> Self {
        Self::with_config(100)
    }

    pub fn with_config(max_queue_size: usize) -> Self {
        Self {
            items: DashMap::new(),
            tasks: DashMap::new(),
            contexts: DashMap::new(),
            recent_items: RwLock::new(LruCache::new(1000)),
            message_queue: RwLock::new(VecDeque::with_capacity(max_queue_size)),
            max_queue_size,
        }
    }

    pub async fn store(&self, item: MemoryItem) -> Result<()> {
        self.items.insert(item.id.clone(), item.clone());
        self.recent_items
            .write()
            .insert(item.id.clone(), item.clone());

        let mut queue = self.message_queue.write();
        queue.push_back(item);
        if queue.len() > self.max_queue_size {
            queue.pop_front();
        }

        Ok(())
    }

    pub async fn retrieve(&self, query: &str) -> Result<Vec<MemoryItem>> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        for item in self.items.iter() {
            let content_str = item.content.to_string().to_lowercase();
            let tags_match = item
                .tags
                .iter()
                .any(|tag| tag.to_lowercase().contains(&query_lower));

            if content_str.contains(&query_lower) || tags_match {
                results.push(item.clone());
            }
        }

        Ok(results)
    }

    pub async fn get(&self, id: &str) -> Result<Option<MemoryItem>> {
        Ok(self.items.get(id).map(|item| item.clone()))
    }

    pub async fn get_recent(&self, limit: usize) -> Result<Vec<MemoryItem>> {
        let recent = self.recent_items.read();
        Ok(recent
            .map
            .iter()
            .map(|(_, v)| v.clone())
            .take(limit)
            .collect())
    }

    pub async fn get_queue(&self) -> Result<Vec<MemoryItem>> {
        Ok(self.message_queue.read().iter().cloned().collect())
    }

    pub async fn clear(&self) {
        self.items.clear();
        self.recent_items.write().clear();
        self.message_queue.write().clear();
    }

    pub async fn create_context(&self, session_id: Uuid) -> Result<Uuid> {
        let context = ExecutionContext {
            session_id,
            ..Default::default()
        };
        let context_id = context.id;
        self.contexts
            .insert(context_id.to_string(), Arc::new(RwLock::new(context)));
        Ok(context_id)
    }

    pub async fn get_context(
        &self,
        context_id: Uuid,
    ) -> Result<Option<Arc<RwLock<ExecutionContext>>>> {
        Ok(self.contexts.get(&context_id.to_string()).map(|ctx| ctx.clone()))
    }

    pub async fn set_context_variable(
        &self,
        context_id: Uuid,
        key: String,
        value: serde_json::Value,
    ) -> Result<()> {
        if let Some(ctx) = self.contexts.get(&context_id.to_string()) {
            let mut context = ctx.write();
            context.variables.insert(key, value);
            context.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    pub async fn get_context_variable(
        &self,
        context_id: Uuid,
        key: &str,
    ) -> Result<Option<serde_json::Value>> {
        if let Some(ctx) = self.contexts.get(&context_id.to_string()) {
            let context = ctx.read();
            Ok(context.variables.get(key).cloned())
        } else {
            Ok(None)
        }
    }

    pub async fn push_call_frame(&self, context_id: Uuid, frame: String) -> Result<()> {
        if let Some(ctx) = self.contexts.get(&context_id.to_string()) {
            let mut context = ctx.write();
            context.call_stack.push(frame);
            context.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    pub async fn pop_call_frame(&self, context_id: Uuid) -> Result<Option<String>> {
        if let Some(ctx) = self.contexts.get(&context_id.to_string()) {
            let mut context = ctx.write();
            let frame = context.call_stack.pop();
            context.updated_at = chrono::Utc::now();
            Ok(frame)
        } else {
            Ok(None)
        }
    }

    pub async fn remove_context(&self, context_id: Uuid) -> Result<()> {
        self.contexts.remove(&context_id.to_string());
        Ok(())
    }

    pub async fn len(&self) -> usize {
        self.items.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get_task(&self, task_id: &str) -> Option<crate::core::Task> {
        if let Some(task) = self.tasks.get(task_id) {
            Some(task.value().clone())
        } else {
            self.items
                .get(task_id)
                .and_then(|item| serde_json::from_value::<crate::core::Task>(item.content.clone()).ok())
                .and_then(|task| {
                    // 缓存任务以提高后续查询性能
                    let _ = self.tasks.insert(task_id.to_string(), task.clone());
                    Some(task)
                })
        }
    }

    pub fn get_all_tasks(&self) -> Vec<crate::core::Task> {
        if !self.tasks.is_empty() {
            // 优先从缓存中获取任务
            self.tasks.iter().map(|task| task.value().clone()).collect()
        } else {
            // 如果缓存为空，从 items 中反序列化并缓存
            let tasks: Vec<_> = self.items
                .iter()
                .filter_map(|item| {
                    serde_json::from_value::<crate::core::Task>(item.content.clone()).ok()
                })
                .collect();
            
            // 缓存任务以提高后续查询性能
            for task in &tasks {
                let _ = self.tasks.insert(task.id.clone(), task.clone());
            }
            
            tasks
        }
    }

    pub fn store_task(&self, task: crate::core::Task) {
        let item = MemoryItem {
            id: task.id.clone(),
            content: serde_json::to_value(&task).unwrap_or(serde_json::Value::Null),
            timestamp: task.updated_at,
            tags: task.tags.clone(),
            importance: 0.5,
        };
        let _ = self.items.insert(task.id.clone(), item);
        let _ = self.tasks.insert(task.id.clone(), task);
    }
}

impl Default for ShortTermMemory {
    fn default() -> Self {
        Self::new()
    }
}
