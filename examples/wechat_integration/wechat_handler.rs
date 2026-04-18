use crate::agent::config::webhook::{WebhookHandler, WebhookMessage, WebhookError};
use crate::core::CommanderCore;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, warn};

pub struct WeChatMessageHandler {
    commander_core: Arc<CommanderCore>,
}

impl WeChatMessageHandler {
    pub fn new(commander_core: Arc<CommanderCore>) -> Self {
        Self { commander_core }
    }
}

#[async_trait]
impl WebhookHandler for WeChatMessageHandler {
    async fn handle_message(&self, message: WebhookMessage) -> Result<(), WebhookError> {
        info!("Received WeChat message from user: {}", message.from_user);
        info!("Message content: {}", message.content);

        let (intent, validation) = self.commander_core
            .process_intent(&message.content)
            .await
            .map_err(|e| WebhookError::Http(format!("Intent processing failed: {}", e)))?;

        info!("Parsed intent: {}", intent.intent_id);
        info!("Confidence: {:?}", intent.confidence);

        if !validation.is_valid {
            warn!("Intent validation failed: {:?}", validation);
            return Ok(());
        }

        let plan = self.commander_core
            .create_plan_from_intent(intent)
            .await
            .map_err(|e| WebhookError::Http(format!("Plan creation failed: {}", e)))?;

        info!("Created plan: {}", plan.plan_id);

        let context = self.commander_core
            .execute_plan(plan)
            .await
            .map_err(|e| WebhookError::Http(format!("Plan execution failed: {}", e)))?;

        info!("Plan executed successfully: {:?}", context);
        Ok(())
    }

    async fn send_response(
        &self,
        message: WebhookMessage,
        response: String,
    ) -> Result<(), WebhookError> {
        info!("Sending response to WeChat user: {}", message.from_user);
        info!("Response: {}", response);
        Ok(())
    }
}
