use crate::e2e::common::*;
use aetheris::config::AppConfig;
use aetheris::soul::{Soul, SoulRegistry, SoulMetadata};
use aetheris::skill::{Version, SkillMetadata, SkillPriority};
use aetheris::security::{SecurityManager, SecurityValidationResult};
use aetheris::core::CommanderCore;
use std::path::PathBuf;

#[test]
fn test_config_loading_and_initialization() {
    let mut env = E2ETestEnvironment::new().unwrap();
    env.log("Starting config loading test");

    let config_dir = env.get_config_dir().unwrap();
    let test_config = E2EDataGenerator::generate_test_config();
    let config_path = env.temp_dir.create_json_file("test-config.json", &test_config).unwrap();
    
    env.log(&format!("Created test config at: {}", config_path.display()));
    
    let config = AppConfig::from_env().unwrap();
    assert!(config.validate().is_ok());
    
    env.log("Config validation passed");
    assert!(env.elapsed_time().num_seconds() < 10);
}

#[test]
fn test_soul_complete_workflow() {
    let mut env = E2ETestEnvironment::new().unwrap();
    env.log("Starting soul complete workflow test");

    let souls_dir = env.get_souls_dir().unwrap();
    let soul_content = E2EDataGenerator::generate_test_soul_content("E2E Test Soul", "1.0.0");
    
    let soul_path = souls_dir.join("e2e-test-soul.md");
    std::fs::write(&soul_path, soul_content).unwrap();
    
    env.log(&format!("Created test soul at: {}", soul_path.display()));
    assertions::assert_path_exists(&soul_path);
    assertions::assert_file_not_empty(&soul_path);

    let mut registry = SoulRegistry::new(souls_dir).unwrap();
    registry.load_all().unwrap();
    
    env.log("Soul registry loaded successfully");
    assert_eq!(registry.list().len(), 1);

    let soul = registry.get("E2E Test Soul").unwrap();
    assert_eq!(soul.name(), "E2E Test Soul");
    
    env.log("Soul retrieval successful");
    assert!(env.elapsed_time().num_seconds() < 10);
}

#[test]
fn test_skill_loading_and_validation() {
    let mut env = E2ETestEnvironment::new().unwrap();
    env.log("Starting skill loading and validation test");

    let version = Version::from_string("1.0.0").unwrap();
    let metadata = SkillMetadata::new(
        "e2e-test-skill".to_string(),
        "E2E Test Skill".to_string(),
        version.clone(),
        "A test skill for E2E testing".to_string(),
    ).with_priority(SkillPriority::High);
    
    env.log("Skill metadata created");
    assert_eq!(metadata.id, "e2e-test-skill");
    assert_eq!(metadata.priority, SkillPriority::High);
    assert!(metadata.priority.should_preload());
    assert!(metadata.priority.should_load());

    let v2 = Version::from_string("2.0.0").unwrap();
    assert!(!version.is_compatible_with(&v2));
    assert!(v2.is_compatible_with(&version));
    
    env.log("Version compatibility check passed");
    assert!(env.elapsed_time().num_seconds() < 10);
}

#[test]
fn test_security_validation_workflow() {
    let mut env = E2ETestEnvironment::new().unwrap();
    env.log("Starting security validation workflow test");

    let security_manager = SecurityManager::new();
    assert!(!security_manager.enabled_layers.read().is_empty());
    
    env.log("Security manager initialized successfully");

    let task = aetheris::core::Task {
        id: "e2e-test-task-123".to_string(),
        name: "E2E Test Task".to_string(),
        description: "A test task for E2E testing".to_string(),
        status: aetheris::core::TaskStatus::Pending,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        dependencies: vec![],
        metadata: std::collections::HashMap::new(),
    };
    
    env.log("Test task created");
    
    let result = futures::executor::block_on(security_manager.validate_task(&task));
    assert!(result.is_ok());
    
    let validation_result = result.unwrap();
    assert_eq!(validation_result.task_id, "e2e-test-task-123");
    
    env.log("Security validation passed");
    assert!(env.elapsed_time().num_seconds() < 10);
}

#[test]
fn test_core_initialization_workflow() {
    let mut env = E2ETestEnvironment::new().unwrap();
    env.log("Starting core initialization workflow test");

    let core = CommanderCore::new();
    assert!(core.executors.is_empty());
    
    env.log("CommanderCore initialized successfully");
    assert!(env.elapsed_time().num_seconds() < 10);
}

#[test]
fn test_soul_version_management() {
    let mut env = E2ETestEnvironment::new().unwrap();
    env.log("Starting soul version management test");

    let souls_dir = env.get_souls_dir().unwrap();
    
    let soul_v1_content = E2EDataGenerator::generate_test_soul_content("Versioned Soul", "1.0.0");
    let soul_v1_path = souls_dir.join("versioned-soul-v1.md");
    std::fs::write(&soul_v1_path, soul_v1_content).unwrap();
    
    let soul_v2_content = E2EDataGenerator::generate_test_soul_content("Versioned Soul", "2.0.0");
    let soul_v2_path = souls_dir.join("versioned-soul-v2.md");
    std::fs::write(&soul_v2_path, soul_v2_content).unwrap();
    
    env.log("Created multiple soul versions");

    let mut registry = SoulRegistry::new(souls_dir).unwrap();
    registry.load_all().unwrap();
    
    assert_eq!(registry.list().len(), 2);
    
    env.log("Soul version management test passed");
    assert!(env.elapsed_time().num_seconds() < 10);
}
