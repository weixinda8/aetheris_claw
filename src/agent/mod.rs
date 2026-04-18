pub mod base;
pub mod code;
pub mod communication;
pub mod compliance;
pub mod config;
pub mod config_driven;
pub mod data;
pub mod factory;
pub mod industrial;
pub mod matcher;
pub mod office;
pub mod ops;
pub mod task_decomposer;
pub mod wechat_config;
pub mod wechat_deduplication;
pub mod wechat_handler;
pub mod wechat_traits;

pub use base::{
    Agent, AgentCapabilities, AgentConfig, AgentMessage, AgentRegistry, AgentState, AgentStatus,
    AgentType, BaseAgent,
};
pub use code::CodeAgent;
pub use communication::{
    AcknowledgementType, AgentCommunicationBus, BroadcastChannel, CommunicationBus, Message,
    MessageError, MessageHeader, MessageQueue, MessageQueueEntry, MessageType,
    MessageValidationResult, PointToPointChannel, QueuePriority, ReliableMessageChannel,
    RetryConfig,
};
pub use compliance::ComplianceAgent;
pub use config::config::{
    AgentConfig as ConfigurableAgentConfig, AgentConfigError, AgentDefaults, AgentMeta,
    AgentPersona, ChannelsConfig, ConfigFormat, ConsulStorageConfig, DingTalkConfig,
    EtcdStorageConfig, FeishuConfig, GlobalAgentConfig, HumanInterveneConfig, LocalStorageConfig,
    MemoryConfig, MidTermMemoryConfig, ModelConfig, ModelParams, PersonalityConfig, SandboxConfig,
    SandboxMode, SchedulerConfig, SecurityConfig, ShortTermMemoryConfig, SkillPriority,
    SkillsConfig, StorageBackend, StorageConfig, WeChatConfig, WeComConfig,
};
pub use config::loader::AgentConfigLoader;
pub use config::template::{
    AgentTemplate, AgentTemplateEngine, TemplateError, TemplateVariable, VariableType,
    create_default_templates,
};
pub use config_driven::ConfigDrivenAgent;
pub use data::DataAgent;
pub use factory::{AgentFactory, AgentFactoryError};
pub use industrial::IndustrialAgent;
pub use matcher::{
    AgentMatch, AgentMatcher, AgentProfile, FallbackStrategy, MatchResult, SmartAgentMatcher,
    TaskRequirement,
};
pub use office::OfficeAgent;
pub use ops::OpsAgent;
pub use task_decomposer::{
    AgentCapabilityMatch, AgentInfo, DecomposedTask, DecompositionOptions, DecompositionTemplate,
    DecompositionValidationResult, LlmTaskDecomposer, SubTask, SubTaskPattern, TaskComplexity,
    TaskDecomposer,
};
pub use wechat_config::{
    ConfidenceThreshold, DeduplicationConfig, GracefulDegradationConfig, HandlerMode,
    WeChatHandlerConfig,
};
pub use wechat_deduplication::MessageDeduplicator;
pub use wechat_handler::WeChatMessageHandler;
pub use wechat_traits::{
    CommanderCoreTrait, FailureCounter, ModeManager, ResponseSender,
};
