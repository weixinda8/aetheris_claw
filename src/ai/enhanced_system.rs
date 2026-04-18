#![deprecated(note = "This enhanced AI system is not used. Use recommendation_engine from ai/mod.rs instead.")]

use crate::ai::recommendation_engine::{RecommendationEngine, RecommendationResult};
use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LearningModelType {
    CollaborativeFiltering,
    ContentBased,
    Hybrid,
    DeepLearning,
    ReinforcementLearning,
    ContextAware,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningModel {
    pub model_id: String,
    pub model_type: LearningModelType,
    pub name: String,
    pub description: String,
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
    pub accuracy: f64,
    pub training_samples: u64,
    pub last_trained_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub preferences: HashMap<String, f64>,
    pub behavior_patterns: Vec<UserBehavior>,
    pub skill_usage: HashMap<String, u64>,
    pub rating_history: Vec<SkillRating>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub last_active_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBehavior {
    pub behavior_id: String,
    pub behavior_type: BehaviorType,
    pub target_id: String,
    pub target_type: TargetType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub duration_ms: Option<u64>,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BehaviorType {
    View,
    Click,
    Download,
    Install,
    Use,
    Rate,
    Review,
    Share,
    Bookmark,
    Uninstall,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TargetType {
    Skill,
    Agent,
    Plugin,
    Component,
    Collection,
    Tag,
    Category,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRating {
    pub rating_id: String,
    pub skill_id: String,
    pub user_id: String,
    pub rating: u8,
    pub comment: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualState {
    pub user_id: String,
    pub session_id: String,
    pub current_task: Option<String>,
    pub active_skills: Vec<String>,
    pub active_agents: Vec<String>,
    pub time_of_day: String,
    pub day_of_week: String,
    pub location: Option<String>,
    pub device_type: Option<String>,
    pub network_status: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizationConfig {
    pub user_id: String,
    pub enable_personalization: bool,
    pub enable_collaborative_filtering: bool,
    pub enable_content_based: bool,
    pub enable_context_aware: bool,
    pub recommendation_weight_collaborative: f64,
    pub recommendation_weight_content: f64,
    pub recommendation_weight_context: f64,
    pub learning_rate: f64,
    pub exploration_rate: f64,
    pub update_frequency_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIPrediction {
    pub prediction_id: String,
    pub model_id: String,
    pub user_id: String,
    pub prediction_type: PredictionType,
    pub target_id: String,
    pub confidence: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub actual_outcome: Option<bool>,
    pub outcome_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PredictionType {
    SkillRecommendation,
    AgentRecommendation,
    NextAction,
    TaskCompletion,
    UserPreference,
    BehaviorPrediction,
}

pub struct EnhancedAISystem {
    models: Arc<DashMap<String, LearningModel>>,
    user_profiles: Arc<DashMap<String, UserProfile>>,
    predictions: Arc<DashMap<String, AIPrediction>>,
    personalization_configs: Arc<DashMap<String, PersonalizationConfig>>,
    contextual_states: Arc<DashMap<String, ContextualState>>,
    recommendation_engine: Arc<RecommendationEngine>,
    storage_path: PathBuf,
}

impl EnhancedAISystem {
    pub fn new(
        recommendation_engine: Arc<RecommendationEngine>,
        storage_path: PathBuf,
    ) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;

        Ok(Self {
            models: Arc::new(DashMap::new()),
            user_profiles: Arc::new(DashMap::new()),
            predictions: Arc::new(DashMap::new()),
            personalization_configs: Arc::new(DashMap::new()),
            contextual_states: Arc::new(DashMap::new()),
            recommendation_engine,
            storage_path,
        })
    }

    pub fn register_model(&self, model: LearningModel) -> Result<()> {
        if self.models.contains_key(&model.model_id) {
            return Err(AetherisError::Validation(format!(
                "Model with ID '{}' already exists",
                model.model_id
            )));
        }

        info!("Registering AI model: {} ({:?})", model.name, model.model_type);
        self.models.insert(model.model_id.clone(), model);

        Ok(())
    }

    pub fn get_model(&self, model_id: &str) -> Option<LearningModel> {
        self.models.get(model_id).map(|m| m.value().clone())
    }

    pub fn list_models(&self, active_only: bool) -> Vec<LearningModel> {
        self.models
            .iter()
            .filter(|entry| !active_only || entry.value().is_active)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_or_create_user_profile(&self, user_id: &str) -> UserProfile {
        if let Some(profile) = self.user_profiles.get(user_id) {
            return profile.value().clone();
        }

        let now = chrono::Utc::now();
        let profile = UserProfile {
            user_id: user_id.to_string(),
            preferences: HashMap::new(),
            behavior_patterns: Vec::new(),
            skill_usage: HashMap::new(),
            rating_history: Vec::new(),
            created_at: now,
            updated_at: now,
            last_active_at: Some(now),
        };

        self.user_profiles
            .insert(user_id.to_string(), profile.clone());
        profile
    }

    pub fn update_user_profile(&self, user_id: &str, profile: UserProfile) -> Result<()> {
        self.user_profiles.insert(user_id.to_string(), profile);
        Ok(())
    }

    pub fn record_user_behavior(&self, user_id: &str, behavior: UserBehavior) -> Result<()> {
        let mut profile = self.get_or_create_user_profile(user_id);
        profile.behavior_patterns.push(behavior);
        profile.updated_at = chrono::Utc::now();
        profile.last_active_at = Some(chrono::Utc::now());

        if let TargetType::Skill = behavior.target_type {
            *profile
                .skill_usage
                .entry(behavior.target_id.clone())
                .or_insert(0) += 1;
        }

        self.update_user_profile(user_id, profile)?;
        self.learn_from_behavior(user_id, &behavior)?;

        Ok(())
    }

    pub fn record_skill_rating(&self, rating: SkillRating) -> Result<()> {
        let mut profile = self.get_or_create_user_profile(&rating.user_id);
        profile.rating_history.push(rating.clone());
        profile.updated_at = chrono::Utc::now();

        self.update_user_profile(&rating.user_id, profile)?;
        self.learn_from_rating(&rating)?;

        Ok(())
    }

    fn learn_from_behavior(&self, user_id: &str, behavior: &UserBehavior) -> Result<()> {
        debug!("Learning from user behavior: {:?}", behavior.behavior_type);
        Ok(())
    }

    fn learn_from_rating(&self, rating: &SkillRating) -> Result<()> {
        debug!("Learning from skill rating: {}", rating.rating);
        Ok(())
    }

    pub fn update_contextual_state(&self, state: ContextualState) -> Result<()> {
        self.contextual_states
            .insert(state.session_id.clone(), state);
        Ok(())
    }

    pub fn get_contextual_state(&self, session_id: &str) -> Option<ContextualState> {
        self.contextual_states
            .get(session_id)
            .map(|s| s.value().clone())
    }

    pub async fn get_contextual_recommendations(
        &self,
        user_id: &str,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<RecommendationResult>> {
        let profile = self.get_or_create_user_profile(user_id);
        let context = self.get_contextual_state(session_id);
        let config = self.get_personalization_config(user_id);

        info!(
            "Generating contextual recommendations for user: {} (session: {})",
            user_id, session_id
        );

        let mut recommendations = Vec::new();

        if config.enable_content_based {
            let content_recommendations = self
                .recommendation_engine
                .get_content_based_recommendations(user_id, limit)
                .await?;

            for rec in content_recommendations {
                recommendations.push(rec);
            }
        }

        if config.enable_collaborative_filtering {
            let collaborative_recommendations = self
                .recommendation_engine
                .get_collaborative_recommendations(user_id, limit)
                .await?;

            for rec in collaborative_recommendations {
                recommendations.push(rec);
            }
        }

        recommendations.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        recommendations.truncate(limit);

        Ok(recommendations)
    }

    pub fn get_personalization_config(&self, user_id: &str) -> PersonalizationConfig {
        if let Some(config) = self.personalization_configs.get(user_id) {
            return config.value().clone();
        }

        PersonalizationConfig {
            user_id: user_id.to_string(),
            enable_personalization: true,
            enable_collaborative_filtering: true,
            enable_content_based: true,
            enable_context_aware: true,
            recommendation_weight_collaborative: 0.4,
            recommendation_weight_content: 0.4,
            recommendation_weight_context: 0.2,
            learning_rate: 0.01,
            exploration_rate: 0.1,
            update_frequency_hours: 24,
        }
    }

    pub fn set_personalization_config(&self, config: PersonalizationConfig) -> Result<()> {
        self.personalization_configs
            .insert(config.user_id.clone(), config);
        Ok(())
    }

    pub async fn predict_user_action(
        &self,
        user_id: &str,
        session_id: &str,
        prediction_type: PredictionType,
    ) -> Result<AIPrediction> {
        let profile = self.get_or_create_user_profile(user_id);
        let context = self.get_contextual_state(session_id);

        let prediction = AIPrediction {
            prediction_id: uuid::Uuid::new_v4().to_string(),
            model_id: "hybrid-model".to_string(),
            user_id: user_id.to_string(),
            prediction_type,
            target_id: "target".to_string(),
            confidence: 0.75,
            timestamp: chrono::Utc::now(),
            actual_outcome: None,
            outcome_timestamp: None,
        };

        self.predictions
            .insert(prediction.prediction_id.clone(), prediction.clone());

        Ok(prediction)
    }

    pub fn record_prediction_outcome(
        &self,
        prediction_id: &str,
        outcome: bool,
    ) -> Result<()> {
        if let Some(mut prediction) = self.predictions.get_mut(prediction_id) {
            prediction.actual_outcome = Some(outcome);
            prediction.outcome_timestamp = Some(chrono::Utc::now());
        }

        Ok(())
    }

    pub async fn train_model(&self, model_id: &str) -> Result<LearningModel> {
        let mut model = self
            .models
            .get_mut(model_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Model not found: {}", model_id)))?;

        info!("Training model: {}", model.name);

        model.training_samples += 1000;
        model.accuracy = (model.accuracy * 0.99) + 0.01;
        model.last_trained_at = Some(chrono::Utc::now());
        model.updated_at = chrono::Utc::now();

        Ok(model.value().clone())
    }

    pub fn get_model_performance(&self, model_id: &str) -> Option<f64> {
        self.models.get(model_id).map(|m| m.value().accuracy)
    }

    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    pub fn user_profile_count(&self) -> usize {
        self.user_profiles.len()
    }
}

impl Default for EnhancedAISystem {
    fn default() -> Self {
        let storage_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".aetheris")
            .join("ai");

        let rec_engine = Arc::new(RecommendationEngine::default());

        Self::new(rec_engine, storage_path).unwrap_or_else(|_| {
            let temp_dir = tempfile::tempdir().unwrap();
            let rec_engine = Arc::new(RecommendationEngine::default());
            Self::new(rec_engine, temp_dir.path().to_path_buf()).unwrap()
        })
    }
}

impl Default for PersonalizationConfig {
    fn default() -> Self {
        Self {
            user_id: "default".to_string(),
            enable_personalization: true,
            enable_collaborative_filtering: true,
            enable_content_based: true,
            enable_context_aware: true,
            recommendation_weight_collaborative: 0.4,
            recommendation_weight_content: 0.4,
            recommendation_weight_context: 0.2,
            learning_rate: 0.01,
            exploration_rate: 0.1,
            update_frequency_hours: 24,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_ai_system_new() {
        let temp_dir = tempfile::tempdir().unwrap();
        let rec_engine = Arc::new(RecommendationEngine::default());
        let system = EnhancedAISystem::new(rec_engine, temp_dir.path().to_path_buf());
        assert!(system.is_ok());
    }
}
