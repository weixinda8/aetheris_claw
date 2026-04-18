
use aetheris::agent::config::webhook::{WebhookManager, WebhookConfig, IMPlatform};
use aetheris::agent::WeChatMessageHandler;
use aetheris::core::CommanderCore;
use std::sync::Arc;
use tracing::info;

/// 示例：微信集成完整配置
/// 
/// 本示例展示如何在 Aetheris 中集成微信消息处理
/// 
/// 使用方法：
/// 1. 配置 ilink 服务（获取 ilink_url 和 token）
/// 2. 在 Agent 配置中启用微信通道
/// 3. 集成 WeChatMessageHandler 到 main.rs

async fn setup_wechat_integration_example() {
    info!("=== Aetheris 微信集成示例 ===");

    let commander_core = Arc::new(CommanderCore::new());

    let webhook_config = WebhookConfig {
        enabled: true,
        endpoint: "/webhook".to_string(),
        secret: None,
        verify_signature: false,
    };

    let webhook_manager = Arc::new(WebhookManager::new(webhook_config));

    let wechat_handler = Arc::new(WeChatMessageHandler::new(commander_core));
    webhook_manager.register_handler(IMPlatform::WeChat, wechat_handler).await;

    info!("✓ WeChatMessageHandler 已注册");
    info!("✓ 微信集成配置完成");
}

/// Agent 配置示例 - 启用微信通道
///
/// 将此配置保存到 agents/coordinator_agent.yaml
const AGENT_CONFIG_EXAMPLE: &str = r#"
meta:
  id: "coordinator-agent"
  name: "Coordinator Agent"
  version: "1.0.0"
  type: "Generic"
  enabled: true
  hot_reload: false
  workspace: "./agents"
  tags: ["coordinator", "orchestrator"]

channels:
  wechat:
    enabled: true
    ilink_enabled: true
    ilink_server: "https://your-ilink-server.com"
    ilink_token: "your-token-here"
    poll_interval_seconds: 5
"#;

/// main.rs 集成代码示例
///
/// 将此代码添加到 src/main.rs 的 run_server() 函数中
const MAIN_RS_INTEGRATION_EXAMPLE: &str = r#"
use aetheris::agent::config::webhook::{WebhookManager, WebhookConfig, IMPlatform};
use aetheris::agent::WeChatMessageHandler;

async fn run_server() -> Result<()> {
    // ... 现有初始化代码 ...

    let commander = CommanderCore::new();

    // === 微信集成开始 ===
    info!("Initializing WeChat integration");
    let webhook_config = WebhookConfig {
        enabled: true,
        endpoint: "/webhook".to_string(),
        secret: None,
        verify_signature: false,
    };
    let webhook_manager = Arc::new(WebhookManager::new(webhook_config));

    let wechat_handler = Arc::new(WeChatMessageHandler::new(Arc::new(commander.clone())));
    webhook_manager.register_handler(IMPlatform::WeChat, wechat_handler).await;
    info!("WeChat integration initialized successfully");
    // === 微信集成结束 ===

    // ... 继续 AppStateBuilder 和服务器启动 ...
}
"#;

fn main() {
    println!("=== Aetheris 微信集成示例指南 ===");
    println!();
    println!("1. WeChatMessageHandler 已创建在: src/agent/wechat_handler.rs");
    println!("2. 模块已在 src/agent/mod.rs 中导出");
    println!();
    println!("使用步骤:");
    println!("1. 配置 ilink 服务（获取 API 凭证）");
    println!("2. 创建 agents/coordinator_agent.yaml（见下方示例）");
    println!("3. 在 src/main.rs 中集成（见下方示例）");
    println!("4. 启动 Aetheris");
    println!("5. 用微信发送消息测试");
    println!();
    println!("--- Agent 配置示例 ---");
    println!("{}", AGENT_CONFIG_EXAMPLE);
    println!();
    println!("--- main.rs 集成示例 ---");
    println!("{}", MAIN_RS_INTEGRATION_EXAMPLE);
}
