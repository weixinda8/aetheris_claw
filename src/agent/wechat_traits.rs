//! WeChat 处理器 Trait 接口
//! 
//! 提供抽象接口以提高可测试性和可维护性，
//! 支持依赖注入和 Mock 实现。

use crate::agent::config::webhook::{WebhookMessage, WebhookError};
use crate::core::ExecutionContext;
use crate::core::intent::{Intent, ValidationResult};
use crate::core::planner::ExecutionPlan;
use async_trait::async_trait;

/// CommanderCore 抽象 Trait
/// 
/// 用于解耦 WeChatMessageHandler 对具体 CommanderCore 的依赖，
/// 便于单元测试和 Mock。
#[async_trait]
pub trait CommanderCoreTrait: Send + Sync + std::fmt::Debug {
    /// 处理意图
    async fn process_intent(
        &self,
        input: &str,
    ) -> Result<(Intent, ValidationResult), Box<dyn std::error::Error + Send + Sync>>;

    /// 从意图创建计划
    async fn create_plan_from_intent(
        &self,
        intent: Intent,
    ) -> Result<ExecutionPlan, Box<dyn std::error::Error + Send + Sync>>;

    /// 执行计划
    async fn execute_plan(
        &self,
        plan: ExecutionPlan,
    ) -> Result<ExecutionContext, Box<dyn std::error::Error + Send + Sync>>;
}

/// 响应发送器 Trait
/// 
/// 抽象响应发送功能，便于 Mock 和测试。
#[async_trait]
pub trait ResponseSender: Send + Sync + std::fmt::Debug {
    /// 发送响应
    async fn send_response(
        &self,
        message: WebhookMessage,
        response: String,
    ) -> Result<(), WebhookError>;
}

/// 模式管理 Trait
/// 
/// 抽象处理器模式管理功能。
#[async_trait]
pub trait ModeManager: Send + Sync + std::fmt::Debug {
    /// 获取当前模式
    async fn get_mode(&self) -> HandlerMode;
    
    /// 设置模式
    async fn set_mode(&self, mode: HandlerMode);
}

/// 失败计数器 Trait
/// 
/// 抽象失败计数功能。
#[async_trait]
pub trait FailureCounter: Send + Sync + std::fmt::Debug {
    /// 获取失败计数
    async fn get_failure_count(&self) -> u32;
    
    /// 重置失败计数
    async fn reset_failure_count(&self);
    
    /// 记录失败
    async fn record_failure(&self);
}

/// 处理器模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HandlerMode {
    /// 完整功能模式
    #[default]
    Full,
    /// 简单回复模式
    SimpleReply,
    /// 离线模式
    Offline,
}

#[cfg(test)]
pub mod mocks {
    //! Mock 实现模块
    //! 
    //! 提供各种 Trait 的 Mock 实现，用于单元测试。

    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Mock CommanderCore
    #[derive(Debug, Clone, Default)]
    pub struct MockCommanderCore {
        /// 是否应该返回错误
        pub should_error: Arc<RwLock<bool>>,
        /// 返回的意图 ID
        pub intent_id: Arc<RwLock<String>>,
        /// 返回的计划 ID
        pub plan_id: Arc<RwLock<String>>,
        /// 调用计数
        pub call_count: Arc<RwLock<u32>>,
    }

    impl MockCommanderCore {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_should_error(self, should_error: bool) -> Self {
            *self.should_error.blocking_write() = should_error;
            self
        }

        pub fn with_intent_id(self, intent_id: &str) -> Self {
            *self.intent_id.blocking_write() = intent_id.to_string();
            self
        }

        pub fn with_plan_id(self, plan_id: &str) -> Self {
            *self.plan_id.blocking_write() = plan_id.to_string();
            self
        }

        pub async fn get_call_count(&self) -> u32 {
            *self.call_count.read().await
        }
    }

    #[async_trait]
    impl CommanderCoreTrait for MockCommanderCore {
        async fn process_intent(
            &self,
            _input: &str,
        ) -> Result<(Intent, ValidationResult), Box<dyn std::error::Error + Send + Sync>> {
            *self.call_count.write().await += 1;

            if *self.should_error.read().await {
                return Err("Mock error".into());
            }

            let intent_id = self.intent_id.read().await.clone();
            let intent = Intent::new("Mock input".to_string())
                .with_parsed_goal("Mock goal".to_string())
                .with_confidence(crate::core::intent::IntentConfidence::High);

            let validation = ValidationResult::valid();

            Ok((intent, validation))
        }

        async fn create_plan_from_intent(
            &self,
            _intent: Intent,
        ) -> Result<ExecutionPlan, Box<dyn std::error::Error + Send + Sync>> {
            *self.call_count.write().await += 1;

            if *self.should_error.read().await {
                return Err("Mock error".into());
            }

            let plan_id = self.plan_id.read().await.clone();
            let root_task_id = "mock-root-task".to_string();
            let mut plan = ExecutionPlan::new(root_task_id);
            if !plan_id.is_empty() {
                plan.plan_id = plan_id;
            }

            Ok(plan)
        }

        async fn execute_plan(
            &self,
            _plan: ExecutionPlan,
        ) -> Result<ExecutionContext, Box<dyn std::error::Error + Send + Sync>> {
            *self.call_count.write().await += 1;

            if *self.should_error.read().await {
                return Err("Mock error".into());
            }

            let context = ExecutionContext::new();

            Ok(context)
        }
    }

    /// Mock ResponseSender
    #[derive(Debug, Clone, Default)]
    pub struct MockResponseSender {
        /// 发送的响应
        pub sent_responses: Arc<RwLock<Vec<(WebhookMessage, String)>>>,
        /// 是否应该返回错误
        pub should_error: Arc<RwLock<bool>>,
    }

    impl MockResponseSender {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_should_error(self, should_error: bool) -> Self {
            *self.should_error.blocking_write() = should_error;
            self
        }

        pub async fn get_sent_responses(&self) -> Vec<(WebhookMessage, String)> {
            self.sent_responses.read().await.clone()
        }

        pub async fn get_sent_count(&self) -> usize {
            self.sent_responses.read().await.len()
        }
    }

    #[async_trait]
    impl ResponseSender for MockResponseSender {
        async fn send_response(
            &self,
            message: WebhookMessage,
            response: String,
        ) -> Result<(), WebhookError> {
            if *self.should_error.read().await {
                return Err(WebhookError::Http("Mock error".to_string()));
            }

            self.sent_responses
                .write()
                .await
                .push((message, response));

            Ok(())
        }
    }

    /// Mock ModeManager
    #[derive(Debug, Clone, Default)]
    pub struct MockModeManager {
        pub current_mode: Arc<RwLock<HandlerMode>>,
        pub mode_changes: Arc<RwLock<Vec<HandlerMode>>>,
    }

    impl MockModeManager {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_initial_mode(self, mode: HandlerMode) -> Self {
            *self.current_mode.blocking_write() = mode;
            self
        }

        pub async fn get_mode_changes(&self) -> Vec<HandlerMode> {
            self.mode_changes.read().await.clone()
        }
    }

    #[async_trait]
    impl ModeManager for MockModeManager {
        async fn get_mode(&self) -> HandlerMode {
            *self.current_mode.read().await
        }

        async fn set_mode(&self, mode: HandlerMode) {
            self.mode_changes.write().await.push(mode);
            *self.current_mode.write().await = mode;
        }
    }

    /// Mock FailureCounter
    #[derive(Debug, Clone, Default)]
    pub struct MockFailureCounter {
        pub failure_count: Arc<RwLock<u32>>,
        pub record_calls: Arc<RwLock<u32>>,
        pub reset_calls: Arc<RwLock<u32>>,
    }

    impl MockFailureCounter {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_initial_count(self, count: u32) -> Self {
            *self.failure_count.blocking_write() = count;
            self
        }

        pub async fn get_record_calls(&self) -> u32 {
            *self.record_calls.read().await
        }

        pub async fn get_reset_calls(&self) -> u32 {
            *self.reset_calls.read().await
        }
    }

    #[async_trait]
    impl FailureCounter for MockFailureCounter {
        async fn get_failure_count(&self) -> u32 {
            *self.failure_count.read().await
        }

        async fn reset_failure_count(&self) {
            *self.reset_calls.write().await += 1;
            *self.failure_count.write().await = 0;
        }

        async fn record_failure(&self) {
            *self.record_calls.write().await += 1;
            *self.failure_count.write().await += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mocks::*;

    #[tokio::test]
    async fn test_mock_commander_core() {
        let mock_core = MockCommanderCore::new();
        
        let (intent, validation) = mock_core.process_intent("test").await.unwrap();
        assert!(!intent.intent_id.is_empty());
        assert!(validation.is_valid);
        assert_eq!(mock_core.get_call_count().await, 1);
    }

    #[tokio::test]
    async fn test_mock_commander_core_error() {
        let mock_core = MockCommanderCore::new().with_should_error(true);
        
        let result = mock_core.process_intent("test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_response_sender() {
        let mock_sender = MockResponseSender::new();
        
        let message = create_test_message();
        mock_sender.send_response(message, "test response".to_string()).await.unwrap();
        
        assert_eq!(mock_sender.get_sent_count().await, 1);
    }

    #[tokio::test]
    async fn test_mock_mode_manager() {
        let mock_manager = MockModeManager::new();
        
        assert_eq!(mock_manager.get_mode().await, HandlerMode::Full);
        
        mock_manager.set_mode(HandlerMode::SimpleReply).await;
        assert_eq!(mock_manager.get_mode().await, HandlerMode::SimpleReply);
        
        let changes = mock_manager.get_mode_changes().await;
        assert_eq!(changes.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_failure_counter() {
        let mock_counter = MockFailureCounter::new();
        
        assert_eq!(mock_counter.get_failure_count().await, 0);
        
        mock_counter.record_failure().await;
        mock_counter.record_failure().await;
        assert_eq!(mock_counter.get_failure_count().await, 2);
        assert_eq!(mock_counter.get_record_calls().await, 2);
        
        mock_counter.reset_failure_count().await;
        assert_eq!(mock_counter.get_failure_count().await, 0);
        assert_eq!(mock_counter.get_reset_calls().await, 1);
    }

    fn create_test_message() -> WebhookMessage {
        WebhookMessage {
            message_id: "test-msg-001".to_string(),
            from_platform: crate::agent::config::webhook::IMPlatform::WeChat,
            from_user: "test-user".to_string(),
            to_agent: "test-agent".to_string(),
            content: "测试消息".to_string(),
            timestamp: 1234567890,
            metadata: None,
        }
    }
}
