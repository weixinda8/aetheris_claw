pub mod aggregation;
pub mod compression;
pub mod config;
pub mod filtering;
pub mod lineage_integration;
pub mod pipeline;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdgeData {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub stream_id: String,
    pub values: HashMap<String, f64>,
    pub metadata: Option<serde_json::Value>,
}

impl EdgeData {
    pub fn new(stream_id: String, values: HashMap<String, f64>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            stream_id,
            values,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FilterStrategy {
    None,
    Aggregate,
    Compress,
    AggregateAndCompress,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompressionLevel {
    Low,
    Medium,
    High,
    Lossless,
}

#[async_trait]
pub trait DataFilter: Send + Sync {
    fn name(&self) -> &str;
    async fn filter(&mut self, data: EdgeData) -> crate::utils::Result<Vec<EdgeData>>;
    async fn batch_filter(&mut self, data: Vec<EdgeData>) -> crate::utils::Result<Vec<EdgeData>> {
        let mut results = Vec::with_capacity(data.len());
        for item in data {
            let filtered = self.filter(item).await?;
            results.extend(filtered);
        }
        Ok(results)
    }
}

pub use aggregation::{AggregationFunction, DataAggregator, TimeWindow, WindowType};
pub use compression::DataCompressor;
pub use config::{FilterConfig, StreamConfig};
pub use filtering::{OutlierDetectionMethod, OutlierDetector};
pub use pipeline::EdgeFilterPipeline;
