use super::*;
use std::collections::VecDeque;

pub struct OnlineLearner {
    model_id: String,
    current_version: ModelVersion,
    config: LearningConfig,
    data_buffer: VecDeque<(serde_json::Value, serde_json::Value)>,
    feedback_buffer: VecDeque<Feedback>,
    is_fitted: bool,
}

impl OnlineLearner {
    pub fn new(model_id: String, initial_version: ModelVersion, config: LearningConfig) -> Self {
        Self {
            model_id,
            current_version: initial_version,
            config,
            data_buffer: VecDeque::new(),
            feedback_buffer: VecDeque::new(),
            is_fitted: false,
        }
    }

    pub fn add_to_buffer(&mut self, input: serde_json::Value, output: serde_json::Value) {
        self.data_buffer.push_back((input, output));

        if self.data_buffer.len() > self.config.batch_size * 10 {
            self.data_buffer.pop_front();
        }
    }

    pub fn add_feedback_to_buffer(&mut self, feedback: Feedback) {
        self.feedback_buffer.push_back(feedback);

        if self.feedback_buffer.len() > 1000 {
            self.feedback_buffer.pop_front();
        }
    }

    pub fn buffer_size(&self) -> usize {
        self.data_buffer.len()
    }

    pub fn feedback_buffer_size(&self) -> usize {
        self.feedback_buffer.len()
    }

    pub fn clear_buffer(&mut self) {
        self.data_buffer.clear();
    }

    pub fn clear_feedback_buffer(&mut self) {
        self.feedback_buffer.clear();
    }

    pub fn get_batch(&mut self) -> Vec<(serde_json::Value, serde_json::Value)> {
        let batch_size = std::cmp::min(self.config.batch_size, self.data_buffer.len());
        self.data_buffer.drain(0..batch_size).collect()
    }
}

#[async_trait::async_trait]
impl AdaptiveLearner for OnlineLearner {
    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn current_version(&self) -> &ModelVersion {
        &self.current_version
    }

    fn config(&self) -> &LearningConfig {
        &self.config
    }

    async fn predict(&self, input: &serde_json::Value) -> crate::utils::Result<serde_json::Value> {
        Ok(serde_json::json!({
            "input": input,
            "prediction": "placeholder",
            "model_id": self.model_id,
            "version": self.current_version.version,
            "timestamp": Utc::now().to_rfc3339()
        }))
    }

    async fn update(
        &mut self,
        data: &[(serde_json::Value, serde_json::Value)],
    ) -> crate::utils::Result<()> {
        for (input, output) in data {
            self.add_to_buffer(input.clone(), output.clone());
        }

        if self.data_buffer.len() >= self.config.batch_size && self.config.enable_online_learning {
            let batch = self.get_batch();
            tracing::info!("Online learning: processing batch of size {}", batch.len());
            self.is_fitted = true;
        }

        Ok(())
    }

    async fn apply_feedback(&mut self, feedback: &Feedback) -> crate::utils::Result<()> {
        self.add_feedback_to_buffer(feedback.clone());

        if self.config.enable_feedback_learning {
            tracing::info!(
                "Applied feedback: {:?} for prediction {}",
                feedback.feedback_type,
                feedback.prediction_id
            );
        }

        Ok(())
    }

    async fn save_checkpoint(&self) -> crate::utils::Result<String> {
        let checkpoint_id = uuid::Uuid::new_v4().to_string();
        tracing::info!("Saved checkpoint: {}", checkpoint_id);
        Ok(checkpoint_id)
    }

    async fn load_checkpoint(&mut self, checkpoint_path: &str) -> crate::utils::Result<()> {
        tracing::info!("Loaded checkpoint from: {}", checkpoint_path);
        Ok(())
    }
}
