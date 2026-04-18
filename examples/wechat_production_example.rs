
use aetheris::agent::config::webhook::{WebhookManager, WebhookConfig, IMPlatform};
use aetheris::agent::{
    WeChatMessageHandler, WeChatHandlerConfig, HandlerMode, ConfidenceThreshold,
    MessageDeduplicator, DeduplicationConfig,
};
use aetheris::core::CommanderCore;
use std::sync::Arc;
use tracing::info;

/// 生产级 WeChat 集成示例
/// 
/// 展示如何配置和使用生产级 WeChat 处理器

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Aetheris 生产级 WeChat 集成示例 ===\n");

    // 1. 创建 CommanderCore
    let commander_core = Arc::new(CommanderCore::new());
    info!("✓ CommanderCore 初始化完成");

    // 2. 配置生产级配置
    let mut config = WeChatHandlerConfig::default();
    
    // 启用处理器
    config.enabled = true;
    
    // 设置自动确认阈值
    config.auto_confirm_threshold = ConfidenceThreshold::High;
    
    // 启用进度通知
    config.enable_progress_notification = false;
    
    // 设置最大并发任务
    config.max_concurrent_tasks = 10;
    
    // 配置消息去重
    config.deduplication.enabled = true;
    config.deduplication.ttl = std::time::Duration::from_secs(3600); // 1小时
    config.deduplication.max_cache_size = 10000;
    
    // 配置优雅降级
    config.graceful_degradation.enabled = true;
    config.graceful_degradation.failure_threshold = 5;
    config.graceful_degradation.recovery_window = std::time::Duration::from_secs(60);
    
    info!("✓ 生产级配置完成");

    // 3. 创建 WeChatMessageHandler
    let handler = WeChatMessageHandler::new(commander_core.clone(), config);
    info!("✓ WeChatMessageHandler 创建完成");

    // 4. 创建 WebhookManager
    let webhook_config = WebhookConfig {
        enabled: true,
        endpoint: "/webhook".to_string(),
        secret: None,
        verify_signature: false,
    };
    let webhook_manager = Arc::new(WebhookManager::new(webhook_config));
    
    // 5. 注册 WeChatHandler
    let handler_arc = Arc::new(handler);
    webhook_manager.register_handler(IMPlatform::WeChat, handler_arc.clone()).await;
    info!("✓ WeChatHandler 已注册到 WebhookManager");

    // 6. 演示功能
    println!("\n--- 功能演示 ---");
    
    // 演示 1: 获取当前模式
    let mode = handler_arc.get_mode().await;
    println!("当前处理器模式: {:?}", mode);
    
    // 演示 2: 切换模式
    handler_arc.set_mode(HandlerMode::SimpleReply).await;
    let new_mode = handler_arc.get_mode().await;
    println!("切换模式后: {:?}", new_mode);
    
    // 演示 3: 重置模式
    handler_arc.set_mode(HandlerMode::Full).await;
    println!("重置回 Full 模式");
    
    // 演示 4: 检查失败计数
    let failure_count = handler_arc.get_failure_count().await;
    println!("当前失败计数: {}", failure_count);
    
    // 演示 5: 重置失败计数
    handler_arc.reset_failure_count().await;
    println!("重置失败计数");

    println!("\n--- 消息去重演示 ---");
    
    // 7. 单独演示 MessageDeduplicator
    let dedup_config = DeduplicationConfig::default();
    let deduplicator = MessageDeduplicator::new(dedup_config);
    
    let test_msg_id = "test-message-001";
    
    // 第一次检查
    let is_processed = deduplicator.is_processed(test_msg_id).await;
    println!("消息 '{}' 是否已处理: {}", test_msg_id, is_processed);
    
    // 标记为已处理
    deduplicator.mark_processed(test_msg_id.to_string()).await;
    println!("标记消息 '{}' 为已处理", test_msg_id);
    
    // 再次检查
    let is_processed_again = deduplicator.is_processed(test_msg_id).await;
    println!("消息 '{}' 是否已处理: {}", test_msg_id, is_processed_again);
    
    // 检查处理次数
    let count = deduplicator.get_processed_count(test_msg_id).await;
    println!("消息 '{}' 处理次数: {}", test_msg_id, count);
    
    // 检查缓存大小
    let cache_size = deduplicator.cache_size();
    println!("当前缓存大小: {}", cache_size);

    println!("\n=== 配置文件示例 ===");
    println!("\n将以下配置保存为 config/wechat_handler.yaml:\n");
    println!(r#"enabled: true
auto_confirm_threshold: "high"
enable_progress_notification: false
max_concurrent_tasks: 10

deduplication:
  enabled: true
  ttl: 3600
  max_cache_size: 10000

graceful_degradation:
  enabled: true
  failure_threshold: 5
  recovery_window: 60
"#);

    println!("\n=== 使用步骤 ===");
    println!("1. 创建配置文件: config/wechat_handler.yaml");
    println!("2. 在 main.rs 中加载配置");
    println!("3. 创建 WeChatMessageHandler");
    println!("4. 注册到 WebhookManager");
    println!("5. 启动服务");

    println!("\n✓ 生产级 WeChat 集成示例完成！");
    Ok(())
}
