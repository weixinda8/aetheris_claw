use crate::streaming::traits::*;
use crate::streaming::types::*;
use crate::utils::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct InMemoryStateBackend {
    state: Arc<DashMap<String, Vec<u8>>>,
    offsets: Arc<DashMap<String, u64>>,
}

impl InMemoryStateBackend {
    pub fn new() -> Self {
        Self {
            state: Arc::new(DashMap::new()),
            offsets: Arc::new(DashMap::new()),
        }
    }
}

impl Default for InMemoryStateBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StateBackend for InMemoryStateBackend {
    async fn get(&self, key: String) -> Result<Option<Vec<u8>>> {
        Ok(self.state.get(&key).map(|entry| entry.value().clone()))
    }

    async fn put(&mut self, key: String, value: Vec<u8>) -> Result<()> {
        self.state.insert(key, value);
        Ok(())
    }

    async fn delete(&mut self, key: String) -> Result<()> {
        self.state.remove(&key);
        Ok(())
    }

    async fn save_checkpoint(&self) -> Result<Checkpoint> {
        let mut state_map = HashMap::new();
        for entry in self.state.iter() {
            state_map.insert(entry.key().clone(), entry.value().clone());
        }

        let mut offsets_map = HashMap::new();
        for entry in self.offsets.iter() {
            offsets_map.insert(entry.key().clone(), *entry.value());
        }

        Ok(Checkpoint {
            checkpoint_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            state: state_map,
            offsets: offsets_map,
        })
    }

    async fn load_checkpoint(&mut self, checkpoint: Checkpoint) -> Result<()> {
        self.state.clear();
        for (key, value) in checkpoint.state {
            self.state.insert(key, value);
        }

        self.offsets.clear();
        for (key, value) in checkpoint.offsets {
            self.offsets.insert(key, value);
        }

        Ok(())
    }

    async fn get_key_value_state(&self, name: &str) -> Result<KeyValueState<String, String>> {
        let backend: Arc<RwLock<dyn StateBackend + Send + Sync>> =
            Arc::new(RwLock::new(self.clone()));
        Ok(KeyValueState::new(backend, name.to_string()))
    }
}

impl Clone for InMemoryStateBackend {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            offsets: self.offsets.clone(),
        }
    }
}

pub struct KeyValueState<K, V> {
    backend: Arc<RwLock<dyn StateBackend + Send + Sync>>,
    name: String,
    _phantom: std::marker::PhantomData<(K, V)>,
}

impl<K, V> KeyValueState<K, V> {
    pub fn new(backend: Arc<RwLock<dyn StateBackend + Send + Sync>>, name: String) -> Self {
        Self {
            backend,
            name,
            _phantom: std::marker::PhantomData,
        }
    }

    fn make_key(&self, key: &K) -> Result<String>
    where
        K: serde::Serialize,
    {
        Ok(format!("{}:{}", self.name, serde_json::to_string(key)?))
    }
}

impl<K, V> KeyValueState<K, V>
where
    K: serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
    V: serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
{
    pub async fn get(&self, key: &K) -> Result<Option<V>> {
        let composite_key = self.make_key(key)?;
        let backend = self.backend.read().await;
        if let Some(value_bytes) = backend.get(composite_key).await? {
            let value: V = bincode::deserialize(&value_bytes)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    pub async fn put(&self, key: &K, value: &V) -> Result<()> {
        let composite_key = self.make_key(key)?;
        let value_bytes = bincode::serialize(value)?;
        let mut backend = self.backend.write().await;
        backend.put(composite_key, value_bytes).await
    }

    pub async fn delete(&self, key: &K) -> Result<()> {
        let composite_key = self.make_key(key)?;
        let mut backend = self.backend.write().await;
        backend.delete(composite_key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_state_backend_new() {
        let backend = InMemoryStateBackend::new();
        assert!(backend.state.is_empty());
        assert!(backend.offsets.is_empty());
    }

    #[test]
    fn test_in_memory_state_backend_default() {
        let backend = InMemoryStateBackend::default();
        assert!(backend.state.is_empty());
        assert!(backend.offsets.is_empty());
    }

    #[tokio::test]
    async fn test_in_memory_state_backend_get_put_delete() {
        let mut backend = InMemoryStateBackend::new();
        
        let key = "test_key".to_string();
        let value = b"test_value".to_vec();
        backend.put(key.clone(), value.clone()).await.unwrap();
        
        let retrieved = backend.get(key.clone()).await.unwrap();
        assert_eq!(retrieved, Some(value));
        
        backend.delete(key.clone()).await.unwrap();
        let deleted = backend.get(key).await.unwrap();
        assert_eq!(deleted, None);
    }

    #[tokio::test]
    async fn test_in_memory_state_backend_checkpoint() {
        let mut backend = InMemoryStateBackend::new();
        
        backend.put("key1".to_string(), b"value1".to_vec()).await.unwrap();
        backend.put("key2".to_string(), b"value2".to_vec()).await.unwrap();
        
        let checkpoint = backend.save_checkpoint().await.unwrap();
        assert!(!checkpoint.checkpoint_id.is_empty());
        assert_eq!(checkpoint.state.len(), 2);
        
        backend.state.clear();
        
        backend.load_checkpoint(checkpoint).await.unwrap();
        
        let value1 = backend.get("key1".to_string()).await.unwrap();
        assert_eq!(value1, Some(b"value1".to_vec()));
        let value2 = backend.get("key2".to_string()).await.unwrap();
        assert_eq!(value2, Some(b"value2".to_vec()));
    }

    #[tokio::test]
    async fn test_key_value_state() {
        let backend = InMemoryStateBackend::new();
        let state = backend.get_key_value_state("test").await.unwrap();
        
        state.put(&"key".to_string(), &"value".to_string()).await.unwrap();
        
        let retrieved = state.get(&"key".to_string()).await.unwrap();
        assert_eq!(retrieved, Some("value".to_string()));
        
        state.delete(&"key".to_string()).await.unwrap();
        let deleted = state.get(&"key".to_string()).await.unwrap();
        assert_eq!(deleted, None);
    }
}
