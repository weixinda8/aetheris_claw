pub mod alerts;
pub mod cache;
pub mod offline;
pub mod resource_monitor;
pub mod rules;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[derive(Default)]
pub enum EdgeProfile {
    Core,
    Standard,
    #[default]
    Full,
}

impl EdgeProfile {
    pub fn enabled_features(&self) -> HashSet<FeatureFlag> {
        match self {
            EdgeProfile::Core => {
                let mut features = HashSet::new();
                features.insert(FeatureFlag::EdgeDataFiltering);
                features.insert(FeatureFlag::LocalAlerts);
                features.insert(FeatureFlag::OfflineCache);
                features.insert(FeatureFlag::ResourceMonitoring);
                features
            }
            EdgeProfile::Standard => {
                let mut features = Self::Core.enabled_features();
                features.insert(FeatureFlag::LocalRulesEngine);
                features.insert(FeatureFlag::LocalAgent);
                features
            }
            EdgeProfile::Full => {
                let mut features = Self::Standard.enabled_features();
                features.insert(FeatureFlag::FullLLM);
                features.insert(FeatureFlag::MultiAgent);
                features.insert(FeatureFlag::KnowledgeGraph);
                features
            }
        }
    }
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FeatureFlag {
    EdgeDataFiltering,
    LocalAlerts,
    OfflineCache,
    ResourceMonitoring,
    LocalRulesEngine,
    LocalAgent,
    FullLLM,
    MultiAgent,
    KnowledgeGraph,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAetherisConfig {
    pub profile: EdgeProfile,
    pub max_memory_mb: usize,
    pub max_cpu_cores: f64,
    pub offline_enabled: bool,
    pub cache_ttl_seconds: u64,
    pub auto_downgrade: bool,
}

impl Default for EdgeAetherisConfig {
    fn default() -> Self {
        Self {
            profile: EdgeProfile::default(),
            max_memory_mb: 256,
            max_cpu_cores: 1.0,
            offline_enabled: true,
            cache_ttl_seconds: 3600,
            auto_downgrade: true,
        }
    }
}

#[async_trait]
pub trait EdgeFeature: Send + Sync {
    fn name(&self) -> &str;
    fn required_profile(&self) -> EdgeProfile;
    async fn is_enabled(&self, config: &EdgeAetherisConfig) -> bool {
        config
            .profile
            .enabled_features()
            .contains(&self.required_feature())
    }
    fn required_feature(&self) -> FeatureFlag;
}

pub use alerts::{LocalAlert, LocalAlertManager};
pub use cache::{CacheEntry, CacheEvictionPolicy, EdgeCacheManager};
pub use offline::{ConnectionStatus, OfflineDataRecord, OfflineModeManager};
pub use resource_monitor::{DegradationLevel, ResourceMonitor, ResourceUsage};
pub use rules::{
    AlertLevel, LocalRuleEngine, LogLevel, Rule, RuleAction, RuleCondition, ThresholdOperator,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertStats;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceAlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAlert;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_profile_default() {
        let profile = EdgeProfile::default();
        assert_eq!(profile, EdgeProfile::Full);
    }

    #[test]
    fn test_edge_profile_core_features() {
        let features = EdgeProfile::Core.enabled_features();
        assert!(features.contains(&FeatureFlag::EdgeDataFiltering));
        assert!(features.contains(&FeatureFlag::LocalAlerts));
        assert!(features.contains(&FeatureFlag::OfflineCache));
        assert!(features.contains(&FeatureFlag::ResourceMonitoring));
        assert!(!features.contains(&FeatureFlag::LocalRulesEngine));
    }

    #[test]
    fn test_edge_profile_standard_features() {
        let features = EdgeProfile::Standard.enabled_features();
        assert!(features.contains(&FeatureFlag::EdgeDataFiltering));
        assert!(features.contains(&FeatureFlag::LocalAlerts));
        assert!(features.contains(&FeatureFlag::OfflineCache));
        assert!(features.contains(&FeatureFlag::ResourceMonitoring));
        assert!(features.contains(&FeatureFlag::LocalRulesEngine));
        assert!(features.contains(&FeatureFlag::LocalAgent));
        assert!(!features.contains(&FeatureFlag::FullLLM));
    }

    #[test]
    fn test_edge_profile_full_features() {
        let features = EdgeProfile::Full.enabled_features();
        assert!(features.contains(&FeatureFlag::EdgeDataFiltering));
        assert!(features.contains(&FeatureFlag::LocalAlerts));
        assert!(features.contains(&FeatureFlag::OfflineCache));
        assert!(features.contains(&FeatureFlag::ResourceMonitoring));
        assert!(features.contains(&FeatureFlag::LocalRulesEngine));
        assert!(features.contains(&FeatureFlag::LocalAgent));
        assert!(features.contains(&FeatureFlag::FullLLM));
        assert!(features.contains(&FeatureFlag::MultiAgent));
        assert!(features.contains(&FeatureFlag::KnowledgeGraph));
    }

    #[test]
    fn test_edge_aetheris_config_default() {
        let config = EdgeAetherisConfig::default();
        assert_eq!(config.profile, EdgeProfile::Full);
        assert_eq!(config.max_memory_mb, 256);
        assert_eq!(config.max_cpu_cores, 1.0);
        assert!(config.offline_enabled);
        assert_eq!(config.cache_ttl_seconds, 3600);
        assert!(config.auto_downgrade);
    }

    #[test]
    fn test_feature_flag_equality() {
        let flag1 = FeatureFlag::EdgeDataFiltering;
        let flag2 = FeatureFlag::EdgeDataFiltering;
        let flag3 = FeatureFlag::LocalAlerts;

        assert_eq!(flag1, flag2);
        assert_ne!(flag1, flag3);
    }

    #[test]
    fn test_edge_profile_equality() {
        let profile1 = EdgeProfile::Core;
        let profile2 = EdgeProfile::Core;
        let profile3 = EdgeProfile::Standard;

        assert_eq!(profile1, profile2);
        assert_ne!(profile1, profile3);
    }
}
