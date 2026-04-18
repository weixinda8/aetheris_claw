use crate::utils::Result;
use chrono::{Datelike, Timelike};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PreloadableType {
    Skill,
    Agent,
    Plugin,
    Component,
    Soul,
    Config,
}

impl PreloadableType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(PreloadableType::Skill),
            1 => Some(PreloadableType::Agent),
            2 => Some(PreloadableType::Plugin),
            3 => Some(PreloadableType::Component),
            4 => Some(PreloadableType::Soul),
            5 => Some(PreloadableType::Config),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActivity {
    pub user_id: String,
    pub item_id: String,
    pub item_type: PreloadableType,
    pub activity_type: ActivityType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub session_id: String,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ActivityType {
    Search,
    View,
    Use,
    Like,
    Save,
    Install,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemUsageStats {
    pub item_id: String,
    pub item_type: PreloadableType,
    pub total_uses: u64,
    pub recent_uses: u64,
    pub average_session_duration: Duration,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    pub time_of_day_distribution: HashMap<u8, u64>,
    pub day_of_week_distribution: HashMap<u8, u64>,
    pub usage_pattern: UsagePattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePattern {
    pub peak_hours: Vec<u8>,
    pub common_sequences: Vec<Vec<String>>,
    pub frequent_cooccurrences: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreloadPrediction {
    pub item_id: String,
    pub item_type: PreloadableType,
    pub confidence: f32,
    pub predicted_time: Option<chrono::DateTime<chrono::Utc>>,
    pub reason: PreloadReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionCacheEntry {
    pub predictions: Vec<PreloadPrediction>,
    pub cached_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreloadReason {
    RecentUsage,
    PeakTime,
    CommonSequence,
    Cooccurrence,
    Recommendation,
    UsagePattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreloadConfig {
    pub enabled: bool,
    pub max_preloaded_items: usize,
    pub preload_threshold: f32,
    pub lookahead_window: Duration,
    pub cleanup_interval: Duration,
    pub usage_history_window: Duration,
}

impl Default for PreloadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_preloaded_items: 20,
            preload_threshold: 0.5,
            lookahead_window: Duration::from_secs(300),
            cleanup_interval: Duration::from_secs(600),
            usage_history_window: Duration::from_secs(86400 * 30),
        }
    }
}

pub struct SmartPreloader {
    config: std::sync::Mutex<PreloadConfig>,
    user_activities: Arc<DashMap<String, Vec<UserActivity>>>,
    item_stats: Arc<DashMap<String, ItemUsageStats>>,
    session_activities: Arc<DashMap<String, Vec<UserActivity>>>,
    preloaded_items: Arc<DashMap<String, PreloadedItem>>,
    prediction_cache: Arc<DashMap<String, PredictionCacheEntry>>,
    last_cleanup: Arc<std::sync::Mutex<Instant>>,
    storage_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreloadedItem {
    pub item_id: String,
    pub item_type: PreloadableType,
    pub preloaded_at: chrono::DateTime<chrono::Utc>,
    pub preload_confidence: f32,
    pub preload_reason: PreloadReason,
    pub used: bool,
    pub used_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl SmartPreloader {
    pub fn new(config: PreloadConfig, storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;

        let instance = Self {
            config: std::sync::Mutex::new(config),
            user_activities: Arc::new(DashMap::new()),
            item_stats: Arc::new(DashMap::new()),
            session_activities: Arc::new(DashMap::new()),
            preloaded_items: Arc::new(DashMap::new()),
            prediction_cache: Arc::new(DashMap::new()),
            last_cleanup: Arc::new(std::sync::Mutex::new(Instant::now())),
            storage_path,
        };

        instance.load()?;

        Ok(instance)
    }

    pub fn save(&self) -> Result<()> {
        let user_activities_path = self.storage_path.join("user_activities.json");
        let item_stats_path = self.storage_path.join("item_stats.json");
        let preloaded_items_path = self.storage_path.join("preloaded_items.json");
        let prediction_cache_path = self.storage_path.join("prediction_cache.json");
        let config_path = self.storage_path.join("config.json");

        let user_activities: Vec<(String, Vec<UserActivity>)> = self
            .user_activities
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        let item_stats: Vec<(String, ItemUsageStats)> = self
            .item_stats
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        let preloaded_items: Vec<(String, PreloadedItem)> = self
            .preloaded_items
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        let prediction_cache: Vec<(String, PredictionCacheEntry)> = self
            .prediction_cache
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        let config = self.config.lock().unwrap();

        std::fs::write(
            user_activities_path,
            serde_json::to_string_pretty(&user_activities)?,
        )?;
        std::fs::write(item_stats_path, serde_json::to_string_pretty(&item_stats)?)?;
        std::fs::write(
            preloaded_items_path,
            serde_json::to_string_pretty(&preloaded_items)?,
        )?;
        std::fs::write(
            prediction_cache_path,
            serde_json::to_string_pretty(&prediction_cache)?,
        )?;
        std::fs::write(config_path, serde_json::to_string_pretty(&*config)?)?;

        info!("SmartPreloader saved to: {:?}", self.storage_path);

        Ok(())
    }

    fn load(&self) -> Result<()> {
        let user_activities_path = self.storage_path.join("user_activities.json");
        let item_stats_path = self.storage_path.join("item_stats.json");
        let preloaded_items_path = self.storage_path.join("preloaded_items.json");
        let prediction_cache_path = self.storage_path.join("prediction_cache.json");
        let config_path = self.storage_path.join("config.json");

        if user_activities_path.exists() {
            let content = std::fs::read_to_string(user_activities_path)?;
            let user_activities: Vec<(String, Vec<UserActivity>)> = serde_json::from_str(&content)?;
            for (user_id, activities) in user_activities {
                self.user_activities.insert(user_id, activities);
            }
        }

        if item_stats_path.exists() {
            let content = std::fs::read_to_string(item_stats_path)?;
            let item_stats: Vec<(String, ItemUsageStats)> = serde_json::from_str(&content)?;
            for (key, stats) in item_stats {
                self.item_stats.insert(key, stats);
            }
        }

        if preloaded_items_path.exists() {
            let content = std::fs::read_to_string(preloaded_items_path)?;
            let preloaded_items: Vec<(String, PreloadedItem)> = serde_json::from_str(&content)?;
            for (key, item) in preloaded_items {
                self.preloaded_items.insert(key, item);
            }
        }

        if prediction_cache_path.exists() {
            let content = std::fs::read_to_string(prediction_cache_path)?;
            let prediction_cache: Vec<(String, PredictionCacheEntry)> =
                serde_json::from_str(&content)?;
            for (key, cache_entry) in prediction_cache {
                self.prediction_cache.insert(key, cache_entry);
            }
        }

        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            let config: PreloadConfig = serde_json::from_str(&content)?;
            *self.config.lock().unwrap() = config;
        }

        info!("SmartPreloader loaded from: {:?}", self.storage_path);

        Ok(())
    }

    pub fn record_activity(&self, activity: UserActivity) -> Result<()> {
        info!(
            "Recording activity: user={}, item={}, type={:?}",
            activity.user_id, activity.item_id, activity.activity_type
        );

        let item_key = format!("{}:{}", activity.item_type as u8, activity.item_id);

        self.user_activities
            .entry(activity.user_id.clone())
            .or_default()
            .push(activity.clone());

        self.session_activities
            .entry(activity.session_id.clone())
            .or_default()
            .push(activity.clone());

        self.update_item_stats(&activity, &item_key)?;

        self.update_usage_pattern(&activity);

        self.trigger_preload_check(&activity);

        self.cleanup_old_data()?;

        self.adapt_strategy()?;

        self.save()?;

        Ok(())
    }

    fn update_item_stats(&self, activity: &UserActivity, item_key: &str) -> Result<()> {
        let mut stats = self
            .item_stats
            .entry(item_key.to_string())
            .or_insert_with(|| ItemUsageStats {
                item_id: activity.item_id.clone(),
                item_type: activity.item_type,
                total_uses: 0,
                recent_uses: 0,
                average_session_duration: Duration::from_secs(0),
                last_used: None,
                time_of_day_distribution: HashMap::new(),
                day_of_week_distribution: HashMap::new(),
                usage_pattern: UsagePattern {
                    peak_hours: Vec::new(),
                    common_sequences: Vec::new(),
                    frequent_cooccurrences: HashMap::new(),
                },
            });

        stats.total_uses += 1;
        stats.recent_uses += 1;
        stats.last_used = Some(activity.timestamp);

        let hour = activity.timestamp.hour() as u8;
        *stats.time_of_day_distribution.entry(hour).or_insert(0) += 1;

        let weekday = activity.timestamp.weekday() as u8;
        *stats.day_of_week_distribution.entry(weekday).or_insert(0) += 1;

        Ok(())
    }

    fn update_usage_pattern(&self, activity: &UserActivity) {
        if let Some(session_activities) = self.session_activities.get(&activity.session_id) {
            let activities = session_activities.value();

            if activities.len() >= 2 {
                let sequence: Vec<String> = activities
                    .iter()
                    .rev()
                    .take(5)
                    .rev()
                    .map(|a| format!("{}:{}", a.item_type as u8, a.item_id))
                    .collect();

                if let Some(mut stats) = self.item_stats.get_mut(&format!(
                    "{}:{}",
                    activity.item_type as u8, activity.item_id
                )) {
                    if !stats.usage_pattern.common_sequences.contains(&sequence) {
                        stats.usage_pattern.common_sequences.push(sequence);
                        if stats.usage_pattern.common_sequences.len() > 10 {
                            stats.usage_pattern.common_sequences.remove(0);
                        }
                    }
                }
            }

            for prev_activity in activities.iter().rev().skip(1).take(5) {
                let prev_key = format!(
                    "{}:{}",
                    prev_activity.item_type as u8, prev_activity.item_id
                );
                if let Some(mut stats) = self.item_stats.get_mut(&prev_key) {
                    let curr_key = format!("{}:{}", activity.item_type as u8, activity.item_id);
                    *stats
                        .usage_pattern
                        .frequent_cooccurrences
                        .entry(curr_key)
                        .or_insert(0) += 1;
                }
            }
        }
    }

    fn trigger_preload_check(&self, activity: &UserActivity) {
        let config = self.config.lock().unwrap();
        if !config.enabled {
            return;
        }
        let threshold = config.preload_threshold;
        drop(config);

        let predictions = self.generate_predictions(&activity.user_id);

        for prediction in predictions {
            if prediction.confidence >= threshold {
                let _ = self.preload_item(&prediction);
            }
        }
    }

    pub fn generate_predictions(&self, user_id: &str) -> Vec<PreloadPrediction> {
        let now = chrono::Utc::now();
        let cache_ttl = Duration::from_secs(300);

        if let Some(cache_entry) = self.prediction_cache.get(user_id) {
            if (now - cache_entry.cached_at).to_std().unwrap_or_default() < cache_ttl {
                debug!("Using cached predictions for user: {}", user_id);
                return cache_entry.predictions.clone();
            }
        }

        let mut predictions = Vec::new();
        let current_hour = now.hour() as u8;
        let _current_weekday = now.weekday() as u8;

        if let Some(user_activities) = self.user_activities.get(user_id) {
            let activities = user_activities.value();

            for activity in activities.iter().rev().take(100) {
                let item_key = format!("{}:{}", activity.item_type as u8, activity.item_id);
                if let Some(stats) = self.item_stats.get(&item_key) {
                    let recent_use = stats
                        .last_used
                        .map(|t| (now - t).num_minutes() < 60)
                        .unwrap_or(false);

                    if recent_use {
                        predictions.push(PreloadPrediction {
                            item_id: activity.item_id.clone(),
                            item_type: activity.item_type,
                            confidence: 0.9,
                            predicted_time: None,
                            reason: PreloadReason::RecentUsage,
                        });
                    }

                    let hour_count = stats
                        .time_of_day_distribution
                        .get(&current_hour)
                        .copied()
                        .unwrap_or(0);
                    let total_hours: u64 = stats.time_of_day_distribution.values().sum();
                    if total_hours > 0 && hour_count as f64 / total_hours as f64 > 0.3 {
                        predictions.push(PreloadPrediction {
                            item_id: activity.item_id.clone(),
                            item_type: activity.item_type,
                            confidence: 0.7,
                            predicted_time: None,
                            reason: PreloadReason::PeakTime,
                        });
                    }

                    for (cooccur_item, count) in &stats.usage_pattern.frequent_cooccurrences {
                        if *count >= 3 {
                            let parts: Vec<&str> = cooccur_item.split(':').collect();
                            if parts.len() == 2 {
                                if let Ok(type_num) = parts[0].parse::<u8>() {
                                    if let Some(item_type) = PreloadableType::from_u8(type_num) {
                                        predictions.push(PreloadPrediction {
                                            item_id: parts[1].to_string(),
                                            item_type,
                                            confidence: 0.6,
                                            predicted_time: None,
                                            reason: PreloadReason::Cooccurrence,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        predictions.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        predictions.dedup_by(|a, b| a.item_id == b.item_id && a.item_type == b.item_type);

        let cache_entry = PredictionCacheEntry {
            predictions: predictions.clone(),
            cached_at: now,
        };
        self.prediction_cache
            .insert(user_id.to_string(), cache_entry);

        predictions
    }

    fn preload_item(&self, prediction: &PreloadPrediction) -> Result<()> {
        let item_key = format!("{}:{}", prediction.item_type as u8, prediction.item_id);

        if self.preloaded_items.contains_key(&item_key) {
            return Ok(());
        }

        let max_items = self.config.lock().unwrap().max_preloaded_items;

        if self.preloaded_items.len() >= max_items {
            self.evict_lowest_confidence()?;
        }

        info!(
            "Preloading item: {} ({:?}) with confidence: {}",
            prediction.item_id, prediction.item_type, prediction.confidence
        );

        let preloaded = PreloadedItem {
            item_id: prediction.item_id.clone(),
            item_type: prediction.item_type,
            preloaded_at: chrono::Utc::now(),
            preload_confidence: prediction.confidence,
            preload_reason: prediction.reason.clone(),
            used: false,
            used_at: None,
        };

        self.preloaded_items.insert(item_key, preloaded);

        self.save()?;

        Ok(())
    }

    fn evict_lowest_confidence(&self) -> Result<()> {
        let mut items: Vec<_> = self.preloaded_items.iter().collect();
        items.sort_by(|a, b| {
            a.value()
                .preload_confidence
                .partial_cmp(&b.value().preload_confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if let Some(item) = items.first() {
            self.preloaded_items.remove(item.key());
        }

        Ok(())
    }

    pub fn mark_as_used(&self, item_type: PreloadableType, item_id: &str) -> Result<()> {
        let item_key = format!("{}:{}", item_type as u8, item_id);

        if let Some(mut preloaded) = self.preloaded_items.get_mut(&item_key) {
            preloaded.used = true;
            preloaded.used_at = Some(chrono::Utc::now());
            debug!(
                "Marked preloaded item as used: {} ({:?})",
                item_id, item_type
            );
        }

        self.save()?;

        Ok(())
    }

    pub fn is_preloaded(&self, item_type: PreloadableType, item_id: &str) -> bool {
        let item_key = format!("{}:{}", item_type as u8, item_id);
        self.preloaded_items.contains_key(&item_key)
    }

    pub fn get_preloaded_items(&self) -> Vec<PreloadedItem> {
        self.preloaded_items
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    fn cleanup_old_data(&self) -> Result<()> {
        let mut last_cleanup = self.last_cleanup.lock().unwrap();
        let config = self.config.lock().unwrap();
        if last_cleanup.elapsed() < config.cleanup_interval {
            return Ok(());
        }

        *last_cleanup = Instant::now();
        let lookahead_window = config.lookahead_window;
        let usage_history_window = config.usage_history_window;
        drop(config);

        info!("Cleaning up old preload data");

        let cutoff = chrono::Utc::now() - usage_history_window;

        for mut entry in self.user_activities.iter_mut() {
            entry.value_mut().retain(|a| a.timestamp >= cutoff);
        }

        let now = chrono::Utc::now();
        self.preloaded_items.retain(|_, item| {
            (now - item.preloaded_at).to_std().unwrap_or_default() < lookahead_window
        });

        let cache_ttl = Duration::from_secs(3600);
        self.prediction_cache
            .retain(|_, entry| (now - entry.cached_at).to_std().unwrap_or_default() < cache_ttl);

        Ok(())
    }

    pub fn get_preload_stats(&self) -> PreloadStats {
        let preloaded = self.get_preloaded_items();
        let used_count = preloaded.iter().filter(|i| i.used).count();
        let total_count = preloaded.len();
        let hit_rate = if total_count > 0 {
            used_count as f32 / total_count as f32
        } else {
            0.0
        };

        PreloadStats {
            total_preloaded: total_count,
            used_count,
            hit_rate,
            average_confidence: if total_count > 0 {
                preloaded.iter().map(|i| i.preload_confidence).sum::<f32>() / total_count as f32
            } else {
                0.0
            },
        }
    }

    fn adapt_strategy(&self) -> Result<()> {
        let stats = self.get_preload_stats();

        let mut config = self.config.lock().unwrap();

        const TARGET_HIT_RATE: f32 = 0.7;
        const MIN_THRESHOLD: f32 = 0.3;
        const MAX_THRESHOLD: f32 = 0.9;
        const MIN_MAX_ITEMS: usize = 10;
        const MAX_MAX_ITEMS: usize = 50;

        if stats.total_preloaded >= 10 {
            if stats.hit_rate < TARGET_HIT_RATE - 0.1 {
                config.preload_threshold = (config.preload_threshold + 0.05).min(MAX_THRESHOLD);
                config.max_preloaded_items =
                    (config.max_preloaded_items.saturating_sub(2)).max(MIN_MAX_ITEMS);
                info!(
                    "Increasing preload threshold to {} and decreasing max items to {} due to low hit rate: {}",
                    config.preload_threshold, config.max_preloaded_items, stats.hit_rate
                );
            } else if stats.hit_rate > TARGET_HIT_RATE + 0.1 {
                config.preload_threshold = (config.preload_threshold - 0.05).max(MIN_THRESHOLD);
                config.max_preloaded_items = (config.max_preloaded_items + 2).min(MAX_MAX_ITEMS);
                info!(
                    "Decreasing preload threshold to {} and increasing max items to {} due to high hit rate: {}",
                    config.preload_threshold, config.max_preloaded_items, stats.hit_rate
                );
            }
        }

        Ok(())
    }
}

impl Default for SmartPreloader {
    fn default() -> Self {
        let storage_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".aetheris")
            .join("smart-preloader");

        Self::new(PreloadConfig::default(), storage_path).unwrap_or_else(|_| {
            let temp_dir = tempfile::tempdir().unwrap();
            Self::new(PreloadConfig::default(), temp_dir.path().to_path_buf()).unwrap()
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreloadStats {
    pub total_preloaded: usize,
    pub used_count: usize,
    pub hit_rate: f32,
    pub average_confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_preloader_new() {
        let preloader = SmartPreloader::default();
        assert!(preloader.config.lock().unwrap().enabled);
    }

    #[test]
    fn test_record_activity() {
        let preloader = SmartPreloader::default();

        let activity = UserActivity {
            user_id: "test-user".to_string(),
            item_id: "test-skill".to_string(),
            item_type: PreloadableType::Skill,
            activity_type: ActivityType::Use,
            timestamp: chrono::Utc::now(),
            session_id: "session-1".to_string(),
            context: HashMap::new(),
        };

        let result = preloader.record_activity(activity);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_predictions() {
        let preloader = SmartPreloader::default();

        let activity = UserActivity {
            user_id: "test-user".to_string(),
            item_id: "test-skill".to_string(),
            item_type: PreloadableType::Skill,
            activity_type: ActivityType::Use,
            timestamp: chrono::Utc::now(),
            session_id: "session-1".to_string(),
            context: HashMap::new(),
        };

        preloader.record_activity(activity).unwrap();

        let predictions = preloader.generate_predictions("test-user");
        assert!(!predictions.is_empty());
    }

    #[test]
    fn test_preload_stats() {
        let preloader = SmartPreloader::default();

        let stats = preloader.get_preload_stats();
        assert_eq!(stats.total_preloaded, 0);
        assert_eq!(stats.hit_rate, 0.0);
    }
}
