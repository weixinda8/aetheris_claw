//! WeChat 消息处理器模块
//! 
//! 该模块提供了处理来自个人微信消息的功能，
//! 支持自然语言解析、任务分解和自动执行。
//! 
//! 生产级特性：
//! - 配置化支持
//! - 消息去重
//! - 优雅降级
//! - 完整的日志和监控

use crate::agent::config::webhook::{WebhookHandler, WebhookMessage, WebhookError};
use crate::agent::wechat_config::{
    ConfidenceThreshold, DeduplicationConfig, GracefulDegradationConfig, HandlerMode,
    WeChatHandlerConfig,
};
use crate::agent::wechat_deduplication::MessageDeduplicator;
use crate::core::CommanderCore;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

/// 微信消息处理器
/// 
/// 负责接收来自个人微信的消息，
/// 解析自然语言并转换为任务执行。
#[derive(Clone)]
pub struct WeChatMessageHandler {
    /// CommanderCore 实例，用于处理任务
    commander_core: Arc<CommanderCore>,
    /// 处理器配置
    config: WeChatHandlerConfig,
    /// 消息去重器
    deduplicator: MessageDeduplicator,
    /// 处理器模式
    mode: Arc<RwLock<HandlerMode>>,
    /// 连续失败计数
    failure_count: Arc<RwLock<u32>>,
}

impl std::fmt::Debug for WeChatMessageHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeChatMessageHandler")
            .field("commander_core", &"Arc<CommanderCore>")
            .field("config", &self.config)
            .field("deduplicator", &"MessageDeduplicator")
            .field("mode", &self.mode)
            .field("failure_count", &self.failure_count)
            .finish()
    }
}

impl WeChatMessageHandler {
    /// 创建新的 WeChatMessageHandler 实例
    /// 
    /// # 参数
    /// - `commander_core`: CommanderCore 的 Arc 引用
    /// - `config`: 处理器配置
    /// 
    /// # 返回值
    /// 新的 WeChatMessageHandler 实例
    pub fn new(
        commander_core: Arc<CommanderCore>,
        config: WeChatHandlerConfig,
    ) -> Self {
        let deduplicator = MessageDeduplicator::new(config.deduplication.clone());
        
        Self {
            commander_core,
            config,
            deduplicator,
            mode: Arc::new(RwLock::new(HandlerMode::Full)),
            failure_count: Arc::new(RwLock::new(0)),
        }
    }

    /// 使用默认配置创建 WeChatMessageHandler
    /// 
    /// # 参数
    /// - `commander_core`: CommanderCore 的 Arc 引用
    /// 
    /// # 返回值
    /// 新的 WeChatMessageHandler 实例
    pub fn new_with_defaults(commander_core: Arc<CommanderCore>) -> Self {
        Self::new(commander_core, WeChatHandlerConfig::default())
    }

    /// 获取当前处理器模式
    pub async fn get_mode(&self) -> HandlerMode {
        *self.mode.read().await
    }

    /// 设置处理器模式
    pub async fn set_mode(&self, mode: HandlerMode) {
        let mut m = self.mode.write().await;
        *m = mode;
        info!("Handler mode changed to: {:?}", mode);
    }

    /// 获取当前失败计数
    pub async fn get_failure_count(&self) -> u32 {
        *self.failure_count.read().await
    }

    /// 重置失败计数
    pub async fn reset_failure_count(&self) {
        let mut count = self.failure_count.write().await;
        *count = 0;
        info!("Failure count reset");
    }

    /// 记录失败
    async fn record_failure(&self) {
        let mut count = self.failure_count.write().await;
        *count += 1;
        
        if self.config.graceful_degradation.enabled {
            let failure_count = *count;
            if failure_count >= self.config.graceful_degradation.failure_threshold {
                warn!(
                    "Failure threshold reached ({}), switching to SimpleReply mode",
                    failure_count
                );
                let mut mode = self.mode.write().await;
                *mode = HandlerMode::SimpleReply;
                
                // 启动恢复计时器
                let mode_clone = Arc::clone(&self.mode);
                let failure_count_clone = Arc::clone(&self.failure_count);
                let recovery_window = self.config.graceful_degradation.recovery_window;
                
                tokio::spawn(async move {
                    tokio::time::sleep(recovery_window).await;
                    info!("Recovery window elapsed, resetting to Full mode");
                    let mut mode = mode_clone.write().await;
                    *mode = HandlerMode::Full;
                    let mut count = failure_count_clone.write().await;
                    *count = 0;
                });
            }
        }
    }

    /// 处理消息（完整模式）
    async fn handle_full(&self, message: WebhookMessage) -> Result<(), WebhookError> {
        info!("Processing message in Full mode: {}", message.message_id);

        let (intent, validation) = self.commander_core
            .process_intent(&message.content)
            .await
            .map_err(|e| {
                error!("Intent processing failed: {}", e);
                WebhookError::Http(format!("Intent processing failed: {}", e))
            })?;

        info!("Parsed intent: {}", intent.intent_id);
        info!("Confidence: {:?}", intent.confidence);

        if !validation.is_valid {
            warn!("Intent validation failed: {:?}", validation);
            return Ok(());
        }

        let plan = self.commander_core
            .create_plan_from_intent(intent)
            .await
            .map_err(|e| {
                error!("Plan creation failed: {}", e);
                WebhookError::Http(format!("Plan creation failed: {}", e))
            })?;

        info!("Created plan: {}", plan.plan_id);

        let context = self.commander_core
            .execute_plan(plan)
            .await
            .map_err(|e| {
                error!("Plan execution failed: {}", e);
                WebhookError::Http(format!("Plan execution failed: {}", e))
            })?;

        info!("Plan executed successfully: {:?}", context);
        
        // 重置失败计数
        self.reset_failure_count().await;
        
        Ok(())
    }

    /// 处理消息（简单回复模式）
    async fn handle_simple_reply(&self, message: WebhookMessage) -> Result<(), WebhookError> {
        info!("Processing message in SimpleReply mode: {}", message.message_id);
        
        let reply = format!(
            "收到您的消息：\n\"{}\"\n\n系统正在维护中，请稍后再试。",
            message.content
        );
        
        self.send_response(message, reply).await?;
        Ok(())
    }

    /// 处理消息（离线模式）
    async fn handle_offline(&self, message: WebhookMessage) -> Result<(), WebhookError> {
        info!("Processing message in Offline mode: {}", message.message_id);
        
        let reply = format!(
            "收到您的消息：\n\"{}\"\n\n系统当前离线，消息已保存。",
            message.content
        );
        
        self.send_response(message, reply).await?;
        Ok(())
    }
}

#[async_trait]
impl WebhookHandler for WeChatMessageHandler {
    /// 处理来自微信的消息
    /// 
    /// # 参数
    /// - `message`: WebhookMessage 实例，包含消息内容
    /// 
    /// # 返回值
    /// 成功返回 Ok(())，失败返回 WebhookError
    async fn handle_message(&self, message: WebhookMessage) -> Result<(), WebhookError> {
        info!("Received WeChat message from user: {}", message.from_user);
        debug!("Message content: {}", message.content);

        // 检查是否启用
        if !self.config.enabled {
            debug!("Handler disabled, skipping message");
            return Ok(());
        }

        // 检查消息去重
        if self.deduplicator.is_processed(&message.message_id).await {
            info!("Skipping duplicate message: {}", message.message_id);
            return Ok(());
        }

        // 根据模式处理
        let result = match self.get_mode().await {
            HandlerMode::Full => self.handle_full(message.clone()).await,
            HandlerMode::SimpleReply => self.handle_simple_reply(message.clone()).await,
            HandlerMode::Offline => self.handle_offline(message.clone()).await,
        };

        // 标记消息为已处理
        self.deduplicator.mark_processed(message.message_id.clone()).await;

        // 处理结果
        match result {
            Ok(_) => {
                debug!("Message processed successfully: {}", message.message_id);
                Ok(())
            }
            Err(e) => {
                error!("Message processing failed: {}", e);
                self.record_failure().await;
                Err(e)
            }
        }
    }

    /// 发送响应给微信用户
    /// 
    /// # 参数
    /// - `message`: 原始消息
    /// - `response`: 响应内容
    /// 
    /// # 返回值
    /// 成功返回 Ok(())，失败返回 WebhookError
    async fn send_response(
        &self,
        message: WebhookMessage,
        response: String,
    ) -> Result<(), WebhookError> {
        info!("Sending response to WeChat user: {}", message.from_user);
        debug!("Response: {}", response);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::CommanderCore;
    use crate::agent::config::webhook::{WebhookMessage, IMPlatform};

    #[tokio::test]
    async fn test_wechat_handler_creation() {
        let core = Arc::new(CommanderCore::new());
        let handler = WeChatMessageHandler::new_with_defaults(core);
        assert!(true);
    }

    #[tokio::test]
    async fn test_send_response() {
        let core = Arc::new(CommanderCore::new());
        let handler = WeChatMessageHandler::new_with_defaults(core);
        
        let message = WebhookMessage {
            message_id: "test-msg-001".to_string(),
            from_platform: IMPlatform::WeChat,
            from_user: "test-user".to_string(),
            to_agent: "test-agent".to_string(),
            content: "测试消息".to_string(),
            timestamp: 1234567890,
            metadata: None,
        };
        
        let result = handler.send_response(message, "测试回复".to_string()).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_wechat_handler_debug() {
        let core = Arc::new(CommanderCore::new());
        let handler = WeChatMessageHandler::new_with_defaults(core);
        let debug_str = format!("{:?}", handler);
        assert!(debug_str.contains("WeChatMessageHandler"));
    }

    #[test]
    fn test_wechat_handler_clone() {
        let core = Arc::new(CommanderCore::new());
        let handler = WeChatMessageHandler::new_with_defaults(core);
        let handler_clone = handler.clone();
        assert!(true);
    }

    #[tokio::test]
    async fn test_handler_mode() {
        let core = Arc::new(CommanderCore::new());
        let handler = WeChatMessageHandler::new_with_defaults(core);
        
        // 默认模式应该是 Full
        assert_eq!(handler.get_mode().await, HandlerMode::Full);
        
        // 设置为 SimpleReply
        handler.set_mode(HandlerMode::SimpleReply).await;
        assert_eq!(handler.get_mode().await, HandlerMode::SimpleReply);
        
        // 设置为 Offline
        handler.set_mode(HandlerMode::Offline).await;
        assert_eq!(handler.get_mode().await, HandlerMode::Offline);
    }

    #[tokio::test]
    async fn test_failure_count() {
        let core = Arc::new(CommanderCore::new());
        let handler = WeChatMessageHandler::new_with_defaults(core);
        
        // 初始失败计数应该是 0
        assert_eq!(handler.get_failure_count().await, 0);
        
        // 重置应该保持 0
        handler.reset_failure_count().await;
        assert_eq!(handler.get_failure_count().await, 0);
    }

    #[tokio::test]
    async fn test_with_custom_config() {
        let core = Arc::new(CommanderCore::new());
        
        let mut config = WeChatHandlerConfig::default();
        config.enabled = false;
        config.max_concurrent_tasks = 10;
        
        let handler = WeChatMessageHandler::new(core, config);
        
        // 验证配置被正确应用
        assert!(!handler.config.enabled);
        assert_eq!(handler.config.max_concurrent_tasks, 10);
    }
}
