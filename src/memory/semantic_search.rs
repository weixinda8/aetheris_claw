use crate::memory::vector_db::{VectorDatabase, VectorDatabaseTrait};
use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SearchType {
    Vector,
    Keyword,
    Hybrid,
}

#[derive(Debug, Clone)]
pub struct SearchIndex {
    pub id: String,
    pub name: String,
    pub description: String,
    pub vector_db: VectorDatabase,
    pub keyword_index: DashMap<String, HashSet<String>>,
    pub documents: DashMap<String, IndexDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDocument {
    pub id: String,
    pub content: String,
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl IndexDocument {
    pub fn new(
        id: String,
        content: String,
        title: Option<String>,
        tags: Vec<String>,
        metadata: serde_json::Value,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            content,
            title,
            tags,
            metadata,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    pub document: IndexDocument,
    pub score: f32,
    pub search_type: SearchType,
}

pub struct SemanticSearchEngine {
    indices: Arc<DashMap<String, SearchIndex>>,
    default_index_name: String,
    vector_size: usize,
}

impl SemanticSearchEngine {
    pub fn new(default_index_name: String, vector_size: usize) -> Self {
        Self {
            indices: Arc::new(DashMap::new()),
            default_index_name,
            vector_size,
        }
    }

    pub fn create_index(&self, name: String, description: String) -> Result<()> {
        info!("Creating search index: {}", name);

        if self.indices.contains_key(&name) {
            return Err(AetherisError::Validation(format!(
                "Index already exists: {}",
                name
            )));
        }

        let vector_db = VectorDatabase::new_memory_with_config(name.clone(), self.vector_size);

        let index = SearchIndex {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.clone(),
            description,
            vector_db,
            keyword_index: DashMap::new(),
            documents: DashMap::new(),
        };

        self.indices.insert(name, index);

        Ok(())
    }

    pub fn get_or_create_default_index(&self) -> Result<SearchIndex> {
        if !self.indices.contains_key(&self.default_index_name) {
            self.create_index(
                self.default_index_name.clone(),
                "Default search index".to_string(),
            )?;
        }

        self.indices
            .get(&self.default_index_name)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| AetherisError::NotFound("Default index not found".to_string()))
    }

    pub async fn index_document(
        &self,
        index_name: Option<&str>,
        document: IndexDocument,
        vector: Option<&[f32]>,
    ) -> Result<()> {
        let index_name = index_name.unwrap_or(&self.default_index_name);

        info!(
            "Indexing document: {} into index: {}",
            document.id, index_name
        );

        let mut index = self
            .indices
            .get_mut(index_name)
            .ok_or_else(|| AetherisError::NotFound(format!("Index not found: {}", index_name)))?;

        if let Some(vector) = vector {
            if vector.len() != self.vector_size {
                return Err(AetherisError::Validation(format!(
                    "Vector size mismatch: expected {}, got {}",
                    self.vector_size,
                    vector.len()
                )));
            }
            index
                .vector_db
                .insert(&document.id, vector, document.metadata.clone())
                .await?;
        }

        self.index_keywords(&mut index, &document);

        index.documents.insert(document.id.clone(), document);

        Ok(())
    }

    fn index_keywords(&self, index: &mut SearchIndex, document: &IndexDocument) {
        let keywords = self.extract_keywords(document);

        for keyword in keywords {
            index
                .keyword_index
                .entry(keyword)
                .or_default()
                .insert(document.id.clone());
        }
    }

    fn extract_keywords(&self, document: &IndexDocument) -> Vec<String> {
        let mut keywords = Vec::new();

        let text = if let Some(title) = &document.title {
            format!("{} {}", title, document.content)
        } else {
            document.content.clone()
        };

        let words: Vec<String> = text
            .to_lowercase()
            .split_whitespace()
            .map(|word| {
                word.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string()
            })
            .filter(|word| word.len() > 2)
            .collect();

        let mut unique_words = HashSet::new();
        for word in words {
            unique_words.insert(word);
        }

        keywords.extend(unique_words);
        keywords.extend(document.tags.iter().map(|t| t.to_lowercase()));

        keywords
    }

    pub async fn search(
        &self,
        index_name: Option<&str>,
        query: &str,
        vector: Option<&[f32]>,
        search_type: SearchType,
        limit: usize,
        score_threshold: Option<f32>,
    ) -> Result<Vec<SemanticSearchResult>> {
        let index_name = index_name.unwrap_or(&self.default_index_name);

        debug!(
            "Searching index: {}, query: {}, type: {:?}",
            index_name, query, search_type
        );

        let index = self
            .indices
            .get(index_name)
            .ok_or_else(|| AetherisError::NotFound(format!("Index not found: {}", index_name)))?;

        let mut results = Vec::new();

        match search_type {
            SearchType::Vector => {
                if let Some(vector) = vector {
                    results.extend(
                        self.search_vector(&index, vector, limit, score_threshold)
                            .await?,
                    );
                } else {
                    warn!("Vector search requested but no vector provided");
                }
            }
            SearchType::Keyword => {
                results.extend(self.search_keyword(&index, query, limit));
            }
            SearchType::Hybrid => {
                let vector_results = if let Some(vector) = vector {
                    self.search_vector(&index, vector, limit, score_threshold)
                        .await?
                } else {
                    Vec::new()
                };

                let keyword_results = self.search_keyword(&index, query, limit);

                if vector_results.is_empty() && !keyword_results.is_empty() {
                    results = keyword_results;
                } else if !vector_results.is_empty() {
                    results = self.merge_results(vector_results, keyword_results, limit);
                }
            }
        }

        Ok(results)
    }

    async fn search_vector(
        &self,
        index: &SearchIndex,
        vector: &[f32],
        limit: usize,
        score_threshold: Option<f32>,
    ) -> Result<Vec<SemanticSearchResult>> {
        let search_results = if let Some(threshold) = score_threshold {
            index
                .vector_db
                .search_with_threshold(vector, limit, threshold)
                .await?
        } else {
            index.vector_db.search(vector, limit).await?
        };

        let mut results = Vec::new();

        for search_result in search_results {
            if let Some(doc) = index.documents.get(&search_result.id.to_string()) {
                results.push(SemanticSearchResult {
                    document: doc.value().clone(),
                    score: search_result.score,
                    search_type: SearchType::Vector,
                });
            }
        }

        Ok(results)
    }

    fn search_keyword(
        &self,
        index: &SearchIndex,
        query: &str,
        limit: usize,
    ) -> Vec<SemanticSearchResult> {
        let query_words: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|word| {
                word.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string()
            })
            .filter(|word| word.len() > 2)
            .collect();

        let mut document_scores: HashMap<String, f32> = HashMap::new();

        for word in query_words {
            if let Some(doc_ids) = index.keyword_index.get(&word) {
                for doc_id in doc_ids.iter() {
                    *document_scores.entry(doc_id.clone()).or_insert(0.0) += 1.0;
                }
            }
        }

        let mut sorted_docs: Vec<_> = document_scores.into_iter().collect();
        sorted_docs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut results = Vec::new();
        for (doc_id, score) in sorted_docs.into_iter().take(limit) {
            if let Some(doc) = index.documents.get(&doc_id) {
                results.push(SemanticSearchResult {
                    document: doc.value().clone(),
                    score,
                    search_type: SearchType::Keyword,
                });
            }
        }

        results
    }

    fn merge_results(
        &self,
        mut vector_results: Vec<SemanticSearchResult>,
        mut keyword_results: Vec<SemanticSearchResult>,
        limit: usize,
    ) -> Vec<SemanticSearchResult> {
        let mut seen_ids = HashSet::new();
        let mut merged = Vec::new();

        for result in vector_results.iter_mut() {
            if !seen_ids.contains(&result.document.id) {
                seen_ids.insert(result.document.id.clone());
                merged.push(result.clone());
            }
        }

        for result in keyword_results.iter_mut() {
            if !seen_ids.contains(&result.document.id) {
                seen_ids.insert(result.document.id.clone());
                merged.push(result.clone());
            }
        }

        merged.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        merged.into_iter().take(limit).collect()
    }

    pub async fn delete_document(&self, index_name: Option<&str>, doc_id: &str) -> Result<()> {
        let index_name = index_name.unwrap_or(&self.default_index_name);

        info!("Deleting document: {} from index: {}", doc_id, index_name);

        let index = self
            .indices
            .get_mut(index_name)
            .ok_or_else(|| AetherisError::NotFound(format!("Index not found: {}", index_name)))?;

        if let Some((_, document)) = index.documents.remove(doc_id) {
            let keywords = self.extract_keywords(&document);

            for keyword in keywords {
                if let Some(mut doc_ids) = index.keyword_index.get_mut(&keyword) {
                    doc_ids.remove(doc_id);
                    if doc_ids.is_empty() {
                        index.keyword_index.remove(&keyword);
                    }
                }
            }
        }

        index.vector_db.delete(doc_id).await?;

        Ok(())
    }

    pub fn get_document(&self, index_name: Option<&str>, doc_id: &str) -> Option<IndexDocument> {
        let index_name = index_name.unwrap_or(&self.default_index_name);
        self.indices
            .get(index_name)
            .and_then(|index| index.documents.get(doc_id).map(|d| d.value().clone()))
    }

    pub fn list_indices(&self) -> Vec<SearchIndex> {
        self.indices
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn index_count(&self) -> usize {
        self.indices.len()
    }

    pub fn document_count(&self, index_name: Option<&str>) -> usize {
        let index_name = index_name.unwrap_or(&self.default_index_name);
        self.indices
            .get(index_name)
            .map(|index| index.documents.len())
            .unwrap_or(0)
    }
}

impl Default for SemanticSearchEngine {
    fn default() -> Self {
        Self::new("default".to_string(), 1536)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_engine_creation() {
        let engine = SemanticSearchEngine::new("test-index".to_string(), 1536);
        assert_eq!(engine.index_count(), 0);
    }

    #[test]
    fn test_create_index() {
        let engine = SemanticSearchEngine::new("default".to_string(), 1536);

        let result = engine.create_index("test".to_string(), "Test index".to_string());
        assert!(result.is_ok());
        assert_eq!(engine.index_count(), 1);
    }

    #[tokio::test]
    async fn test_index_and_search_document() {
        let engine = SemanticSearchEngine::new("default".to_string(), 1536);

        engine
            .create_index("default".to_string(), "Default index".to_string())
            .unwrap();

        let document = IndexDocument::new(
            "doc1".to_string(),
            "This is a test document about artificial intelligence".to_string(),
            Some("Test Document".to_string()),
            vec!["AI".to_string(), "test".to_string()],
            serde_json::json!({"category": "technology"}),
        );

        engine.index_document(None, document, None).await.unwrap();

        assert_eq!(engine.document_count(None), 1);
    }

    #[test]
    fn test_extract_keywords() {
        let engine = SemanticSearchEngine::new("default".to_string(), 1536);

        let document = IndexDocument::new(
            "doc1".to_string(),
            "This is a test document about artificial intelligence".to_string(),
            Some("Test Document".to_string()),
            vec!["AI".to_string(), "test".to_string()],
            serde_json::json!({"category": "technology"}),
        );

        let keywords = engine.extract_keywords(&document);

        assert!(keywords.contains(&"test".to_string()));
        assert!(keywords.contains(&"document".to_string()));
        assert!(keywords.contains(&"about".to_string()));
        assert!(keywords.contains(&"artificial".to_string()));
        assert!(keywords.contains(&"intelligence".to_string()));
    }
}
