pub mod deep_learning;
pub mod features;
pub mod ml;
pub mod online_learning;
pub mod statistical;
pub mod visualization;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Anomaly {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub score: f64,
    pub is_anomaly: bool,
    pub feature_values: HashMap<String, f64>,
    pub method: AnomalyDetectionMethod,
    pub description: Option<String>,
}

impl Anomaly {
    pub fn new(
        score: f64,
        is_anomaly: bool,
        feature_values: HashMap<String, f64>,
        method: AnomalyDetectionMethod,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            score,
            is_anomaly,
            feature_values,
            method,
            description: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AnomalyDetectionMethod {
    Statistical3Sigma,
    StatisticalIQR,
    IsolationForest,
    LOF,
    Autoencoder,
}

#[async_trait::async_trait]
pub trait AnomalyDetector: Send + Sync {
    fn name(&self) -> &str;
    fn method(&self) -> AnomalyDetectionMethod;
    async fn detect(&mut self, features: &HashMap<String, f64>) -> crate::utils::Result<Anomaly>;
    async fn fit(&mut self, data: &[HashMap<String, f64>]) -> crate::utils::Result<()>;
    fn is_fitted(&self) -> bool;
}

pub use deep_learning::AutoencoderDetector;
pub use features::{FeatureExtractor, StreamingFeatureExtractor};
pub use ml::{IsolationForestDetector, LOFDetector};
pub use online_learning::{DriftDetector, OnlineLearner};
pub use statistical::{Statistical3SigmaDetector, StatisticalIQRDetector};
pub use visualization::AnomalyVisualizationData;
