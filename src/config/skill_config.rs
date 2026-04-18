use crate::skill::{CacheConfig, MatchConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct SkillConfig {
    pub enable_progressive_disclosure: bool,
    pub enable_enterprise_features: bool,
    pub skill_cache_config: CacheConfig,
    pub match_config: MatchConfig,
}


impl SkillConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_progressive_disclosure(mut self, enabled: bool) -> Self {
        self.enable_progressive_disclosure = enabled;
        self
    }

    pub fn with_enterprise_features(mut self, enabled: bool) -> Self {
        self.enable_enterprise_features = enabled;
        self
    }

    pub fn with_skill_cache_config(mut self, config: CacheConfig) -> Self {
        self.skill_cache_config = config;
        self
    }

    pub fn with_match_config(mut self, config: MatchConfig) -> Self {
        self.match_config = config;
        self
    }
}
