use aetheris::security::{
    SecurityManager, SecurityLayer, SecurityValidationResult,
};
use aetheris::core::Task;

#[tokio::test]
async fn test_security_manager_new() {
    let manager = SecurityManager::new();
    let enabled_layers = {
        let guard = manager.enabled_layers.read().await;
        guard.clone()
    };
    
    assert!(enabled_layers.contains(&SecurityLayer::RuleBlocking));
    assert!(enabled_layers.contains(&SecurityLayer::AuditSigning));
    assert!(enabled_layers.contains(&SecurityLayer::SandboxIsolation));
    assert!(enabled_layers.contains(&SecurityLayer::ThreeLayerQualityCheck));
    assert!(enabled_layers.contains(&SecurityLayer::IndustryCompliance));
    assert!(enabled_layers.contains(&SecurityLayer::HumanIntervention));
}

#[tokio::test]
async fn test_security_manager_default() {
    let manager1 = SecurityManager::new();
    let manager2 = SecurityManager::default();
    
    let layers1 = {
        let guard = manager1.enabled_layers.read().await;
        guard.clone()
    };
    let layers2 = {
        let guard = manager2.enabled_layers.read().await;
        guard.clone()
    };
    
    assert_eq!(layers1, layers2);
}

#[tokio::test]
async fn test_security_manager_enable_layer() {
    let manager = SecurityManager::new();
    
    manager.enable_layer(SecurityLayer::RuleBlocking).await;
    
    let enabled_layers = {
        let guard = manager.enabled_layers.read().await;
        guard.clone()
    };
    
    assert!(enabled_layers.contains(&SecurityLayer::RuleBlocking));
}

#[tokio::test]
async fn test_security_manager_disable_layer() {
    let manager = SecurityManager::new();
    
    manager.disable_layer(&SecurityLayer::RuleBlocking).await;
    
    let enabled_layers = {
        let guard = manager.enabled_layers.read().await;
        guard.clone()
    };
    
    assert!(!enabled_layers.contains(&SecurityLayer::RuleBlocking));
}

#[tokio::test]
async fn test_security_manager_validate_task() {
    let manager = SecurityManager::new();
    let task = Task::new("Test Task".to_string(), 1);
    
    let result = manager.validate_task(&task).await;
    
    assert!(result.is_ok());
    let validation = result.unwrap();
    assert_eq!(validation.task_id, task.id);
    assert!(validation.passed);
}

#[tokio::test]
async fn test_security_manager_add_security_rule() {
    let manager = SecurityManager::new();
    
    let rule = Default::default();
    manager.add_security_rule(rule).await;
    
    let rule_engine = manager.rule_engine.read().await;
    assert!(rule_engine.rules.len() > 0);
}

#[tokio::test]
async fn test_security_manager_enable_compliance_standard() {
    let manager = SecurityManager::new();
    
    let standard = Default::default();
    manager.enable_compliance_standard(standard).await;
    
    let compliance_engine = manager.compliance_engine.read().await;
    assert!(compliance_engine.active_standards.len() > 0);
}
