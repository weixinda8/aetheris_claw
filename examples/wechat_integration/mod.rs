pub mod wechat_handler;

pub use wechat_handler::WeChatMessageHandler;

use crate::agent::config::webhook::{WebhookManager, WebhookConfig, IMPlatform};
use crate::core::CommanderCore;
use std::sync::Arc;

pub async fn setup_wechat_integration(
    commander_core: Arc<CommanderCore>,
) -> Arc<WebhookManager> {
    let webhook_config = WebhookConfig {
        enabled: true,
        endpoint: "/webhook".to_string(),
        secret: None,
        verify_signature: false,
    };

    let webhook_manager = Arc::new(WebhookManager::new(webhook_config));

    let wechat_handler = Arc::new(WeChatMessageHandler::new(commander_core));
    webhook_manager.register_handler(IMPlatform::WeChat, wechat_handler).await;

    webhook_manager
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::CommanderCore;

    #[tokio::test]
    async fn test_wechat_handler_creation() {
        let core = Arc::new(CommanderCore::new());
        let handler = WeChatMessageHandler::new(core);
        
        // Just test that it can be created
        assert!(true);
    }
}
