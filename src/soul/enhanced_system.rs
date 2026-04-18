use crate::soul::Soul;
use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PersonalityType {
    Assistant,
    Developer,
    Designer,
    Analyst,
    Manager,
    Teacher,
    Creative,
    Analytical,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityProfile {
    pub personality_id: String,
    pub name: String,
    pub description: String,
    pub personality_type: PersonalityType,
    pub version: String,
    pub author: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
    pub is_official: bool,
    pub is_published: bool,
    pub tags: Vec<String>,
    pub soul: Soul,
    pub personality_traits: HashMap<String, f32>,
    pub behavioral_patterns: HashMap<String, String>,
    pub conversation_style: HashMap<String, serde_json::Value>,
    pub knowledge_base: Vec<String>,
    pub skill_preferences: HashMap<String, SkillPreference>,
    pub evolution_history: Vec<EvolutionRecord>,
    pub rating: f32,
    pub rating_count: u32,
    pub download_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPreference {
    pub skill_id: String,
    pub priority: u8,
    pub auto_select: bool,
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionRecord {
    pub record_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub change_type: EvolutionChangeType,
    pub description: String,
    pub previous_state: Option<serde_json::Value>,
    pub new_state: Option<serde_json::Value>,
    pub triggered_by: EvolutionTrigger,
    pub effectiveness_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvolutionChangeType {
    PersonalityTrait,
    BehavioralPattern,
    ConversationStyle,
    SkillPreference,
    KnowledgeBase,
    SoulUpdate,
    ManualAdjustment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvolutionTrigger {
    UserFeedback,
    PerformanceMetrics,
    ConversationAnalysis,
    SkillUsagePatterns,
    Manual,
    SystemUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityRating {
    pub rating_id: String,
    pub personality_id: String,
    pub user_id: String,
    pub rating: u8,
    pub comment: Option<String>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityMarketEntry {
    pub entry_id: String,
    pub personality_id: String,
    pub marketplace_id: String,
    pub price: Option<f64>,
    pub currency: Option<String>,
    pub listed_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
    pub total_sales: u64,
    pub revenue: f64,
}

pub struct EnhancedSoulSystem {
    pub personalities: Arc<DashMap<String, PersonalityProfile>>,
    active_personality: Arc<DashMap<String, String>>,
    ratings: Arc<DashMap<String, Vec<PersonalityRating>>>,
    market_entries: Arc<DashMap<String, PersonalityMarketEntry>>,
    personality_type_index: Arc<DashMap<PersonalityType, Vec<String>>>,
    tag_index: Arc<DashMap<String, Vec<String>>>,
    author_index: Arc<DashMap<String, Vec<String>>>,
    evolution_tracker: Arc<DashMap<String, Vec<EvolutionRecord>>>,
    storage_path: PathBuf,
}

impl EnhancedSoulSystem {
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;

        let system = Self {
            personalities: Arc::new(DashMap::new()),
            active_personality: Arc::new(DashMap::new()),
            ratings: Arc::new(DashMap::new()),
            market_entries: Arc::new(DashMap::new()),
            personality_type_index: Arc::new(DashMap::new()),
            tag_index: Arc::new(DashMap::new()),
            author_index: Arc::new(DashMap::new()),
            evolution_tracker: Arc::new(DashMap::new()),
            storage_path,
        };

        system.load()?;
        Ok(system)
    }

    fn save(&self) -> Result<()> {
        let personalities_path = self.storage_path.join("personalities.json");
        let personalities: Vec<_> = self
            .personalities
            .iter()
            .map(|e| e.value().clone())
            .collect();
        std::fs::write(
            &personalities_path,
            serde_json::to_string_pretty(&personalities)?,
        )?;

        let active_personality_path = self.storage_path.join("active_personality.json");
        let active_personality_map: Vec<(String, String)> = self
            .active_personality
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect();
        std::fs::write(
            &active_personality_path,
            serde_json::to_string_pretty(&active_personality_map)?,
        )?;

        let ratings_path = self.storage_path.join("ratings.json");
        let ratings_map: Vec<(String, Vec<PersonalityRating>)> = self
            .ratings
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect();
        std::fs::write(&ratings_path, serde_json::to_string_pretty(&ratings_map)?)?;

        let market_entries_path = self.storage_path.join("market_entries.json");
        let market_entries: Vec<_> = self
            .market_entries
            .iter()
            .map(|e| e.value().clone())
            .collect();
        std::fs::write(
            &market_entries_path,
            serde_json::to_string_pretty(&market_entries)?,
        )?;

        let evolution_tracker_path = self.storage_path.join("evolution_tracker.json");
        let evolution_tracker_map: Vec<(String, Vec<EvolutionRecord>)> = self
            .evolution_tracker
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect();
        std::fs::write(
            &evolution_tracker_path,
            serde_json::to_string_pretty(&evolution_tracker_map)?,
        )?;

        Ok(())
    }

    fn load(&self) -> Result<()> {
        let personalities_path = self.storage_path.join("personalities.json");
        if personalities_path.exists() {
            let content = std::fs::read_to_string(&personalities_path)?;
            let personalities: Vec<PersonalityProfile> = serde_json::from_str(&content)?;
            for personality in personalities {
                self.personalities
                    .insert(personality.personality_id.clone(), personality.clone());
                self.update_indices(&personality.personality_id, &personality);
            }
        }

        let active_personality_path = self.storage_path.join("active_personality.json");
        if active_personality_path.exists() {
            let content = std::fs::read_to_string(&active_personality_path)?;
            let active_personality_map: Vec<(String, String)> = serde_json::from_str(&content)?;
            for (user_id, personality_id) in active_personality_map {
                self.active_personality.insert(user_id, personality_id);
            }
        }

        let ratings_path = self.storage_path.join("ratings.json");
        if ratings_path.exists() {
            let content = std::fs::read_to_string(&ratings_path)?;
            let ratings_map: Vec<(String, Vec<PersonalityRating>)> =
                serde_json::from_str(&content)?;
            for (personality_id, ratings) in ratings_map {
                self.ratings.insert(personality_id, ratings);
            }
        }

        let market_entries_path = self.storage_path.join("market_entries.json");
        if market_entries_path.exists() {
            let content = std::fs::read_to_string(&market_entries_path)?;
            let market_entries: Vec<PersonalityMarketEntry> = serde_json::from_str(&content)?;
            for entry in market_entries {
                self.market_entries.insert(entry.entry_id.clone(), entry);
            }
        }

        let evolution_tracker_path = self.storage_path.join("evolution_tracker.json");
        if evolution_tracker_path.exists() {
            let content = std::fs::read_to_string(&evolution_tracker_path)?;
            let evolution_tracker_map: Vec<(String, Vec<EvolutionRecord>)> =
                serde_json::from_str(&content)?;
            for (personality_id, records) in evolution_tracker_map {
                self.evolution_tracker.insert(personality_id, records);
            }
        }

        Ok(())
    }

    pub fn register_personality(&self, personality: PersonalityProfile) -> Result<()> {
        info!(
            "Registering personality: {} ({:?}) by {}",
            personality.name, personality.personality_type, personality.author
        );

        let personality_id = personality.personality_id.clone();

        if self.personalities.contains_key(&personality_id) {
            return Err(AetherisError::Validation(format!(
                "Personality with ID '{}' already exists",
                personality_id
            )));
        }

        self.personalities
            .insert(personality_id.clone(), personality.clone());

        self.update_indices(&personality_id, &personality);
        self.save()?;

        Ok(())
    }

    fn update_indices(&self, personality_id: &str, personality: &PersonalityProfile) {
        self.personality_type_index
            .entry(personality.personality_type.clone())
            .or_default()
            .push(personality_id.to_string());

        for tag in &personality.tags {
            self.tag_index
                .entry(tag.clone())
                .or_default()
                .push(personality_id.to_string());
        }

        self.author_index
            .entry(personality.author.clone())
            .or_default()
            .push(personality_id.to_string());
    }

    pub fn get_personality(&self, personality_id: &str) -> Option<PersonalityProfile> {
        self.personalities
            .get(personality_id)
            .map(|p| p.value().clone())
    }

    pub fn list_personalities(&self) -> Vec<PersonalityProfile> {
        self.personalities
            .iter()
            .filter(|entry| entry.value().is_published)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn switch_personality(&self, user_id: &str, personality_id: &str) -> Result<()> {
        if !self.personalities.contains_key(personality_id) {
            return Err(AetherisError::NotFound(format!(
                "Personality not found: {}",
                personality_id
            )));
        }

        info!(
            "Switching user {} to personality {}",
            user_id, personality_id
        );

        self.active_personality
            .insert(user_id.to_string(), personality_id.to_string());

        if let Some(mut personality) = self.personalities.get_mut(personality_id) {
            personality.is_active = true;
        }

        self.save()?;
        Ok(())
    }

    pub fn get_active_personality(&self, user_id: &str) -> Option<PersonalityProfile> {
        self.active_personality
            .get(user_id)
            .and_then(|p| self.get_personality(p.value()))
    }

    pub fn evolve_personality(
        &self,
        personality_id: &str,
        change_type: EvolutionChangeType,
        description: String,
        previous_state: Option<serde_json::Value>,
        new_state: Option<serde_json::Value>,
        triggered_by: EvolutionTrigger,
    ) -> Result<()> {
        info!(
            "Evolving personality: {} (change: {:?}, triggered by: {:?})",
            personality_id, change_type, triggered_by
        );

        let evolution_record = EvolutionRecord {
            record_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            change_type,
            description,
            previous_state,
            new_state,
            triggered_by,
            effectiveness_score: None,
        };

        self.evolution_tracker
            .entry(personality_id.to_string())
            .or_default()
            .push(evolution_record.clone());

        self.apply_evolution(personality_id, &evolution_record)?;
        self.save()?;

        Ok(())
    }

    fn apply_evolution(&self, personality_id: &str, record: &EvolutionRecord) -> Result<()> {
        if let Some(mut personality) = self.personalities.get_mut(personality_id) {
            personality.updated_at = chrono::Utc::now();
            personality.evolution_history.push(record.clone());
        }

        Ok(())
    }

    pub fn get_evolution_history(&self, personality_id: &str) -> Vec<EvolutionRecord> {
        self.evolution_tracker
            .get(personality_id)
            .map(|h| h.value().clone())
            .unwrap_or_default()
    }

    pub fn rate_personality(
        &self,
        personality_id: &str,
        user_id: String,
        rating: u8,
        comment: Option<String>,
        tags: Vec<String>,
    ) -> Result<()> {
        if !(1..=5).contains(&rating) {
            return Err(AetherisError::Validation(
                "Rating must be between 1 and 5".to_string(),
            ));
        }

        info!(
            "Rating personality: {} by user: {} with rating: {}",
            personality_id, user_id, rating
        );

        let personality_rating = PersonalityRating {
            rating_id: uuid::Uuid::new_v4().to_string(),
            personality_id: personality_id.to_string(),
            user_id,
            rating,
            comment,
            tags,
            created_at: chrono::Utc::now(),
        };

        self.ratings
            .entry(personality_id.to_string())
            .or_default()
            .push(personality_rating);

        self.update_personality_rating(personality_id)?;
        self.save()?;

        Ok(())
    }

    fn update_personality_rating(&self, personality_id: &str) -> Result<()> {
        if let Some(ratings) = self.ratings.get(personality_id) {
            let rating_count = ratings.len() as u32;
            if rating_count > 0 {
                let total_rating: u32 = ratings.iter().map(|r| r.rating as u32).sum();
                let average_rating = total_rating as f32 / rating_count as f32;

                if let Some(mut personality) = self.personalities.get_mut(personality_id) {
                    personality.rating = average_rating;
                    personality.rating_count = rating_count;
                }
            }
        }

        Ok(())
    }

    pub fn list_personality_ratings(&self, personality_id: &str) -> Vec<PersonalityRating> {
        self.ratings
            .get(personality_id)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }

    pub fn publish_to_market(
        &self,
        personality_id: &str,
        marketplace_id: &str,
        price: Option<f64>,
        currency: Option<String>,
    ) -> Result<()> {
        if !self.personalities.contains_key(personality_id) {
            return Err(AetherisError::NotFound(format!(
                "Personality not found: {}",
                personality_id
            )));
        }

        info!(
            "Publishing personality: {} to marketplace: {}",
            personality_id, marketplace_id
        );

        let market_entry = PersonalityMarketEntry {
            entry_id: uuid::Uuid::new_v4().to_string(),
            personality_id: personality_id.to_string(),
            marketplace_id: marketplace_id.to_string(),
            price,
            currency,
            listed_at: chrono::Utc::now(),
            is_active: true,
            total_sales: 0,
            revenue: 0.0,
        };

        self.market_entries
            .insert(market_entry.entry_id.clone(), market_entry);

        if let Some(mut personality) = self.personalities.get_mut(personality_id) {
            personality.is_published = true;
        }

        self.save()?;
        Ok(())
    }

    pub fn purchase_personality(
        &self,
        entry_id: &str,
        buyer_id: &str,
    ) -> Result<PersonalityProfile> {
        let entry = self
            .market_entries
            .get(entry_id)
            .ok_or_else(|| {
                AetherisError::NotFound(format!("Market entry not found: {}", entry_id))
            })?
            .value()
            .clone();

        if !entry.is_active {
            return Err(AetherisError::Validation(
                "This personality is no longer available".to_string(),
            ));
        }

        let personality = self.get_personality(&entry.personality_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Personality not found: {}", entry.personality_id))
        })?;

        info!(
            "Purchasing personality: {} by user: {}",
            entry.personality_id, buyer_id
        );

        if let Some(mut market_entry) = self.market_entries.get_mut(entry_id) {
            market_entry.total_sales += 1;
            if let Some(price) = entry.price {
                market_entry.revenue += price;
            }
        }

        if let Some(mut personality) = self.personalities.get_mut(&entry.personality_id) {
            personality.download_count += 1;
        }

        self.save()?;
        Ok(personality)
    }

    pub fn search_personalities(&self, query: &str) -> Vec<PersonalityProfile> {
        let query_lower = query.to_lowercase();
        self.personalities
            .iter()
            .filter(|entry| {
                let personality = entry.value();
                if !personality.is_published {
                    return false;
                }

                personality.name.to_lowercase().contains(&query_lower)
                    || personality
                        .description
                        .to_lowercase()
                        .contains(&query_lower)
                    || personality
                        .personality_id
                        .to_lowercase()
                        .contains(&query_lower)
                    || personality.author.to_lowercase().contains(&query_lower)
                    || personality
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_personalities_by_type(
        &self,
        personality_type: &PersonalityType,
    ) -> Vec<PersonalityProfile> {
        self.personality_type_index
            .get(personality_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_personality(id))
                    .filter(|p| p.is_published)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_popular_personalities(&self, limit: usize) -> Vec<PersonalityProfile> {
        let mut personalities = self.list_personalities();
        personalities.sort_by(|a, b| {
            b.download_count.cmp(&a.download_count).then_with(|| {
                b.rating
                    .partial_cmp(&a.rating)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });
        personalities.truncate(limit);
        personalities
    }

    pub fn get_top_rated_personalities(&self, limit: usize) -> Vec<PersonalityProfile> {
        let mut personalities = self.list_personalities();
        personalities.sort_by(|a, b| {
            b.rating
                .partial_cmp(&a.rating)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.rating_count.cmp(&a.rating_count))
        });
        personalities.truncate(limit);
        personalities
    }

    pub fn personality_count(&self) -> usize {
        self.personalities.len()
    }

    pub fn published_personality_count(&self) -> usize {
        self.list_personalities().len()
    }

    pub fn get_personalities_by_tag(&self, tag: &str) -> Vec<PersonalityProfile> {
        self.tag_index
            .get(tag)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_personality(id))
                    .filter(|p| p.is_published)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_personalities_by_author(&self, author: &str) -> Vec<PersonalityProfile> {
        self.author_index
            .get(author)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_personality(id))
                    .filter(|p| p.is_published)
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for EnhancedSoulSystem {
    fn default() -> Self {
        let storage_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".aetheris")
            .join("soul-personalities");

        Self::new(storage_path).unwrap_or_else(|_| {
            let temp_dir = tempfile::tempdir().unwrap();
            Self::new(temp_dir.path().to_path_buf()).unwrap()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_soul_system_new() {
        let temp_dir = tempfile::tempdir().unwrap();
        let system = EnhancedSoulSystem::new(temp_dir.path().to_path_buf());
        assert!(system.is_ok());
    }

    #[test]
    fn test_register_personality() {
        let temp_dir = tempfile::tempdir().unwrap();
        let system = EnhancedSoulSystem::new(temp_dir.path().to_path_buf()).unwrap();

        let soul = Soul::default();
        let personality = PersonalityProfile {
            personality_id: "test-personality".to_string(),
            name: "Test Personality".to_string(),
            description: "A test personality".to_string(),
            personality_type: PersonalityType::Assistant,
            version: "1.0.0".to_string(),
            author: "test-author".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            is_active: false,
            is_official: false,
            is_published: true,
            tags: vec!["test".to_string()],
            soul,
            personality_traits: HashMap::new(),
            behavioral_patterns: HashMap::new(),
            conversation_style: HashMap::new(),
            knowledge_base: vec![],
            skill_preferences: HashMap::new(),
            evolution_history: vec![],
            rating: 0.0,
            rating_count: 0,
            download_count: 0,
        };

        let result = system.register_personality(personality);
        assert!(result.is_ok());
        assert_eq!(system.published_personality_count(), 1);
    }

    #[test]
    fn test_switch_personality() {
        let temp_dir = tempfile::tempdir().unwrap();
        let system = EnhancedSoulSystem::new(temp_dir.path().to_path_buf()).unwrap();

        let soul = Soul::default();
        let personality = PersonalityProfile {
            personality_id: "test-personality".to_string(),
            name: "Test Personality".to_string(),
            description: "A test personality".to_string(),
            personality_type: PersonalityType::Assistant,
            version: "1.0.0".to_string(),
            author: "test-author".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            is_active: false,
            is_official: false,
            is_published: true,
            tags: vec!["test".to_string()],
            soul,
            personality_traits: HashMap::new(),
            behavioral_patterns: HashMap::new(),
            conversation_style: HashMap::new(),
            knowledge_base: vec![],
            skill_preferences: HashMap::new(),
            evolution_history: vec![],
            rating: 0.0,
            rating_count: 0,
            download_count: 0,
        };

        system.register_personality(personality).unwrap();

        let result = system.switch_personality("test-user", "test-personality");
        assert!(result.is_ok());

        let active = system.get_active_personality("test-user");
        assert!(active.is_some());
    }
}
