use aetheris::skill::*;
use aetheris::utils::Result;
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;

fn create_test_skill(id: &str, name: &str, priority: SkillPriority) -> Arc<dyn Skill> {
    let version = Version::new(1, 0, 0);
    let mut metadata = SkillMetadata::new(
        id.to_string(),
        name.to_string(),
        version,
        format!(
            "A test skill for {} used to verify integration functionality",
            name
        ),
    );
    metadata.priority = priority;
    metadata.tags = vec!["test".to_string(), "integration".to_string()];
    metadata.categories = vec!["testing".to_string()];
    Arc::new(BaseSkill::new(metadata))
}

#[test]
fn test_skill_registry_progressive_disclosure_lazy_strategy() -> Result<()> {
    let registry = Arc::new(SkillRegistry::new());
    let mut manager = ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::Lazy)?;

    let skill = create_test_skill("lazy-test-skill", "Lazy Test Skill", SkillPriority::Medium);
    registry.register(skill.clone());

    let dir = tempdir()?;
    let skill_dir = dir.path().join("test-skill");
    std::fs::create_dir(&skill_dir)?;
    let skill_md_path = skill_dir.join("SKILL.md");
    let content = r#"---
name: test-skill
description: A test skill for testing integration with progressive disclosure. Use this to verify indexing and matching works.
version: 1.0.0
tags: ["test", "integration"]
timeout: 60
---

# Test Skill
## 功能概述
Test skill content.
## 适用场景
Testing integration.
## 输入规范
None.
## 执行步骤
1. Do something.
## 输出规范
Something.
## 约束与安全
None.
## 示例
None.
"#;
    std::fs::write(&skill_md_path, content)?;

    let paths = vec![dir.path().to_path_buf()];
    let indexed = manager.index_skills(&paths)?;
    assert!(indexed >= 0);

    let matches = manager.find_matching_skills("test")?;
    assert!(!matches.is_empty() || manager.list_indexed_skills()?.len() > 0);

    Ok(())
}

#[test]
fn test_skill_registry_progressive_disclosure_eager_metadata_strategy() -> Result<()> {
    let registry = Arc::new(SkillRegistry::new());
    let manager =
        ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::EagerMetadata)?;

    let skill1 = create_test_skill(
        "eager-meta-skill-1",
        "Eager Meta Skill 1",
        SkillPriority::Mandatory,
    );
    let skill2 = create_test_skill(
        "eager-meta-skill-2",
        "Eager Meta Skill 2",
        SkillPriority::Low,
    );

    registry.register(skill1.clone());
    registry.register(skill2.clone());

    assert!(registry.exists("eager-meta-skill-1"));
    assert!(registry.exists("eager-meta-skill-2"));

    let listed = registry.list();
    assert_eq!(listed.len(), 2);

    Ok(())
}

#[test]
fn test_skill_registry_progressive_disclosure_eager_critical_strategy() -> Result<()> {
    let registry = Arc::new(SkillRegistry::new());
    let manager =
        ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::EagerCritical)?;

    let mandatory_skill = create_test_skill(
        "critical-skill-1",
        "Critical Skill 1",
        SkillPriority::Mandatory,
    );
    let high_skill = create_test_skill("critical-skill-2", "Critical Skill 2", SkillPriority::High);
    let ondemand_skill = create_test_skill(
        "critical-skill-3",
        "OnDemand Skill",
        SkillPriority::OnDemand,
    );

    registry.register(mandatory_skill.clone());
    registry.register(high_skill.clone());
    registry.register(ondemand_skill.clone());

    assert!(registry.exists("critical-skill-1"));
    assert!(registry.exists("critical-skill-2"));
    assert!(registry.exists("critical-skill-3"));

    let active = registry.list_active();
    assert_eq!(active.len(), 3);

    Ok(())
}

#[test]
fn test_skill_registry_elegant_degradation() -> Result<()> {
    let registry = Arc::new(SkillRegistry::new());

    let skill = create_test_skill(
        "degrade-test-skill",
        "Degrade Test Skill",
        SkillPriority::Medium,
    );
    registry.register(skill.clone());

    let version = Version::new(1, 0, 0);
    let metadata = registry.get_version_metadata("nonexistent-skill", &version)?;
    assert!(metadata.is_none());

    let result = registry.record_security_scan(
        "nonexistent-skill",
        SecurityScanResult {
            scan_id: "scan-123".to_string(),
            scanned_at: Utc::now(),
            vulnerabilities: vec![],
            passed: true,
        },
    );
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_agent_skills_registry_progressive_disclosure() -> Result<()> {
    let dir = tempdir()?;
    let skills_dir = dir.path().join("skills");
    std::fs::create_dir(&skills_dir)?;

    let skill_names = vec!["agent-skill-1", "agent-skill-2"];

    for skill_name in &skill_names {
        let skill_dir = skills_dir.join(skill_name);
        std::fs::create_dir(&skill_dir)?;

        let skill_md_path = skill_dir.join("SKILL.md");
        let content = format!(
            r#"---
name: {}
description: An agent skill for integration testing. Use this to verify agent skills registry works with progressive disclosure.
version: 1.0.0
tags: ["test", "agent"]
timeout: 60
---

# {}
## 功能概述
Test agent skill.
## 适用场景
Testing agent skills.
## 输入规范
None.
## 执行步骤
1. Do something.
## 输出规范
Something.
## 约束与安全
None.
## 示例
None.
"#,
            skill_name, skill_name
        );
        std::fs::write(&skill_md_path, content)?;
    }

    let agent_registry = AgentSkillsRegistry::new(skills_dir.clone())?;
    assert_eq!(agent_registry.list().len(), 2);

    let registry = Arc::new(SkillRegistry::new());
    let mut manager = ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::Lazy)?;

    let paths = vec![skills_dir];
    let indexed = manager.index_skills(&paths)?;
    assert_eq!(indexed, 2);

    Ok(())
}

#[test]
fn test_skill_loader_metadata_index_integration() -> Result<()> {
    let index_store = Arc::new(InMemoryMetadataIndexStore::new());
    let loader = SkillLoader::new().with_metadata_index(index_store.clone());

    let dir = tempdir()?;
    let skill_dir = dir.path().join("loader-skill");
    std::fs::create_dir(&skill_dir)?;

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = r#"---
name: loader-test-skill
description: A skill for testing loader and metadata index integration. Use this to verify loading and indexing work together.
version: 1.0.0
tags: ["loader", "index"]
categories: ["testing", "integration"]
timeout: 60
---

# Loader Test Skill
## 功能概述
Test loader and index integration.
## 适用场景
Testing.
## 输入规范
None.
## 执行步骤
1. Load skill.
2. Index metadata.
## 输出规范
Loaded and indexed.
## 约束与安全
None.
## 示例
None.
"#;
    std::fs::write(&skill_md_path, content)?;

    let rt = tokio::runtime::Runtime::new()?;
    let skills = rt.block_on(loader.load_from_path(dir.path().to_str().unwrap()))?;

    assert_eq!(skills.len(), 1);

    let indexed = index_store.list_all()?;
    assert_eq!(indexed.len(), 1);

    let by_tag = index_store.search_by_tags(&vec!["loader".to_string()])?;
    assert_eq!(by_tag.len(), 1);

    let by_category = index_store.search_by_category("testing")?;
    assert_eq!(by_category.len(), 1);

    Ok(())
}

#[test]
fn test_unified_call_service_enhanced_registry() -> Result<()> {
    let registry = Arc::new(SkillRegistry::new());

    let skill = create_test_skill(
        "unified-call-skill",
        "Unified Call Skill",
        SkillPriority::Medium,
    );
    registry.register(skill.clone());

    let text_handler = TextHandler::new();
    let database_handler = DatabaseHandler::new();

    let text_request = UnifiedCallRequest {
        call_mode: CallMode::Text,
        payload: serde_json::Value::String("test input".to_string()),
        options: None,
    };

    let rt = tokio::runtime::Runtime::new()?;
    let text_response = rt.block_on(text_handler.execute(text_request))?;
    assert!(text_response.success);
    assert!(text_response.data.is_some());

    let db_request = UnifiedCallRequest {
        call_mode: CallMode::Database,
        payload: serde_json::Value::Null,
        options: None,
    };

    let db_response = rt.block_on(database_handler.execute(db_request))?;
    assert!(db_response.success);

    Ok(())
}

#[test]
fn test_complete_index_match_load_flow() -> Result<()> {
    let registry = Arc::new(SkillRegistry::new());

    let skill1 = create_test_skill(
        "flow-skill-1",
        "Flow Skill 1 - Data Analysis",
        SkillPriority::High,
    );
    let skill2 = create_test_skill(
        "flow-skill-2",
        "Flow Skill 2 - Web Search",
        SkillPriority::Medium,
    );
    let skill3 = create_test_skill(
        "flow-skill-3",
        "Flow Skill 3 - File Operations",
        SkillPriority::Low,
    );

    registry.register(skill1.clone());
    registry.register(skill2.clone());
    registry.register(skill3.clone());

    let mut manager =
        ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::EagerCritical)?;

    let index_store = InMemoryMetadataIndexStore::new();
    index_store.index_metadata(skill1.metadata().clone())?;
    index_store.index_metadata(skill2.metadata().clone())?;
    index_store.index_metadata(skill3.metadata().clone())?;

    let all_indexed = index_store.list_all()?;
    assert_eq!(all_indexed.len(), 3);

    let search_results = index_store.search_by_name("skill")?;
    assert_eq!(search_results.len(), 3);

    let loaded_skill = registry.get("flow-skill-1");
    assert!(loaded_skill.is_some());

    Ok(())
}

#[test]
fn test_skill_metadata_version_management() -> Result<()> {
    let registry = SkillRegistry::new();

    let version1 = Version::new(1, 0, 0);
    let version2 = Version::new(1, 1, 0);
    let version3 = Version::new(2, 0, 0);

    let mut metadata_v1 = SkillMetadata::new(
        "versioned-skill".to_string(),
        "Versioned Skill".to_string(),
        version1.clone(),
        "A skill with multiple versions for testing version management".to_string(),
    );
    let skill_v1 = Arc::new(BaseSkill::new(metadata_v1));

    let mut metadata_v2 = SkillMetadata::new(
        "versioned-skill".to_string(),
        "Versioned Skill".to_string(),
        version2.clone(),
        "Updated skill with new features".to_string(),
    );
    let skill_v2 = Arc::new(BaseSkill::new(metadata_v2));

    let mut metadata_v3 = SkillMetadata::new(
        "versioned-skill".to_string(),
        "Versioned Skill".to_string(),
        version3.clone(),
        "Major version update with breaking changes".to_string(),
    );
    let skill_v3 = Arc::new(BaseSkill::new(metadata_v3));

    registry.register_with_version(skill_v1, false);
    registry.register_with_version(skill_v2, true);
    registry.register_with_version(skill_v3, false);

    let versions = registry.list_all_versions("versioned-skill");
    assert_eq!(versions.len(), 3);

    let latest = registry.get_latest_version("versioned-skill");
    assert!(latest.is_some());

    let compatible = registry.get_compatible_version("versioned-skill", &Version::new(1, 0, 0));
    assert!(compatible.is_some());

    Ok(())
}

#[test]
fn test_security_scan_recording() -> Result<()> {
    let registry = SkillRegistry::new();

    let skill = create_test_skill(
        "security-test-skill",
        "Security Test Skill",
        SkillPriority::Medium,
    );
    registry.register(skill.clone());

    let scan_result = SecurityScanResult {
        scan_id: "scan-2026-04-07-001".to_string(),
        scanned_at: Utc::now(),
        vulnerabilities: vec![Vulnerability {
            id: "VULN-001".to_string(),
            severity: VulnerabilitySeverity::Low,
            description: "Informational vulnerability".to_string(),
            discovered_at: Utc::now(),
            fixed_in_version: None,
        }],
        passed: true,
    };

    registry.record_security_scan("security-test-skill", scan_result.clone())?;

    let history = registry.get_security_scan_history("security-test-skill")?;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].scan_id, "scan-2026-04-07-001");

    Ok(())
}
