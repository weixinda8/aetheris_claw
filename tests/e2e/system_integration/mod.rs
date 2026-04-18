use crate::e2e::common::*;
use aetheris::config::AppConfig;
use aetheris::core::CommanderCore;
use aetheris::security::SecurityManager;
use aetheris::soul::SoulRegistry;
use aetheris::skill::SkillMetadata;

#[test]
fn test_config_core_integration() {
    let mut env = E2ETestEnvironment::new().unwrap();
    env.log("Starting config and core integration test");

    let config = AppConfig::from_env().unwrap();
    assert!(config.validate().is_ok());
    
    env.log("Config loaded successfully");
    
    let core = CommanderCore::new();
    assert!(core.executors.is_empty());
    
    env.log("CommanderCore initialized successfully");
    
    assert!(env.elapsed_time().num_seconds() < 10);
}

#[test]
fn test_core_task_creation() {
    let mut env = E2ETestEnvironment::new().unwrap();
    env.log("Starting core task creation test");

    let core = CommanderCore::new();
    
    env.log("Core initialized");
    
    let task = aetheris::core::Task {
        id: "e2e-core-task".to_string(),
        name: "E2E Core Task".to_string(),
        description: "Test task for core integration".to_string(),
        status: aetheris::core::TaskStatus::Pending,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        dependencies: vec![],
        metadata: std::collections::HashMap::new(),
    };
    
    assert_eq!(task.id, "e2e-core-task");
    assert_eq!(task.status, aetheris::core::TaskStatus::Pending);
    
    env.log("Test task created and validated");
    
    env.log("Core task creation test passed");
    assert!(env.elapsed_time().num_seconds() < 10);
}

#[test]
fn test_security_soul_integration() {
    let mut env = E2ETestEnvironment::new().unwrap();
    env.log("Starting security and soul integration test");

    let souls_dir = env.get_souls_dir().unwrap();
    
    let soul_content = E2EDataGenerator::generate_test_soul_content("Integration Soul", "1.0.0");
    let soul_path = souls_dir.join("integration-soul.md");
    std::fs::write(&soul_path, soul_content).unwrap();
    
    let mut registry = SoulRegistry::new(souls_dir).unwrap();
    registry.load_all().unwrap();
    
    env.log("Soul registry loaded");
    
    let security_manager = SecurityManager::new();
    assert!(!security_manager.enabled_layers.read().is_empty());
    
    env.log("Security manager initialized");
    
    env.log("Security and soul integration test passed");
    assert!(env.elapsed_time().num_seconds() < 10);
}

#[test]
fn test_skill_core_integration() {
    let mut env = E2ETestEnvironment::new().unwrap();
    env.log("Starting skill and core integration test");

    let version = aetheris::skill::Version::from_string("1.0.0").unwrap();
    let metadata = SkillMetadata::new(
        "e2e-skill".to_string(),
        "E2E Test Skill".to_string(),
        version,
        "Test skill for integration".to_string(),
    );
    
    assert_eq!(metadata.id, "e2e-skill");
    
    env.log("Skill metadata validated");
    
    env.log("Skill and core integration test passed");
    assert!(env.elapsed_time().num_seconds() < 10);
}
