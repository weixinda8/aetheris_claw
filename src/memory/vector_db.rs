#![allow(async_fn_in_trait)]

use crate::utils::Result;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    CountPointsBuilder, CreateCollectionBuilder, DeletePointsBuilder, Distance, Filter, PointId,
    PointStruct, QueryPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const DEFAULT_COLLECTION_NAME: &str = "aetheris_memory";
const DEFAULT_VECTOR_SIZE: usize = 1536;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum VectorDistance {
    #[default]
    Cosine,
    DotProduct,
    Euclidean,
}


impl From<VectorDistance> for Distance {
    fn from(distance: VectorDistance) -> Self {
        match distance {
            VectorDistance::Cosine => Distance::Cosine,
            VectorDistance::DotProduct => Distance::Dot,
            VectorDistance::Euclidean => Distance::Euclid,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDocument {
    pub id: Uuid,
    pub vector: Vec<f32>,
    pub payload: serde_json::Value,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: Uuid,
    pub score: f32,
    pub payload: serde_json::Value,
}

pub trait VectorDatabaseTrait: Send + Sync {
    async fn connect(&mut self, _url: &str) -> Result<()>;
    async fn insert(&self, id: &str, vector: &[f32], payload: serde_json::Value) -> Result<()>;
    async fn insert_batch(&self, documents: Vec<VectorDocument>) -> Result<()>;
    async fn search(&self, vector: &[f32], limit: usize) -> Result<Vec<SearchResult>>;
    async fn search_with_threshold(
        &self,
        vector: &[f32],
        limit: usize,
        score_threshold: f32,
    ) -> Result<Vec<SearchResult>>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<SearchResult>>;
    async fn count(&self) -> Result<u64>;
    async fn delete_collection(&self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct InMemoryVectorDatabase {
    collection_name: String,
    vector_size: usize,
    distance: VectorDistance,
    documents: std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<Uuid, VectorDocument>>>,
}

impl InMemoryVectorDatabase {
    pub fn new() -> Self {
        Self {
            collection_name: DEFAULT_COLLECTION_NAME.to_string(),
            vector_size: DEFAULT_VECTOR_SIZE,
            distance: VectorDistance::default(),
            documents: std::sync::Arc::new(parking_lot::RwLock::new(
                std::collections::HashMap::new(),
            )),
        }
    }

    pub fn with_config(collection_name: String, vector_size: usize) -> Self {
        Self {
            collection_name,
            vector_size,
            distance: VectorDistance::default(),
            documents: std::sync::Arc::new(parking_lot::RwLock::new(
                std::collections::HashMap::new(),
            )),
        }
    }

    pub fn with_distance(mut self, distance: VectorDistance) -> Self {
        self.distance = distance;
        self
    }

    fn calculate_score(&self, a: &[f32], b: &[f32]) -> f32 {
        match self.distance {
            VectorDistance::Cosine => cosine_similarity(a, b),
            VectorDistance::DotProduct => dot_product(a, b),
            VectorDistance::Euclidean => 1.0 / (1.0 + euclidean_distance(a, b)),
        }
    }
}

impl VectorDatabaseTrait for InMemoryVectorDatabase {
    async fn connect(&mut self, _url: &str) -> Result<()> {
        Ok(())
    }

    async fn insert(&self, id: &str, vector: &[f32], payload: serde_json::Value) -> Result<()> {
        let uuid = Uuid::parse_str(id).unwrap_or_else(|_| Uuid::new_v4());
        let doc = VectorDocument {
            id: uuid,
            vector: vector.to_vec(),
            payload,
            tags: Vec::new(),
            created_at: chrono::Utc::now(),
        };
        self.documents.write().insert(uuid, doc);
        Ok(())
    }

    async fn insert_batch(&self, documents: Vec<VectorDocument>) -> Result<()> {
        let mut docs = self.documents.write();
        for doc in documents {
            docs.insert(doc.id, doc);
        }
        Ok(())
    }

    async fn search(&self, vector: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        let docs = self.documents.read();
        let mut results: Vec<(f32, &VectorDocument)> = docs
            .values()
            .map(|doc| {
                let score = self.calculate_score(vector, &doc.vector);
                (score, doc)
            })
            .collect();

        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results
            .into_iter()
            .take(limit)
            .map(|(score, doc)| SearchResult {
                id: doc.id,
                score,
                payload: doc.payload.clone(),
            })
            .collect())
    }

    async fn search_with_threshold(
        &self,
        vector: &[f32],
        limit: usize,
        score_threshold: f32,
    ) -> Result<Vec<SearchResult>> {
        let results = self.search(vector, limit).await?;
        Ok(results
            .into_iter()
            .filter(|r| r.score >= score_threshold)
            .collect())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        if let Ok(uuid) = Uuid::parse_str(id) {
            self.documents.write().remove(&uuid);
        }
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<SearchResult>> {
        if let Ok(uuid) = Uuid::parse_str(id) {
            let docs = self.documents.read();
            Ok(docs.get(&uuid).map(|doc| SearchResult {
                id: doc.id,
                score: 1.0,
                payload: doc.payload.clone(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn count(&self) -> Result<u64> {
        Ok(self.documents.read().len() as u64)
    }

    async fn delete_collection(&self) -> Result<()> {
        self.documents.write().clear();
        Ok(())
    }
}

fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot = dot_product(a, b);
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

impl Default for InMemoryVectorDatabase {
    fn default() -> Self {
        Self::new()
    }
}

pub struct QdrantVectorDatabase {
    client: Option<std::sync::Arc<Qdrant>>,
    collection_name: String,
    vector_size: usize,
    distance: VectorDistance,
}

impl std::fmt::Debug for QdrantVectorDatabase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QdrantVectorDatabase")
            .field("client", &self.client.as_ref().map(|_| "QdrantClient"))
            .field("collection_name", &self.collection_name)
            .field("vector_size", &self.vector_size)
            .field("distance", &self.distance)
            .finish()
    }
}

impl Clone for QdrantVectorDatabase {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            collection_name: self.collection_name.clone(),
            vector_size: self.vector_size,
            distance: self.distance,
        }
    }
}

impl QdrantVectorDatabase {
    pub fn new() -> Self {
        Self {
            client: None,
            collection_name: DEFAULT_COLLECTION_NAME.to_string(),
            vector_size: DEFAULT_VECTOR_SIZE,
            distance: VectorDistance::default(),
        }
    }

    pub fn with_config(collection_name: String, vector_size: usize) -> Self {
        Self {
            client: None,
            collection_name,
            vector_size,
            distance: VectorDistance::default(),
        }
    }

    pub fn with_distance(mut self, distance: VectorDistance) -> Self {
        self.distance = distance;
        self
    }

    async fn ensure_collection_exists(&self) -> Result<()> {
        if let Some(client) = &self.client {
            let exists = client.collection_exists(&self.collection_name).await?;
            if !exists {
                client
                    .create_collection(
                        CreateCollectionBuilder::new(&self.collection_name).vectors_config(
                            VectorParamsBuilder::new(self.vector_size as u64, self.distance.into()),
                        ),
                    )
                    .await?;
            }
        }
        Ok(())
    }
}

fn point_id_to_uuid(point_id: Option<PointId>) -> Option<Uuid> {
    point_id.and_then(|id| {
        if let Some(options) = id.point_id_options {
            match options {
                qdrant_client::qdrant::point_id::PointIdOptions::Uuid(uuid_str) => {
                    Uuid::parse_str(&uuid_str).ok()
                }
                qdrant_client::qdrant::point_id::PointIdOptions::Num(num) => {
                    Some(Uuid::from_u64_pair(0, num))
                }
            }
        } else {
            None
        }
    })
}

impl VectorDatabaseTrait for QdrantVectorDatabase {
    async fn connect(&mut self, url: &str) -> Result<()> {
        let client = Qdrant::from_url(url).build()?;
        self.client = Some(std::sync::Arc::new(client));
        self.ensure_collection_exists().await?;
        Ok(())
    }

    async fn insert(&self, id: &str, vector: &[f32], payload: serde_json::Value) -> Result<()> {
        if let Some(client) = &self.client {
            let payload_map: std::collections::HashMap<String, serde_json::Value> = match payload {
                serde_json::Value::Object(obj) => obj.into_iter().collect(),
                _ => {
                    let mut map = std::collections::HashMap::new();
                    map.insert("value".to_string(), payload);
                    map
                }
            };
            let point = PointStruct::new(id.to_string(), vector.to_vec(), payload_map);
            client
                .upsert_points(UpsertPointsBuilder::new(&self.collection_name, vec![point]))
                .await?;
        }
        Ok(())
    }

    async fn insert_batch(&self, documents: Vec<VectorDocument>) -> Result<()> {
        if let Some(client) = &self.client {
            let points: Vec<PointStruct> = documents
                .into_iter()
                .map(|doc| {
                    let payload_map: std::collections::HashMap<String, serde_json::Value> =
                        match doc.payload {
                            serde_json::Value::Object(obj) => obj.into_iter().collect(),
                            _ => {
                                let mut map = std::collections::HashMap::new();
                                map.insert("value".to_string(), doc.payload);
                                map
                            }
                        };
                    PointStruct::new(doc.id.to_string(), doc.vector, payload_map)
                })
                .collect();
            client
                .upsert_points(UpsertPointsBuilder::new(&self.collection_name, points))
                .await?;
        }
        Ok(())
    }

    async fn search(&self, vector: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        if let Some(client) = &self.client {
            let query = QueryPointsBuilder::new(&self.collection_name)
                .query(vector.to_vec())
                .limit(limit as u64)
                .with_payload(true);
            let response = client.query(query).await?;
            let results: Vec<SearchResult> = response
                .result
                .into_iter()
                .filter_map(|scored_point| {
                    let id = point_id_to_uuid(scored_point.id)?;
                    let payload = serde_json::to_value(scored_point.payload)
                        .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));
                    Some(SearchResult {
                        id,
                        score: scored_point.score,
                        payload,
                    })
                })
                .collect();
            Ok(results)
        } else {
            Ok(Vec::new())
        }
    }

    async fn search_with_threshold(
        &self,
        vector: &[f32],
        limit: usize,
        score_threshold: f32,
    ) -> Result<Vec<SearchResult>> {
        if let Some(client) = &self.client {
            let query = QueryPointsBuilder::new(&self.collection_name)
                .query(vector.to_vec())
                .limit(limit as u64)
                .with_payload(true)
                .score_threshold(score_threshold);
            let response = client.query(query).await?;
            let results: Vec<SearchResult> = response
                .result
                .into_iter()
                .filter_map(|scored_point| {
                    let id = point_id_to_uuid(scored_point.id)?;
                    let payload = serde_json::to_value(scored_point.payload)
                        .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));
                    Some(SearchResult {
                        id,
                        score: scored_point.score,
                        payload,
                    })
                })
                .collect();
            Ok(results)
        } else {
            Ok(Vec::new())
        }
    }

    async fn delete(&self, id: &str) -> Result<()> {
        if let Some(client) = &self.client {
            let filter = Filter::must([qdrant_client::qdrant::Condition::has_id([id.to_string()])]);
            let delete_builder = DeletePointsBuilder::new(&self.collection_name).points(filter);
            client.delete_points(delete_builder).await?;
        }
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<SearchResult>> {
        if let Some(client) = &self.client {
            let filter = Filter::must([qdrant_client::qdrant::Condition::has_id([id.to_string()])]);
            let query = QueryPointsBuilder::new(&self.collection_name)
                .query(vec![0.0; self.vector_size])
                .limit(1)
                .with_payload(true)
                .filter(filter);
            let response = client.query(query).await?;
            Ok(response.result.into_iter().next().map(|scored_point| {
                let id = point_id_to_uuid(scored_point.id).unwrap_or_else(Uuid::new_v4);
                let payload = serde_json::to_value(scored_point.payload)
                    .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));
                SearchResult {
                    id,
                    score: 1.0,
                    payload,
                }
            }))
        } else {
            Ok(None)
        }
    }

    async fn count(&self) -> Result<u64> {
        if let Some(client) = &self.client {
            let count = client
                .count(CountPointsBuilder::new(&self.collection_name))
                .await?;
            Ok(count.result.map(|r| r.count).unwrap_or(0))
        } else {
            Ok(0)
        }
    }

    async fn delete_collection(&self) -> Result<()> {
        if let Some(client) = &self.client {
            client.delete_collection(&self.collection_name).await?;
        }
        Ok(())
    }
}

impl Default for QdrantVectorDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum VectorDatabaseBackend {
    Memory,
    Qdrant,
}

#[derive(Debug, Clone)]
pub enum VectorDatabase {
    Memory(InMemoryVectorDatabase),
    Qdrant(QdrantVectorDatabase),
}

impl VectorDatabase {
    pub fn new() -> Self {
        Self::new_memory()
    }

    pub fn new_memory() -> Self {
        Self::Memory(InMemoryVectorDatabase::new())
    }

    pub fn new_memory_with_config(collection_name: String, vector_size: usize) -> Self {
        Self::Memory(InMemoryVectorDatabase::with_config(
            collection_name,
            vector_size,
        ))
    }

    pub fn new_memory_with_distance(
        collection_name: String,
        vector_size: usize,
        distance: VectorDistance,
    ) -> Self {
        Self::Memory(
            InMemoryVectorDatabase::with_config(collection_name, vector_size)
                .with_distance(distance),
        )
    }

    pub fn new_qdrant() -> Self {
        Self::Qdrant(QdrantVectorDatabase::new())
    }

    pub fn new_qdrant_with_config(collection_name: String, vector_size: usize) -> Self {
        Self::Qdrant(QdrantVectorDatabase::with_config(
            collection_name,
            vector_size,
        ))
    }

    pub fn new_qdrant_with_distance(
        collection_name: String,
        vector_size: usize,
        distance: VectorDistance,
    ) -> Self {
        Self::Qdrant(
            QdrantVectorDatabase::with_config(collection_name, vector_size).with_distance(distance),
        )
    }

    pub fn with_config(collection_name: String, vector_size: usize) -> Self {
        Self::new_memory_with_config(collection_name, vector_size)
    }

    pub fn with_distance(mut self, distance: VectorDistance) -> Self {
        match &mut self {
            VectorDatabase::Memory(db) => {
                *db = std::mem::take(db).with_distance(distance);
            }
            VectorDatabase::Qdrant(db) => {
                *db = std::mem::take(db).with_distance(distance);
            }
        }
        self
    }

    pub fn collection_name(&self) -> &str {
        match self {
            VectorDatabase::Memory(db) => &db.collection_name,
            VectorDatabase::Qdrant(db) => &db.collection_name,
        }
    }

    pub fn vector_size(&self) -> usize {
        match self {
            VectorDatabase::Memory(db) => db.vector_size,
            VectorDatabase::Qdrant(db) => db.vector_size,
        }
    }

    pub fn distance(&self) -> VectorDistance {
        match self {
            VectorDatabase::Memory(db) => db.distance,
            VectorDatabase::Qdrant(db) => db.distance,
        }
    }
}

impl Default for VectorDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl VectorDatabaseTrait for VectorDatabase {
    async fn connect(&mut self, url: &str) -> Result<()> {
        match self {
            VectorDatabase::Memory(db) => db.connect(url).await,
            VectorDatabase::Qdrant(db) => db.connect(url).await,
        }
    }

    async fn insert(&self, id: &str, vector: &[f32], payload: serde_json::Value) -> Result<()> {
        match self {
            VectorDatabase::Memory(db) => db.insert(id, vector, payload).await,
            VectorDatabase::Qdrant(db) => db.insert(id, vector, payload).await,
        }
    }

    async fn insert_batch(&self, documents: Vec<VectorDocument>) -> Result<()> {
        match self {
            VectorDatabase::Memory(db) => db.insert_batch(documents).await,
            VectorDatabase::Qdrant(db) => db.insert_batch(documents).await,
        }
    }

    async fn search(&self, vector: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        match self {
            VectorDatabase::Memory(db) => db.search(vector, limit).await,
            VectorDatabase::Qdrant(db) => db.search(vector, limit).await,
        }
    }

    async fn search_with_threshold(
        &self,
        vector: &[f32],
        limit: usize,
        score_threshold: f32,
    ) -> Result<Vec<SearchResult>> {
        match self {
            VectorDatabase::Memory(db) => {
                db.search_with_threshold(vector, limit, score_threshold)
                    .await
            }
            VectorDatabase::Qdrant(db) => {
                db.search_with_threshold(vector, limit, score_threshold)
                    .await
            }
        }
    }

    async fn delete(&self, id: &str) -> Result<()> {
        match self {
            VectorDatabase::Memory(db) => db.delete(id).await,
            VectorDatabase::Qdrant(db) => db.delete(id).await,
        }
    }

    async fn get(&self, id: &str) -> Result<Option<SearchResult>> {
        match self {
            VectorDatabase::Memory(db) => db.get(id).await,
            VectorDatabase::Qdrant(db) => db.get(id).await,
        }
    }

    async fn count(&self) -> Result<u64> {
        match self {
            VectorDatabase::Memory(db) => db.count().await,
            VectorDatabase::Qdrant(db) => db.count().await,
        }
    }

    async fn delete_collection(&self) -> Result<()> {
        match self {
            VectorDatabase::Memory(db) => db.delete_collection().await,
            VectorDatabase::Qdrant(db) => db.delete_collection().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vector_db_creation() {
        let db = VectorDatabase::new();
        assert_eq!(db.collection_name(), DEFAULT_COLLECTION_NAME);
        assert_eq!(db.vector_size(), DEFAULT_VECTOR_SIZE);
    }

    #[tokio::test]
    async fn test_vector_db_with_config() {
        let db = VectorDatabase::with_config("test_collection".to_string(), 768);
        assert_eq!(db.collection_name(), "test_collection");
        assert_eq!(db.vector_size(), 768);
    }

    #[tokio::test]
    async fn test_insert_and_search() {
        let db = VectorDatabase::new();
        let id = Uuid::new_v4();
        let vector = vec![0.1, 0.2, 0.3];
        let payload = serde_json::json!({"test": "data"});

        db.insert(&id.to_string(), &vector, payload.clone())
            .await
            .unwrap();

        let results = db.search(&vector, 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
    }
}
