use super::*;
use dashmap::DashMap;

pub struct FeedbackManager {
    feedback_store: DashMap<String, Feedback>,
    model_feedback_index: DashMap<String, Vec<String>>,
    prediction_feedback_index: DashMap<String, String>,
}

impl FeedbackManager {
    pub fn new() -> Self {
        Self {
            feedback_store: DashMap::new(),
            model_feedback_index: DashMap::new(),
            prediction_feedback_index: DashMap::new(),
        }
    }

    pub fn submit_feedback(&self, feedback: Feedback) -> String {
        let feedback_id = feedback.id.clone();

        self.model_feedback_index
            .entry(feedback.model_id.clone())
            .or_default()
            .push(feedback_id.clone());

        self.prediction_feedback_index
            .insert(feedback.prediction_id.clone(), feedback_id.clone());

        self.feedback_store.insert(feedback_id.clone(), feedback);

        feedback_id
    }

    pub fn get_feedback(&self, feedback_id: &str) -> Option<Feedback> {
        self.feedback_store.get(feedback_id).map(|f| f.clone())
    }

    pub fn get_feedback_for_prediction(&self, prediction_id: &str) -> Option<Feedback> {
        self.prediction_feedback_index
            .get(prediction_id)
            .and_then(|id| self.get_feedback(&id))
    }

    pub fn get_feedback_for_model(&self, model_id: &str, limit: Option<usize>) -> Vec<Feedback> {
        self.model_feedback_index
            .get(model_id)
            .map(|ids| {
                let mut feedbacks: Vec<Feedback> =
                    ids.iter().filter_map(|id| self.get_feedback(id)).collect();

                feedbacks.sort_by(|a, b| b.created_at.cmp(&a.created_at));

                if let Some(limit) = limit {
                    feedbacks.truncate(limit);
                }

                feedbacks
            })
            .unwrap_or_default()
    }

    pub fn list_feedback(&self, limit: Option<usize>) -> Vec<Feedback> {
        let mut feedbacks: Vec<Feedback> = self.feedback_store.iter().map(|f| f.clone()).collect();

        feedbacks.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        if let Some(limit) = limit {
            feedbacks.truncate(limit);
        }

        feedbacks
    }

    pub fn get_feedback_stats(&self, model_id: &str) -> FeedbackStats {
        let feedbacks = self.get_feedback_for_model(model_id, None);
        let positive_count = feedbacks
            .iter()
            .filter(|f| f.feedback_type == FeedbackType::Positive)
            .count();
        let negative_count = feedbacks
            .iter()
            .filter(|f| f.feedback_type == FeedbackType::Negative)
            .count();

        FeedbackStats {
            total: feedbacks.len(),
            positive: positive_count,
            negative: negative_count,
            positive_ratio: if feedbacks.is_empty() {
                0.0
            } else {
                positive_count as f64 / feedbacks.len() as f64
            },
        }
    }

    pub fn delete_feedback(&self, feedback_id: &str) -> bool {
        if let Some((_, feedback)) = self.feedback_store.remove(feedback_id) {
            if let Some(mut ids) = self.model_feedback_index.get_mut(&feedback.model_id) {
                ids.retain(|id| id != feedback_id);
            }
            self.prediction_feedback_index
                .remove(&feedback.prediction_id);
            true
        } else {
            false
        }
    }
}

impl Default for FeedbackManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackStats {
    pub total: usize,
    pub positive: usize,
    pub negative: usize,
    pub positive_ratio: f64,
}
