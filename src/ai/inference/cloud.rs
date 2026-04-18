use async_trait::async_trait;
use tracing::info;

use super::{InferenceEngine, InferenceInput, InferenceOutput};

pub struct CloudInferenceEngine {
    base_url: String,
    api_key: Option<String>,
}

impl CloudInferenceEngine {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self { base_url, api_key }
    }
}

impl Default for CloudInferenceEngine {
    fn default() -> Self {
        Self::new("https://api.example.com".to_string(), None)
    }
}

#[async_trait]
impl InferenceEngine for CloudInferenceEngine {
    async fn inference(&self, input: InferenceInput) -> crate::utils::Result<InferenceOutput> {
        let start_time = std::time::Instant::now();
        info!("Starting cloud inference for model: {}", input.model_id);

        let output_data = serde_json::json!({
            "model_id": input.model_id.clone(),
            "input": input.data,
            "result": "simulated_cloud_inference_result",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "engine": "cloud",
            "base_url": self.base_url,
        });

        let elapsed = start_time.elapsed();
        let latency_ms = elapsed.as_millis() as u64;

        info!(
            "Cloud inference completed for model: {} in {}ms",
            input.model_id, latency_ms
        );

        Ok(InferenceOutput {
            model_id: input.model_id.clone(),
            data: output_data,
            latency_ms,
            success: true,
            error_message: None,
        })
    }

    async fn batch_inference(
        &self,
        inputs: Vec<InferenceInput>,
    ) -> crate::utils::Result<Vec<InferenceOutput>> {
        info!("Starting cloud batch inference for {} models", inputs.len());
        let mut outputs = Vec::with_capacity(inputs.len());

        for input in inputs {
            let output = self.inference(input).await?;
            outputs.push(output);
        }

        info!("Cloud batch inference completed");
        Ok(outputs)
    }
}
