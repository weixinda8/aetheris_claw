use async_trait::async_trait;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DataClassification {
    Public,
    Internal,
    Confidential,
    Restricted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationTag {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub classification: DataClassification,
    pub created_at: DateTime<Utc>,
}

impl ClassificationTag {
    pub fn new(name: String, description: String, classification: DataClassification) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            classification,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub data_id: String,
    pub classification: DataClassification,
    pub confidence: f64,
    pub tags: Vec<String>,
    pub classified_by: String,
    pub classified_at: DateTime<Utc>,
    pub needs_review: bool,
}

#[async_trait]
pub trait DataClassifier: Send + Sync {
    async fn classify(
        &self,
        data: &str,
        metadata: Option<&HashMap<String, String>>,
    ) -> crate::utils::Result<ClassificationResult>;
    fn name(&self) -> &str;
}

pub struct ContentBasedClassifier {
    patterns: HashMap<DataClassification, Vec<Regex>>,
    keywords: HashMap<DataClassification, Vec<String>>,
}

impl ContentBasedClassifier {
    pub fn new() -> Self {
        let mut patterns = HashMap::new();
        let mut keywords = HashMap::new();

        patterns.insert(
            DataClassification::Restricted,
            vec![Regex::new(r"(?i)password|credit\s*card|social\s*security|ssn|passport").unwrap()],
        );

        patterns.insert(
            DataClassification::Confidential,
            vec![Regex::new(r"(?i)confidential|secret|proprietary").unwrap()],
        );

        keywords.insert(
            DataClassification::Restricted,
            vec![
                "password".to_string(),
                "credit card".to_string(),
                "ssn".to_string(),
            ],
        );

        keywords.insert(
            DataClassification::Confidential,
            vec!["confidential".to_string(), "internal use only".to_string()],
        );

        Self { patterns, keywords }
    }

    pub fn add_pattern(&mut self, classification: DataClassification, pattern: Regex) {
        self.patterns
            .entry(classification)
            .or_default()
            .push(pattern);
    }

    pub fn add_keyword(&mut self, classification: DataClassification, keyword: String) {
        self.keywords
            .entry(classification)
            .or_default()
            .push(keyword);
    }
}

impl Default for ContentBasedClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataClassifier for ContentBasedClassifier {
    async fn classify(
        &self,
        data: &str,
        _metadata: Option<&HashMap<String, String>>,
    ) -> crate::utils::Result<ClassificationResult> {
        let mut highest_classification = DataClassification::Public;
        let mut confidence = 0.0;
        let mut tags = Vec::new();

        for (classification, patterns) in &self.patterns {
            for pattern in patterns {
                if pattern.is_match(data) {
                    let priority = match classification {
                        DataClassification::Restricted => 4,
                        DataClassification::Confidential => 3,
                        DataClassification::Internal => 2,
                        DataClassification::Public => 1,
                    };

                    let current_priority = match highest_classification {
                        DataClassification::Restricted => 4,
                        DataClassification::Confidential => 3,
                        DataClassification::Internal => 2,
                        DataClassification::Public => 1,
                    };

                    if priority > current_priority {
                        highest_classification = classification.clone();
                        confidence = 0.9;
                    }
                }
            }
        }

        for keywords in self.keywords.values() {
            for keyword in keywords {
                if data.to_lowercase().contains(&keyword.to_lowercase()) {
                    tags.push(keyword.clone());
                }
            }
        }

        Ok(ClassificationResult {
            data_id: Uuid::new_v4().to_string(),
            classification: highest_classification,
            confidence,
            tags,
            classified_by: "ContentBasedClassifier".to_string(),
            classified_at: Utc::now(),
            needs_review: confidence < 0.8,
        })
    }

    fn name(&self) -> &str {
        "ContentBasedClassifier"
    }
}

pub struct MetadataBasedClassifier {
    rules: HashMap<String, DataClassification>,
    default_classification: DataClassification,
}

impl MetadataBasedClassifier {
    pub fn new() -> Self {
        let mut rules = HashMap::new();
        rules.insert("sensitivity".to_string(), DataClassification::Confidential);
        rules.insert("restricted".to_string(), DataClassification::Restricted);

        Self {
            rules,
            default_classification: DataClassification::Internal,
        }
    }

    pub fn add_rule(&mut self, metadata_key: String, classification: DataClassification) {
        self.rules.insert(metadata_key, classification);
    }

    pub fn set_default(&mut self, classification: DataClassification) {
        self.default_classification = classification;
    }
}

impl Default for MetadataBasedClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataClassifier for MetadataBasedClassifier {
    async fn classify(
        &self,
        _data: &str,
        metadata: Option<&HashMap<String, String>>,
    ) -> crate::utils::Result<ClassificationResult> {
        let mut classification = self.default_classification.clone();
        let mut confidence = 0.7;
        let mut tags = Vec::new();

        if let Some(meta) = metadata {
            for (key, value) in meta {
                if let Some(rule_class) = self.rules.get(key) {
                    classification = rule_class.clone();
                    confidence = 0.95;
                    tags.push(format!("{}:{}", key, value));
                }
            }
        }

        Ok(ClassificationResult {
            data_id: Uuid::new_v4().to_string(),
            classification,
            confidence,
            tags,
            classified_by: "MetadataBasedClassifier".to_string(),
            classified_at: Utc::now(),
            needs_review: confidence < 0.8,
        })
    }

    fn name(&self) -> &str {
        "MetadataBasedClassifier"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationStrategy {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub classifiers: Vec<String>,
    pub auto_approve_threshold: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ClassificationStrategy {
    pub fn new(name: String, description: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            classifiers: vec![
                "ContentBasedClassifier".to_string(),
                "MetadataBasedClassifier".to_string(),
            ],
            auto_approve_threshold: 0.85,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewTask {
    pub id: Uuid,
    pub data_id: String,
    pub current_classification: DataClassification,
    pub suggested_classification: DataClassification,
    pub reviewer: Option<String>,
    pub status: ReviewStatus,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReviewStatus {
    Pending,
    Approved,
    Rejected,
    Escalated,
}

pub struct ClassificationManager {
    classifiers: HashMap<String, Box<dyn DataClassifier>>,
    tags: HashMap<Uuid, ClassificationTag>,
    strategies: HashMap<Uuid, ClassificationStrategy>,
    review_tasks: HashMap<Uuid, ReviewTask>,
    classification_history: Vec<ClassificationResult>,
}

impl ClassificationManager {
    pub fn new() -> Self {
        let mut classifiers: HashMap<String, Box<dyn DataClassifier>> = HashMap::new();
        classifiers.insert(
            "ContentBasedClassifier".to_string(),
            Box::new(ContentBasedClassifier::new()),
        );
        classifiers.insert(
            "MetadataBasedClassifier".to_string(),
            Box::new(MetadataBasedClassifier::new()),
        );

        Self {
            classifiers,
            tags: HashMap::new(),
            strategies: HashMap::new(),
            review_tasks: HashMap::new(),
            classification_history: Vec::new(),
        }
    }

    pub fn register_classifier(&mut self, name: String, classifier: Box<dyn DataClassifier>) {
        self.classifiers.insert(name, classifier);
    }

    pub async fn classify(
        &self,
        data: &str,
        metadata: Option<&HashMap<String, String>>,
        strategy_id: Option<Uuid>,
    ) -> crate::utils::Result<ClassificationResult> {
        let strategy = strategy_id.and_then(|id| self.strategies.get(&id));

        let results: Vec<ClassificationResult> = if let Some(strat) = strategy {
            let mut results = Vec::new();
            for classifier_name in &strat.classifiers {
                if let Some(classifier) = self.classifiers.get(classifier_name) {
                    let result = classifier.classify(data, metadata).await?;
                    results.push(result);
                }
            }
            results
        } else {
            let mut results = Vec::new();
            for classifier in self.classifiers.values() {
                let result = classifier.classify(data, metadata).await?;
                results.push(result);
            }
            results
        };

        let best_result = results
            .into_iter()
            .max_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or_else(|| ClassificationResult {
                data_id: Uuid::new_v4().to_string(),
                classification: DataClassification::Internal,
                confidence: 0.5,
                tags: Vec::new(),
                classified_by: "Default".to_string(),
                classified_at: Utc::now(),
                needs_review: true,
            });

        Ok(best_result)
    }

    pub fn add_tag(&mut self, tag: ClassificationTag) {
        self.tags.insert(tag.id, tag);
    }

    pub fn get_tag(&self, tag_id: &Uuid) -> Option<&ClassificationTag> {
        self.tags.get(tag_id)
    }

    pub fn list_tags(&self) -> Vec<&ClassificationTag> {
        self.tags.values().collect()
    }

    pub fn add_strategy(&mut self, strategy: ClassificationStrategy) {
        self.strategies.insert(strategy.id, strategy);
    }

    pub fn get_strategy(&self, strategy_id: &Uuid) -> Option<&ClassificationStrategy> {
        self.strategies.get(strategy_id)
    }

    pub fn list_strategies(&self) -> Vec<&ClassificationStrategy> {
        self.strategies.values().collect()
    }

    pub fn create_review_task(
        &mut self,
        data_id: String,
        current_classification: DataClassification,
        suggested_classification: DataClassification,
    ) -> ReviewTask {
        let task = ReviewTask {
            id: Uuid::new_v4(),
            data_id,
            current_classification,
            suggested_classification,
            reviewer: None,
            status: ReviewStatus::Pending,
            created_at: Utc::now(),
            reviewed_at: None,
        };
        self.review_tasks.insert(task.id, task.clone());
        task
    }

    pub fn review_task(
        &mut self,
        task_id: Uuid,
        status: ReviewStatus,
        reviewer: String,
    ) -> Option<ReviewTask> {
        if let Some(task) = self.review_tasks.get_mut(&task_id) {
            task.status = status;
            task.reviewer = Some(reviewer);
            task.reviewed_at = Some(Utc::now());
            Some(task.clone())
        } else {
            None
        }
    }

    pub fn list_review_tasks(&self) -> Vec<&ReviewTask> {
        self.review_tasks.values().collect()
    }

    pub fn get_review_task(&self, task_id: &Uuid) -> Option<&ReviewTask> {
        self.review_tasks.get(task_id)
    }
}

impl Default for ClassificationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_data_classification_equality() {
        assert_eq!(DataClassification::Public, DataClassification::Public);
        assert_eq!(DataClassification::Internal, DataClassification::Internal);
        assert_eq!(
            DataClassification::Confidential,
            DataClassification::Confidential
        );
        assert_eq!(
            DataClassification::Restricted,
            DataClassification::Restricted
        );
    }

    #[test]
    fn test_classification_tag_new() {
        let tag = ClassificationTag::new(
            "Test Tag".to_string(),
            "Test Description".to_string(),
            DataClassification::Confidential,
        );
        assert_eq!(tag.name, "Test Tag");
        assert_eq!(tag.description, "Test Description");
        assert_eq!(tag.classification, DataClassification::Confidential);
    }

    #[test]
    fn test_content_based_classifier_new() {
        let classifier = ContentBasedClassifier::new();
        assert_eq!(classifier.name(), "ContentBasedClassifier");
    }

    #[test]
    fn test_content_based_classifier_default() {
        let classifier = ContentBasedClassifier::default();
        assert_eq!(classifier.name(), "ContentBasedClassifier");
    }

    #[tokio::test]
    async fn test_content_based_classifier_classify_public() {
        let classifier = ContentBasedClassifier::new();
        let result = classifier
            .classify("This is public data", None)
            .await
            .unwrap();
        assert_eq!(result.classification, DataClassification::Public);
        assert_eq!(result.classified_by, "ContentBasedClassifier");
    }

    #[tokio::test]
    async fn test_content_based_classifier_classify_restricted() {
        let classifier = ContentBasedClassifier::new();
        let result = classifier
            .classify("password is password", None)
            .await
            .unwrap();
        assert_eq!(result.classification, DataClassification::Restricted);
        assert!(!result.tags.contains(&"password".to_string()));
    }

    #[tokio::test]
    async fn test_content_based_classifier_classify_confidential() {
        let classifier = ContentBasedClassifier::new();
        let result = classifier
            .classify("This is confidential", None)
            .await
            .unwrap();
        assert_eq!(result.classification, DataClassification::Confidential);
    }

    #[test]
    fn test_metadata_based_classifier_new() {
        let classifier = MetadataBasedClassifier::new();
        assert_eq!(classifier.name(), "MetadataBasedClassifier");
    }

    #[test]
    fn test_metadata_based_classifier_default() {
        let classifier = MetadataBasedClassifier::default();
        assert_eq!(classifier.name(), "MetadataBasedClassifier");
    }

    #[tokio::test]
    async fn test_metadata_based_classifier_classify_default() {
        let classifier = MetadataBasedClassifier::new();
        let result = classifier.classify("data", None).await.unwrap();
        assert_eq!(result.classification, DataClassification::Internal);
    }

    #[tokio::test]
    async fn test_metadata_based_classifier_classify_with_metadata() {
        let classifier = MetadataBasedClassifier::new();
        let mut metadata = HashMap::new();
        metadata.insert("sensitivity".to_string(), "high".to_string());
        let result = classifier.classify("data", Some(&metadata)).await.unwrap();
        assert_eq!(result.classification, DataClassification::Confidential);
    }

    #[test]
    fn test_classification_strategy_new() {
        let strategy = ClassificationStrategy::new(
            "Test Strategy".to_string(),
            "Test Description".to_string(),
        );
        assert_eq!(strategy.name, "Test Strategy");
        assert_eq!(strategy.description, "Test Description");
        assert!(!strategy.classifiers.is_empty());
    }

    #[test]
    fn test_review_status_equality() {
        assert_eq!(ReviewStatus::Pending, ReviewStatus::Pending);
        assert_eq!(ReviewStatus::Approved, ReviewStatus::Approved);
        assert_eq!(ReviewStatus::Rejected, ReviewStatus::Rejected);
        assert_eq!(ReviewStatus::Escalated, ReviewStatus::Escalated);
    }

    #[test]
    fn test_classification_manager_new() {
        let manager = ClassificationManager::new();
        assert!(manager.list_tags().is_empty());
        assert!(manager.list_strategies().is_empty());
        assert!(manager.list_review_tasks().is_empty());
    }

    #[test]
    fn test_classification_manager_default() {
        let manager = ClassificationManager::default();
        assert!(manager.list_tags().is_empty());
    }

    #[test]
    fn test_classification_manager_add_tag() {
        let mut manager = ClassificationManager::new();
        let tag = ClassificationTag::new(
            "Test Tag".to_string(),
            "Test Description".to_string(),
            DataClassification::Confidential,
        );
        let tag_id = tag.id;
        manager.add_tag(tag);
        assert!(manager.get_tag(&tag_id).is_some());
        assert_eq!(manager.list_tags().len(), 1);
    }

    #[test]
    fn test_classification_manager_add_strategy() {
        let mut manager = ClassificationManager::new();
        let strategy = ClassificationStrategy::new(
            "Test Strategy".to_string(),
            "Test Description".to_string(),
        );
        let strategy_id = strategy.id;
        manager.add_strategy(strategy);
        assert!(manager.get_strategy(&strategy_id).is_some());
        assert_eq!(manager.list_strategies().len(), 1);
    }

    #[test]
    fn test_classification_manager_create_review_task() {
        let mut manager = ClassificationManager::new();
        let task = manager.create_review_task(
            "data-123".to_string(),
            DataClassification::Internal,
            DataClassification::Confidential,
        );
        assert_eq!(task.data_id, "data-123");
        assert_eq!(task.status, ReviewStatus::Pending);
        assert_eq!(manager.list_review_tasks().len(), 1);
    }

    #[test]
    fn test_classification_manager_review_task() {
        let mut manager = ClassificationManager::new();
        let task = manager.create_review_task(
            "data-123".to_string(),
            DataClassification::Internal,
            DataClassification::Confidential,
        );
        let task_id = task.id;

        let reviewed = manager.review_task(task_id, ReviewStatus::Approved, "reviewer".to_string());
        assert!(reviewed.is_some());
        let reviewed = reviewed.unwrap();
        assert_eq!(reviewed.status, ReviewStatus::Approved);
        assert_eq!(reviewed.reviewer, Some("reviewer".to_string()));
    }

    #[tokio::test]
    async fn test_classification_manager_classify() {
        let manager = ClassificationManager::new();
        let result = manager
            .classify("This is public data", None, None)
            .await
            .unwrap();
        assert!(!result.data_id.is_empty());
        assert!(!result.classified_by.is_empty());
    }
}
