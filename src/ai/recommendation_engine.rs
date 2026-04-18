use crate::utils::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RecommendationType {
    Skill,
    Agent,
    Plugin,
    Soul,
    Config,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBehavior {
    pub user_id: String,
    pub item_id: String,
    pub item_type: RecommendationType,
    pub action: BehaviorAction,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub context: HashMap<String, String>,
    pub duration: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BehaviorAction {
    View,
    Use,
    Like,
    Dislike,
    Save,
    Share,
    Comment,
    Install,
    Uninstall,
    Rate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Duration {
    pub seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemFeature {
    pub item_id: String,
    pub item_type: RecommendationType,
    pub features: Vec<f32>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub popularity_score: f32,
    pub average_rating: f32,
    pub rating_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub interests: HashSet<String>,
    pub used_items: HashSet<String>,
    pub liked_items: HashSet<String>,
    pub disliked_items: HashSet<String>,
    pub saved_items: HashSet<String>,
    pub preference_scores: HashMap<String, f32>,
    pub recent_activities: Vec<UserBehavior>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl UserProfile {
    pub fn new(user_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            user_id,
            interests: HashSet::new(),
            used_items: HashSet::new(),
            liked_items: HashSet::new(),
            disliked_items: HashSet::new(),
            saved_items: HashSet::new(),
            preference_scores: HashMap::new(),
            recent_activities: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_behavior(&mut self, behavior: UserBehavior) {
        match behavior.action {
            BehaviorAction::View => {
                if let Some(tags) = behavior.context.get("tags") {
                    self.interests
                        .extend(tags.split(',').map(|t| t.trim().to_string()));
                }
            }
            BehaviorAction::Use => {
                self.used_items.insert(behavior.item_id.clone());
            }
            BehaviorAction::Like => {
                self.liked_items.insert(behavior.item_id.clone());
                *self
                    .preference_scores
                    .entry(behavior.item_id.clone())
                    .or_insert(0.0) += 1.0;
            }
            BehaviorAction::Dislike => {
                self.disliked_items.insert(behavior.item_id.clone());
                *self
                    .preference_scores
                    .entry(behavior.item_id.clone())
                    .or_insert(0.0) -= 1.0;
            }
            BehaviorAction::Save => {
                self.saved_items.insert(behavior.item_id.clone());
                *self
                    .preference_scores
                    .entry(behavior.item_id.clone())
                    .or_insert(0.0) += 0.5;
            }
            _ => {}
        }

        self.recent_activities.push(behavior);
        if self.recent_activities.len() > 100 {
            self.recent_activities.remove(0);
        }

        self.updated_at = chrono::Utc::now();
    }

    pub fn get_preference_score(&self, item_id: &str) -> f32 {
        self.preference_scores.get(item_id).copied().unwrap_or(0.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub item_id: String,
    pub item_type: RecommendationType,
    pub score: f32,
    pub reason: RecommendationReason,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationReason {
    SimilarToLiked,
    Popular,
    Trending,
    RecentActivity,
    CollaborativeFiltering,
    ContentBased,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationRequest {
    pub user_id: String,
    pub item_types: Vec<RecommendationType>,
    pub limit: usize,
    pub context: Option<HashMap<String, String>>,
    pub exclude_used: bool,
    pub exclude_liked: bool,
}

pub struct RecommendationEngine {
    user_profiles: Arc<DashMap<String, UserProfile>>,
    item_features: Arc<DashMap<String, ItemFeature>>,
    behaviors: Arc<DashMap<String, Vec<UserBehavior>>>,
    item_index: Arc<DashMap<RecommendationType, Vec<String>>>,
    tag_index: Arc<DashMap<String, Vec<String>>>,
    category_index: Arc<DashMap<String, Vec<String>>>,
}

impl RecommendationEngine {
    pub fn new() -> Self {
        Self {
            user_profiles: Arc::new(DashMap::new()),
            item_features: Arc::new(DashMap::new()),
            behaviors: Arc::new(DashMap::new()),
            item_index: Arc::new(DashMap::new()),
            tag_index: Arc::new(DashMap::new()),
            category_index: Arc::new(DashMap::new()),
        }
    }

    pub fn get_or_create_user_profile(&self, user_id: &str) -> UserProfile {
        if let Some(profile) = self.user_profiles.get(user_id) {
            return profile.value().clone();
        }

        let profile = UserProfile::new(user_id.to_string());
        self.user_profiles
            .insert(user_id.to_string(), profile.clone());
        profile
    }

    pub fn update_user_profile(&self, user_id: &str, profile: UserProfile) -> Result<()> {
        self.user_profiles.insert(user_id.to_string(), profile);
        Ok(())
    }

    pub fn record_behavior(&self, behavior: UserBehavior) -> Result<()> {
        info!(
            "Recording behavior: user={}, item={}, action={:?}",
            behavior.user_id, behavior.item_id, behavior.action
        );

        let user_id = behavior.user_id.clone();
        let mut profile = self.get_or_create_user_profile(&user_id);
        profile.add_behavior(behavior.clone());
        self.update_user_profile(&user_id, profile)?;

        self.behaviors
            .entry(user_id)
            .or_default()
            .push(behavior);

        Ok(())
    }

    pub fn register_item(&self, feature: ItemFeature) -> Result<()> {
        info!(
            "Registering item: {} ({:?})",
            feature.item_id, feature.item_type
        );

        let item_id = feature.item_id.clone();
        let item_type = feature.item_type.clone();

        self.item_features.insert(item_id.clone(), feature.clone());

        self.item_index
            .entry(item_type)
            .or_default()
            .push(item_id.clone());

        for tag in &feature.tags {
            self.tag_index
                .entry(tag.clone())
                .or_default()
                .push(item_id.clone());
        }

        for category in &feature.categories {
            self.category_index
                .entry(category.clone())
                .or_default()
                .push(item_id.clone());
        }

        Ok(())
    }

    pub fn get_item_feature(&self, item_id: &str) -> Option<ItemFeature> {
        self.item_features.get(item_id).map(|f| f.value().clone())
    }

    pub fn get_recommendations(
        &self,
        request: RecommendationRequest,
    ) -> Result<Vec<Recommendation>> {
        info!(
            "Generating recommendations for user: {}, types: {:?}",
            request.user_id, request.item_types
        );

        let profile = self.get_or_create_user_profile(&request.user_id);

        let mut candidates = HashSet::new();
        let item_types = if request.item_types.is_empty() {
            vec![RecommendationType::All]
        } else {
            request.item_types.clone()
        };

        for item_type in item_types {
            if let Some(item_ids) = self.item_index.get(&item_type) {
                candidates.extend(item_ids.iter().cloned());
            }
        }

        if request.item_types.contains(&RecommendationType::All) {
            for entry in self.item_index.iter() {
                candidates.extend(entry.value().iter().cloned());
            }
        }

        if request.exclude_used {
            candidates.retain(|id| !profile.used_items.contains(id));
        }
        if request.exclude_liked {
            candidates.retain(|id| !profile.liked_items.contains(id));
        }
        candidates.retain(|id| !profile.disliked_items.contains(id));

        let mut recommendations = Vec::new();

        for item_id in candidates {
            if let Some(feature) = self.get_item_feature(&item_id) {
                let score = self.calculate_score(&profile, &feature, &request);
                let reason = self.determine_reason(&profile, &feature);

                recommendations.push(Recommendation {
                    item_id,
                    item_type: feature.item_type,
                    score,
                    reason,
                    confidence: score.min(1.0),
                });
            }
        }

        recommendations.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        recommendations.truncate(request.limit);

        Ok(recommendations)
    }

    fn calculate_score(
        &self,
        profile: &UserProfile,
        feature: &ItemFeature,
        request: &RecommendationRequest,
    ) -> f32 {
        let mut score = 0.0;

        let preference_score = profile.get_preference_score(&feature.item_id);
        score += preference_score * 0.3;

        let mut interest_match = 0;
        for tag in &feature.tags {
            if profile.interests.contains(tag) {
                interest_match += 1;
            }
        }
        if !feature.tags.is_empty() {
            score += (interest_match as f32 / feature.tags.len() as f32) * 0.2;
        }

        score += feature.popularity_score * 0.2;
        score += feature.average_rating * 0.1;

        let mut context_match = 0;
        if let Some(context) = &request.context {
            for tag in &feature.tags {
                if context.values().any(|v| v.contains(tag)) {
                    context_match += 1;
                }
            }
        }
        if !feature.tags.is_empty() {
            score += (context_match as f32 / feature.tags.len() as f32) * 0.2;
        }

        score
    }

    fn determine_reason(
        &self,
        profile: &UserProfile,
        feature: &ItemFeature,
    ) -> RecommendationReason {
        let mut reasons = Vec::new();

        let has_similar_liked = profile.liked_items.iter().any(|liked_id| {
            if let Some(liked_feature) = self.get_item_feature(liked_id) {
                let common_tags = liked_feature
                    .tags
                    .iter()
                    .filter(|tag| feature.tags.contains(tag))
                    .count();
                common_tags >= 2
            } else {
                false
            }
        });

        if has_similar_liked {
            reasons.push(RecommendationReason::SimilarToLiked);
        }

        if feature.popularity_score > 0.8 {
            reasons.push(RecommendationReason::Popular);
        }

        if feature.average_rating > 4.5 {
            reasons.push(RecommendationReason::Trending);
        }

        if reasons.is_empty() {
            RecommendationReason::Hybrid
        } else {
            reasons[0].clone()
        }
    }

    pub fn get_trending_items(
        &self,
        item_type: RecommendationType,
        limit: usize,
    ) -> Vec<ItemFeature> {
        let mut items = Vec::new();

        if let Some(item_ids) = self.item_index.get(&item_type) {
            for item_id in item_ids.iter() {
                if let Some(feature) = self.get_item_feature(item_id) {
                    items.push(feature);
                }
            }
        }

        items.sort_by(|a, b| {
            b.popularity_score
                .partial_cmp(&a.popularity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        items.truncate(limit);
        items
    }

    pub fn get_similar_items(&self, item_id: &str, limit: usize) -> Vec<ItemFeature> {
        let Some(target_feature) = self.get_item_feature(item_id) else {
            return Vec::new();
        };

        let mut similar_items = Vec::new();

        for entry in self.item_features.iter() {
            let feature = entry.value();
            if feature.item_id == item_id {
                continue;
            }

            let common_tags = target_feature
                .tags
                .iter()
                .filter(|tag| feature.tags.contains(tag))
                .count();

            if common_tags >= 1 {
                similar_items.push((feature.clone(), common_tags));
            }
        }

        similar_items.sort_by(|a, b| b.1.cmp(&a.1));

        similar_items
            .into_iter()
            .map(|(f, _)| f)
            .take(limit)
            .collect()
    }

    pub fn user_count(&self) -> usize {
        self.user_profiles.len()
    }

    pub fn item_count(&self) -> usize {
        self.item_features.len()
    }

    pub fn behavior_count(&self) -> usize {
        self.behaviors.iter().map(|e| e.value().len()).sum()
    }
}

impl Default for RecommendationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommendation_engine_new() {
        let engine = RecommendationEngine::new();
        assert_eq!(engine.user_count(), 0);
        assert_eq!(engine.item_count(), 0);
    }

    #[test]
    fn test_user_profile() {
        let mut profile = UserProfile::new("test-user".to_string());

        let behavior = UserBehavior {
            user_id: "test-user".to_string(),
            item_id: "test-item".to_string(),
            item_type: RecommendationType::Skill,
            action: BehaviorAction::Like,
            timestamp: chrono::Utc::now(),
            context: HashMap::new(),
            duration: None,
        };

        profile.add_behavior(behavior);

        assert!(profile.liked_items.contains("test-item"));
    }

    #[test]
    fn test_register_item() {
        let engine = RecommendationEngine::new();

        let feature = ItemFeature {
            item_id: "test-skill".to_string(),
            item_type: RecommendationType::Skill,
            features: vec![0.0, 0.0, 0.0],
            tags: vec!["web".to_string(), "search".to_string()],
            categories: vec!["tools".to_string()],
            popularity_score: 0.9,
            average_rating: 4.8,
            rating_count: 100,
        };

        engine.register_item(feature).unwrap();
        assert_eq!(engine.item_count(), 1);
    }

    #[test]
    fn test_get_recommendations() {
        let engine = RecommendationEngine::new();

        let feature = ItemFeature {
            item_id: "test-skill".to_string(),
            item_type: RecommendationType::Skill,
            features: vec![0.0, 0.0, 0.0],
            tags: vec!["web".to_string(), "search".to_string()],
            categories: vec!["tools".to_string()],
            popularity_score: 0.9,
            average_rating: 4.8,
            rating_count: 100,
        };

        engine.register_item(feature).unwrap();

        let request = RecommendationRequest {
            user_id: "test-user".to_string(),
            item_types: vec![RecommendationType::Skill],
            limit: 5,
            context: None,
            exclude_used: false,
            exclude_liked: false,
        };

        let recommendations = engine.get_recommendations(request).unwrap();
        assert!(!recommendations.is_empty());
    }
}
