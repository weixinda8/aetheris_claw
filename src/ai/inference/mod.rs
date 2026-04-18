pub mod cache;
pub mod cloud;
pub mod local;
pub mod metrics;
pub mod registry;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use cache::{CacheStats, InferenceCache};
pub use cloud::CloudInferenceEngine;
pub use local::LocalInferenceEngine;
pub use metrics::{InferenceMetrics, InferenceMetricsData};
pub use registry::{Model, ModelRegistry};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelFormat {
    ONNX,
    TorchScript,
    TFLite,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceInput {
    pub model_id: String,
    pub data: serde_json::Value,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceOutput {
    pub model_id: String,
    pub data: serde_json::Value,
    pub latency_ms: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

#[async_trait]
pub trait InferenceEngine: Send + Sync {
    async fn inference(&self, input: InferenceInput) -> crate::utils::Result<InferenceOutput>;
    async fn batch_inference(
        &self,
        inputs: Vec<InferenceInput>,
    ) -> crate::utils::Result<Vec<InferenceOutput>>;
}
