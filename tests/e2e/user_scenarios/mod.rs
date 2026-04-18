use crate::e2e::common::*;
use aetheris::config::{AppConfig, OnboardWizard, OnboardProgress};
use aetheris::soul::SoulRegistry;
use aetheris::skill::{Version, SkillMetadata};
use aetheris::security::SecurityManager;
use std::path::PathBuf;

#[test]
fn test_new_user_onboarding_scenario() {
    let mut env = E2ETestEnvironment::new().unwrap();
    env.log("Starting new user onboarding scenario test");

    let config_dir = env.get_config_dir().unwrap();
    
    let mut wizard = OnboardWizard::new(config_dir.clone()).unwrap();
    
    env.log("Onboarding wizard initialized");
    
    let progress = wizard.progress();
    assert_eq!(progress.current_step, "welcome");
    assert!(!progress.is_complete());
    
    env.log("Onboarding progress initialized correctly");
    
    wizard.create_default_config().unwrap();
    
    env.log("Default config created");
    
    wizard.setup_default_soul().unwrap();
    
    env.log("Default soul setup");
    
    wizard.complete_onboard().unwrap();
    
    assert!(wizard.progress().is_complete());
    
    env.log("Onboarding scenario completed successfully");
    assert!(env.elapsed_time().num_seconds() < 10);
}

#[test]
fn test_persona_creator_sharing_scenario() {
    let mut env = E2ETestEnvironment::new().unwrap();
    env.log("Starting persona creator sharing scenario test");

    let souls_dir = env.get_souls_dir().unwrap();
    
    let creator_persona_content = E2EDataGenerator::generate_test_soul_content("Creator Persona", "1.0.0");
    let persona_path = souls_dir.join("creator-persona.md");
    std::fs::write(&persona_path, creator_persona_content).unwrap();
    
    env.log("Created persona file");
    assertions::assert_path_exists(&persona_path);
    assertions::assert_file_not_empty(&persona_path);

    let mut registry = SoulRegistry::new(souls_dir).unwrap();
    registry.load_all().unwrap();
    
    assert_eq!(registry.list().len(), 1);
    
    env.log("Persona loaded into registry");
    
    let persona = registry.get("Creator Persona").unwrap();
    assert_eq!(persona.name(), "Creator Persona");
    
    env.log("Persona sharing scenario completed");
    assert!(env.elapsed_time().num_seconds() < 10);
}

#[test]
fn test_admin_security_management_scenario() {
    let mut env = E2ETestEnvironment::new().unwrap();
    env.log("Starting admin security management scenario test");

    let security_manager = SecurityManager::new();
    
    assert!(!security_manager.enabled_layers.read().is_empty());
    
    env.log("Security manager initialized");
    
    env.log("Admin security management scenario completed");
    assert!(env.elapsed_time().num_seconds() < 10);
}

#[test]
fn test_developer_plugin_development_scenario() {
    let mut env = E2ETestEnvironment::new().unwrap();
    env.log("Starting developer plugin development scenario test");

    let skills_dir = env.get_skills_dir().unwrap();
    
    let skill_metadata = E2EDataGenerator::generate_test_skill_metadata(
        "dev-plugin",
        "Developer Plugin",
        "1.0.0"
    );
    
    let skill_path = skills_dir.join("dev-plugin.yaml");
    env.temp_dir.create_json_file("dev-plugin.yaml", &skill_metadata).unwrap();
    
    env.log("Created plugin metadata");
    assertions::assert_path_exists(&skill_path);

    let version = Version::from_string("1.0.0").unwrap();
    let metadata = SkillMetadata::new(
        "dev-plugin".to_string(),
        "Developer Plugin".to_string(),
        version,
        "A developer test plugin".to_string(),
    );
    
    assert_eq!(metadata.id, "dev-plugin");
    
    env.log("Plugin metadata validated");
    
    env.log("Developer plugin development scenario completed");
    assert!(env.elapsed_time().num_seconds() < 10);
}
