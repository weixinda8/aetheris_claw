#![allow(dead_code)]
#![allow(unused)]

pub mod agent;
pub mod ai;
pub mod api;
pub mod cli;
pub mod cluster;
pub mod config;
pub mod constants;
pub mod core;
pub mod data_governance;
pub mod digital_twin;
pub mod edge;
pub mod edge_build;
pub mod edge_coordination;
pub mod memory;
pub mod observability;
pub mod protocol;
pub mod runtime;
pub mod security;
pub mod skill;
pub mod soul;
pub mod storage;
pub mod streaming;
#[cfg(test)]
pub mod test_utils;
pub mod utils;

pub use data_governance::{
    CheckStatus, ClassificationManager, ClassificationResult, ClassificationStrategy,
    ClassificationTag, ComplianceCheck, ComplianceReport, ComplianceReportGenerator,
    ComplianceStandard, ContentBasedClassifier, DataClassification, DataClassifier, DataLineage,
    DataMasker, DynamicMasker, LineageCollector, LineageEdge, LineageEdgeId, LineageEdgeType,
    LineageGraphBuilder, LineageNode, LineageNodeId, LineageNodeType, LineageQueryEngine,
    LineageRecord, LineageStore, LineageTracker, LineageVisualizationData, MaskingAlgorithm,
    MaskingConfig, MaskingException, MaskingManager, MaskingResult, MaskingRule,
    MetadataBasedClassifier, ReportFormat, ReportTemplate, ReviewStatus, ReviewTask,
    ScheduledReport, StaticMasker,
};
pub use skill::{
    AgentSkillExample, AgentSkillManifest, AgentSkillMetadata, AgentSkillParameter,
    AgentSkillRetryConfig, AgentSkillReturn, AgentSkillType, AgentSkillsRegistry, BaseSkill,
    CallMode, PermissionLevel, Skill, SkillEvaluation, SkillMdDocument, SkillMdFrontmatter,
    SkillMdSections, SkillMetadata, SkillPriority, SkillRegistry, Version,
};
pub use utils::{AetherisError, Result};

pub use digital_twin::{
    CommandStatus, DigitalTwin, DigitalTwinSimulator, DigitalTwinSynchronizer, EntityModification,
    SimulationConfig, SimulationMetrics, SimulationMode, SimulationResult, SyncConfig,
    SyncDirection, SyncStats, TwinCommand, TwinEntity, TwinEntityType, TwinModel, TwinState,
    TwinStateUpdate, TwinVisualizationData, VisualizationConnection, VisualizationEntity,
    WhatIfScenario,
};
pub use edge::{
    CompressionLevel, DataAggregator, DataCompressor, DataFilter, EdgeData, EdgeFilterPipeline,
    FilterConfig, FilterStrategy, OutlierDetector,
};
pub use edge_build::{
    AlertLevel, AlertStats, CacheEntry, CacheEvictionPolicy, CacheStats, ConnectionStatus,
    DegradationLevel, EdgeAetherisConfig, EdgeCacheManager, EdgeFeature, EdgeProfile, FeatureFlag,
    LocalAlert, LocalAlertManager, LocalRuleEngine, LogLevel, OfflineDataRecord,
    OfflineModeManager, ResourceAlert, ResourceAlertSeverity, ResourceMonitor, ResourceUsage, Rule,
    RuleAction, RuleCondition, ThresholdOperator,
};
pub use edge_coordination::{
    DeploymentStatus, EdgeNode, GlobalCoordinator, ModelDeployment, NodeResourceUsage, NodeStatus,
    NodeType, SyncStrategy,
};

pub mod prelude {
    pub use crate::agent::{AgentConfig, AgentRegistry, AgentType, BaseAgent};
    pub use crate::ai::{
        ABTest, ABTestManager, ABTestResult, ABTestStatus, ARIMAForecaster, AdaptiveLearner,
        Anomaly, AnomalyDetectionMethod, AnomalyDetector, AnomalyVisualizationData, AutoForecaster,
        AutoRollbackManager, AutoencoderDetector, CacheStats, CaseLibrary, CloudInferenceEngine,
        ConfidenceEstimator, ConfidenceInterval, DirRecMultiStepStrategy, DirectMultiStepStrategy,
        DriftDetectionResult, DriftDetector, ETSForecaster, Entity, EntityType,
        EntityTypeDefinition, FeatureExtractor, Feedback, FeedbackManager, FeedbackStats,
        FeedbackType, Forecast, ForecastingMethod, GraphVisualizationData, InMemoryKnowledgeGraph,
        IndustrialKnowledgeGraph, IndustrialOntology, InferenceCache, InferenceEngine,
        InferenceInput, InferenceMetrics, InferenceMetricsData, InferenceOutput,
        IsolationForestDetector, KnowledgeGraph, LOFDetector, LSTMForecaster, LearningConfig,
        LightGBMForecaster, LocalInferenceEngine, MaintenanceCase, Model, ModelFormat,
        ModelPerformanceMonitor, ModelRegistry, ModelSelectionCriteria, ModelVersion,
        ModelVersionManager, MonitoringConfig, MonteCarloDropoutEstimator, MultiStepForecast,
        MultiStepStrategy, OnlineLearner, Ontology, PerformanceMetrics,
        QuantileRegressionEstimator, RecursiveMultiStepStrategy, Relationship, RelationshipType,
        RelationshipTypeDefinition, RollbackEvent, RollbackPolicy, RollbackTrigger,
        SemanticSearchEngine, Statistical3SigmaDetector, StatisticalIQRDetector,
        StreamingFeatureExtractor, TimeSeriesForecaster, TransformerForecaster, VersionComparison,
        VersionStats, XGBoostForecaster,
    };
    pub use crate::api::{AppState, AppStateBuilder, create_router};
    pub use crate::core::performance::{
        AlertSeverity, AlertType, BenchmarkResult, HotSpot, HotSpotSeverity, MetricType,
        OptimizationResult, OptimizationStrategy, OptimizationType, PerformanceAlert,
        PerformanceMetric, PerformanceOptimizer, SystemStats, UserExperienceMetric, UxMetricType,
    };
    pub use crate::core::{
        CommanderCore, ExecutionContext, Task, TaskExecutor, TaskStatus,
        PlanAndExecuteEngine, ReActEngine, PlanExecuteState, PlanExecuteStatus,
        PlanExecuteResult, ReActStep, ReActStepType,
    };
    pub use crate::data_governance::{
        CheckStatus, ClassificationManager, ClassificationResult, ClassificationStrategy,
        ClassificationTag, ComplianceCheck, ComplianceReport, ComplianceReportGenerator,
        ComplianceStandard, ContentBasedClassifier, DataClassification, DataClassifier,
        DataLineage, DataMasker, DynamicMasker, LineageCollector, LineageEdge, LineageEdgeId,
        LineageEdgeType, LineageGraphBuilder, LineageNode, LineageNodeId, LineageNodeType,
        LineageQueryEngine, LineageRecord, LineageStore, LineageTracker, LineageVisualizationData,
        MaskingAlgorithm, MaskingConfig, MaskingException, MaskingManager, MaskingResult,
        MaskingRule, MetadataBasedClassifier, ReportFormat, ReportTemplate, ReviewStatus,
        ReviewTask, ScheduledReport, StaticMasker,
    };
    pub use crate::edge::{
        CompressionLevel, DataAggregator, DataCompressor, DataFilter, EdgeData, EdgeFilterPipeline,
        FilterConfig, FilterStrategy, OutlierDetector,
    };
    pub use crate::memory::ShortTermMemory;
    pub use crate::observability::{
        OpenTelemetryConfig, OtlpConfig, PrometheusConfig,
    };
    pub use crate::security::{
        SecurityManager,
        sandbox::{
            AnomalyAlert, AnomalyType, ResourceLimits, SandboxAuditLog, SandboxConfig,
            SandboxManager, SandboxSecurityLevel,
        },
    };
    pub use crate::skill::{
        AgentSkillsRegistry, BaseSkill, CallMode, PermissionLevel, Skill, SkillMdDocument,
        SkillMdFrontmatter, SkillMdSections, SkillMetadata, SkillPriority, SkillRegistry, Version,
    };
    pub use crate::soul::{
        Soul, SoulRegistry,
        marketplace::{
            PersonaMarketplace, PersonaMetadata, PersonaRating, PersonaReview, PersonaVersion,
        },
    };
    pub use crate::utils::{AetherisError, Result};
}
