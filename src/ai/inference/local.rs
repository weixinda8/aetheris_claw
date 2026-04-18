use async_trait::async_trait;
use tracing::info;

use super::{InferenceEngine, InferenceInput, InferenceOutput};

pub struct LocalInferenceEngine;

impl LocalInferenceEngine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalInferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InferenceEngine for LocalInferenceEngine {
    async fn inference(&self, input: InferenceInput) -> crate::utils::Result<InferenceOutput> {
        let start_time = std::time::Instant::now();
        info!("Starting local inference for model: {}", input.model_id);

        let output_data = serde_json::json!({
            "model_id": input.model_id.clone(),
            "input": input.data,
            "result": "simulated_local_inference_result",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let elapsed = start_time.elapsed();
        let latency_ms = elapsed.as_millis() as u64;

        info!(
            "Local inference completed for model: {} in {}ms",
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
        info!("Starting local batch inference for {} models", inputs.len());
        let mut outputs = Vec::with_capacity(inputs.len());

        for input in inputs {
            let output = self.inference(input).await?;
            outputs.push(output);
        }

        info!("Local batch inference completed");
        Ok(outputs)
    }
}
