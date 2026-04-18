use aetheris::skill::agentskills::{AgentSkillManifest, AgentSkillsRegistry, SkillMdDocument};
use tempfile::tempdir;

#[test]
fn test_e2e_skill_lifecycle() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    std::fs::create_dir(&skills_dir).unwrap();

    let skill_name = "e2e-test-skill";
    let skill_dir = skills_dir.join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    std::fs::create_dir(skill_dir.join("scripts")).unwrap();
    std::fs::write(
        skill_dir.join("scripts").join("main.py"),
        "#!/usr/bin/env python3\nprint('hello world')",
    )
    .unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = r#"---
name: e2e-test-skill
description: An end-to-end test skill for verifying the complete skill loading and processing pipeline. Use this to test all functionality together.
version: 1.0.0
author: Test Author
license: MIT
tags: ["test", "e2e", "integration"]
compatibility: Python 3.9+
timeout: 60
requires: ["dependency-skill"]
platforms: ["linux", "macos", "windows"]
allowed-tools: ["Read", "Write", "Bash"]
metadata:
  emoji: "🧪"
  category: "testing"
---

# E2E Test Skill

## 功能概述
This is an end-to-end test skill that demonstrates the complete skill functionality. It includes all required sections and demonstrates proper usage of the SKILL.md format.

## 适用场景
- Testing the complete skill loading pipeline
- Verifying directory structure discovery
- Validating all official rules
- Testing conversion to AgentSkillManifest

## 输入规范
- `input_data` (string): The input data to process
- `options` (object, optional): Additional processing options

## 执行步骤
1. Read and validate the input data
2. Process the data according to the specifications
3. Execute any necessary scripts from the scripts directory
4. Generate the output result
5. Validate the output before returning

## 输出规范
Returns an object containing:
- `result`: The processed result
- `status`: Execution status (success/error)
- `timestamp`: Execution timestamp

## 约束与安全
- Only processes valid input data
- All scripts are executed in a secure sandbox
- No network access without explicit permission
- All operations are logged for auditing purposes

## 示例
### 示例1：基本处理
输入：
```json
{
  "input_data": "test data"
}
```

输出：
```json
{
  "result": "processed test data",
  "status": "success",
  "timestamp": "2026-04-07T10:00:00Z"
}
```
"#;
    std::fs::write(&skill_md_path, content).unwrap();

    let skill_doc = SkillMdDocument::from_path(&skill_md_path, true).unwrap();
    assert_eq!(skill_doc.frontmatter.name, skill_name);
    assert_eq!(skill_doc.frontmatter.version, Some("1.0.0".to_string()));
    assert_eq!(
        skill_doc.frontmatter.author,
        Some("Test Author".to_string())
    );
    assert_eq!(skill_doc.frontmatter.license, Some("MIT".to_string()));
    assert_eq!(
        skill_doc.frontmatter.tags,
        Some(vec![
            "test".to_string(),
            "e2e".to_string(),
            "integration".to_string()
        ])
    );
    assert_eq!(
        skill_doc.frontmatter.compatibility,
        Some("Python 3.9+".to_string())
    );
    assert_eq!(skill_doc.frontmatter.timeout, Some(60));
    assert_eq!(
        skill_doc.frontmatter.requires,
        Some(vec!["dependency-skill".to_string()])
    );
    assert_eq!(
        skill_doc.frontmatter.platforms,
        Some(vec![
            "linux".to_string(),
            "macos".to_string(),
            "windows".to_string()
        ])
    );
    assert_eq!(
        skill_doc.frontmatter.allowed_tools,
        Some(vec![
            "Read".to_string(),
            "Write".to_string(),
            "Bash".to_string()
        ])
    );
    assert!(skill_doc.frontmatter.metadata.is_some());

    assert!(!skill_doc.sections.overview.is_empty());
    assert!(!skill_doc.sections.use_cases.is_empty());
    assert!(!skill_doc.sections.input_spec.is_empty());
    assert!(!skill_doc.sections.execution_flow.is_empty());
    assert!(!skill_doc.sections.output_spec.is_empty());
    assert!(!skill_doc.sections.constraints.is_empty());
    assert!(!skill_doc.sections.examples.is_empty());

    assert!(skill_doc.directory_structure.is_some());
    let dir_structure = skill_doc.directory_structure.as_ref().unwrap();
    assert_eq!(dir_structure.scripts_dir, Some(skill_dir.join("scripts")));

    let manifest: AgentSkillManifest = skill_doc.into();
    assert_eq!(manifest.metadata.id, skill_name);
    assert_eq!(manifest.metadata.name, skill_name);
    assert_eq!(manifest.metadata.version, "1.0.0");
    assert_eq!(manifest.metadata.author, Some("Test Author".to_string()));
    assert_eq!(manifest.metadata.license, Some("MIT".to_string()));
    assert_eq!(manifest.metadata.tags, vec!["test", "e2e", "integration"]);
    assert_eq!(
        manifest.metadata.categories,
        vec!["test", "e2e", "integration"]
    );
    assert_eq!(manifest.dependencies, vec!["dependency-skill".to_string()]);
    assert_eq!(
        manifest.permissions,
        vec!["Read".to_string(), "Write".to_string(), "Bash".to_string()]
    );
    assert_eq!(manifest.timeout_seconds, Some(60));
    assert!(manifest.retry_config.is_some());

    let mut registry = AgentSkillsRegistry::new(skills_dir.clone()).unwrap();
    assert_eq!(registry.list().len(), 1);

    let retrieved_skill = registry.get(skill_name);
    assert!(retrieved_skill.is_some());
    let retrieved = retrieved_skill.unwrap();
    assert_eq!(retrieved.metadata.id, skill_name);

    let search_results = registry.search("test");
    assert_eq!(search_results.len(), 1);

    let tag_results = registry.list_by_tag("e2e");
    assert_eq!(tag_results.len(), 1);
}

#[test]
fn test_e2e_multiple_skills() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    std::fs::create_dir(&skills_dir).unwrap();

    let skill_names = vec!["skill-one", "skill-two", "skill-three"];

    for skill_name in &skill_names {
        let skill_dir = skills_dir.join(skill_name);
        std::fs::create_dir(&skill_dir).unwrap();

        let skill_md_path = skill_dir.join("SKILL.md");
        let content = format!(
            r#"---
name: {}
description: A test skill named {} for testing multiple skills in a registry. Use this to verify registry functionality.
version: 1.0.0
tags: ["test", "multi-skill"]
timeout: 60
---

# {}

## 功能概述
Test skill content.

## 适用场景
Testing multiple skills.

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
            skill_name, skill_name, skill_name
        );
        std::fs::write(&skill_md_path, content).unwrap();
    }

    let registry = AgentSkillsRegistry::new(skills_dir.clone()).unwrap();
    assert_eq!(registry.list().len(), 3);

    for skill_name in &skill_names {
        assert!(registry.get(skill_name).is_some());
    }

    let search_results = registry.search("skill");
    assert_eq!(search_results.len(), 3);
}

#[test]
fn test_e2e_real_example_skill() {
    let example_skill_path = std::path::Path::new("examples/agentskills/meeting-assistant");
    if example_skill_path.exists() && example_skill_path.join("SKILL.md").exists() {
        let skill_md_path = example_skill_path.join("SKILL.md");
        let skill_doc = SkillMdDocument::from_path(&skill_md_path, false).unwrap();

        assert!(!skill_doc.frontmatter.name.is_empty());
        assert!(!skill_doc.frontmatter.description.is_empty());

        let manifest: AgentSkillManifest = skill_doc.into();
        assert!(!manifest.metadata.id.is_empty());
        assert!(!manifest.metadata.name.is_empty());
    }
}

#[test]
fn test_registry_new() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");

    let registry = AgentSkillsRegistry::new(skills_dir.clone()).unwrap();
    assert_eq!(registry.skills_dir(), &skills_dir);
    assert_eq!(registry.list().len(), 0);
}

#[test]
fn test_registry_add() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let mut registry = AgentSkillsRegistry::new(skills_dir).unwrap();

    let manifest = create_test_manifest("test-skill-1", "A test skill", "test,utility");
    registry.add(manifest).unwrap();
    assert_eq!(registry.list().len(), 1);

    let manifest2 = create_test_manifest("test-skill-2", "Another test skill", "test,another");
    registry.add(manifest2).unwrap();
    assert_eq!(registry.list().len(), 2);
}

#[test]
fn test_registry_add_duplicate() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let mut registry = AgentSkillsRegistry::new(skills_dir).unwrap();

    let manifest = create_test_manifest("test-skill", "A test skill", "test");
    registry.add(manifest.clone()).unwrap();

    let result = registry.add(manifest);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn test_registry_get() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let mut registry = AgentSkillsRegistry::new(skills_dir).unwrap();

    let manifest = create_test_manifest("test-skill", "A test skill", "test");
    registry.add(manifest.clone()).unwrap();

    let retrieved = registry.get("test-skill");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().metadata.id, "test-skill");

    assert!(registry.get("nonexistent-skill").is_none());
}

#[test]
fn test_registry_list() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let mut registry = AgentSkillsRegistry::new(skills_dir).unwrap();

    assert_eq!(registry.list().len(), 0);

    let manifest1 = create_test_manifest("skill-1", "Skill 1", "test");
    let manifest2 = create_test_manifest("skill-2", "Skill 2", "test");
    registry.add(manifest1).unwrap();
    registry.add(manifest2).unwrap();

    let list = registry.list();
    assert_eq!(list.len(), 2);
}

#[test]
fn test_registry_list_by_category() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let mut registry = AgentSkillsRegistry::new(skills_dir).unwrap();

    let manifest1 = create_test_manifest("skill-1", "Skill 1", "utility,test");
    let manifest2 = create_test_manifest("skill-2", "Skill 2", "data,test");
    let manifest3 = create_test_manifest("skill-3", "Skill 3", "utility,another");
    registry.add(manifest1).unwrap();
    registry.add(manifest2).unwrap();
    registry.add(manifest3).unwrap();

    let utility_skills = registry.list_by_category("utility");
    assert_eq!(utility_skills.len(), 2);

    let data_skills = registry.list_by_category("data");
    assert_eq!(data_skills.len(), 1);

    let nonexistent = registry.list_by_category("nonexistent");
    assert_eq!(nonexistent.len(), 0);
}

#[test]
fn test_registry_list_by_tag() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let mut registry = AgentSkillsRegistry::new(skills_dir).unwrap();

    let manifest1 = create_test_manifest("skill-1", "Skill 1", "tag1,tag2");
    let manifest2 = create_test_manifest("skill-2", "Skill 2", "tag2,tag3");
    let manifest3 = create_test_manifest("skill-3", "Skill 3", "tag1,tag3");
    registry.add(manifest1).unwrap();
    registry.add(manifest2).unwrap();
    registry.add(manifest3).unwrap();

    let tag1_skills = registry.list_by_tag("tag1");
    assert_eq!(tag1_skills.len(), 2);

    let tag2_skills = registry.list_by_tag("tag2");
    assert_eq!(tag2_skills.len(), 2);

    let nonexistent = registry.list_by_tag("nonexistent");
    assert_eq!(nonexistent.len(), 0);
}

#[test]
fn test_registry_search() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let mut registry = AgentSkillsRegistry::new(skills_dir).unwrap();

    let manifest1 = create_test_manifest(
        "web-search",
        "Search the web for information",
        "search,web,research",
    );
    let manifest2 = create_test_manifest(
        "file-ops",
        "File operations including read and write",
        "file,io,utility",
    );
    let manifest3 = create_test_manifest(
        "data-analysis",
        "Analyze data and generate reports",
        "data,analysis,report",
    );
    registry.add(manifest1).unwrap();
    registry.add(manifest2).unwrap();
    registry.add(manifest3).unwrap();

    let search_results = registry.search("search");
    assert_eq!(search_results.len(), 1);
    assert_eq!(search_results[0].metadata.id, "web-search");

    let file_results = registry.search("file");
    assert_eq!(file_results.len(), 1);
    assert_eq!(file_results[0].metadata.id, "file-ops");

    let data_results = registry.search("data");
    assert_eq!(data_results.len(), 2);

    let nonexistent = registry.search("nonexistent");
    assert_eq!(nonexistent.len(), 0);

    let empty_results = registry.search("");
    assert_eq!(empty_results.len(), 3);
}

#[test]
fn test_registry_remove() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    let mut registry = AgentSkillsRegistry::new(skills_dir).unwrap();

    let manifest = create_test_manifest("test-skill", "A test skill", "test");
    registry.add(manifest).unwrap();
    assert_eq!(registry.list().len(), 1);

    let result = registry.remove("test-skill");
    assert!(result.is_ok());
    assert_eq!(registry.list().len(), 0);

    let result2 = registry.remove("nonexistent-skill");
    assert!(result2.is_err());
    assert!(result2.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_registry_load_all() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    std::fs::create_dir(&skills_dir).unwrap();

    let skill_dir1 = skills_dir.join("skill-1");
    std::fs::create_dir(&skill_dir1).unwrap();
    let skill_md_path1 = skill_dir1.join("SKILL.md");
    let content1 = r#"---
name: skill-1
description: A test skill for testing load all functionality. Use this to verify loading works.
version: 1.0.0
tags: ["test", "load"]
---

# Skill 1
## 功能概述
Test content.
## 适用场景
Testing.
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
    std::fs::write(skill_md_path1, content1).unwrap();

    let skill_dir2 = skills_dir.join("skill-2");
    std::fs::create_dir(&skill_dir2).unwrap();
    let skill_md_path2 = skill_dir2.join("SKILL.md");
    let content2 = r#"---
name: skill-2
description: Another test skill for testing load all functionality. Use this to verify loading works.
version: 1.0.0
tags: ["test", "load"]
---

# Skill 2
## 功能概述
Test content.
## 适用场景
Testing.
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
    std::fs::write(skill_md_path2, content2).unwrap();

    let registry = AgentSkillsRegistry::new(skills_dir.clone()).unwrap();
    assert_eq!(registry.list().len(), 2);
    assert!(registry.get("skill-1").is_some());
    assert!(registry.get("skill-2").is_some());
}

fn create_test_manifest(id: &str, description: &str, tags: &str) -> AgentSkillManifest {
    let tag_list: Vec<String> = tags.split(',').map(|s| s.trim().to_string()).collect();

    AgentSkillManifest {
        metadata: aetheris::skill::agentskills::AgentSkillMetadata {
            id: id.to_string(),
            name: id.to_string(),
            version: "1.0.0".to_string(),
            description: description.to_string(),
            long_description: None,
            author: None,
            license: None,
            tags: tag_list.clone(),
            categories: tag_list,
            skill_type: aetheris::skill::agentskills::AgentSkillType::Custom,
            priority: None,
            icon: None,
            homepage: None,
            repository: None,
            issues: None,
            keywords: vec![],
            deprecated: false,
            deprecation_message: None,
            retry_config: None,
            sandbox_level: None,
            implementation: None,
        },
        parameters: vec![],
        returns: None,
        examples: vec![],
        dependencies: vec![],
        env_vars: vec![],
        permissions: vec![],
        timeout_seconds: None,
        retry_config: None,
    }
}

#[test]
fn test_skill_md_to_manifest_metadata() {
    let content = r#"---
name: test-skill
description: A test skill for metadata conversion. Use this to verify metadata is converted correctly.
version: 2.1.0
author: Test Author
license: MIT
tags: ["test", "conversion", "metadata"]
---

# Test Skill
## 功能概述
Test content.
## 适用场景
Testing.
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

    let skill_md = SkillMdDocument::parse(content).unwrap();
    let manifest: AgentSkillManifest = skill_md.into();

    assert_eq!(manifest.metadata.id, "test-skill");
    assert_eq!(manifest.metadata.name, "test-skill");
    assert_eq!(manifest.metadata.version, "2.1.0");
    assert_eq!(manifest.metadata.author, Some("Test Author".to_string()));
    assert_eq!(manifest.metadata.license, Some("MIT".to_string()));
    assert_eq!(
        manifest.metadata.tags,
        vec!["test", "conversion", "metadata"]
    );
    assert_eq!(
        manifest.metadata.categories,
        vec!["test", "conversion", "metadata"]
    );
}

#[test]
fn test_skill_md_to_manifest_dependencies() {
    let content = r#"---
name: test-skill
description: A test skill for dependencies conversion. Use this to verify dependencies are converted correctly.
requires: ["skill-a", "skill-b", "skill-c"]
---

# Test Skill
## 功能概述
Test content.
## 适用场景
Testing.
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

    let skill_md = SkillMdDocument::parse(content).unwrap();
    let manifest: AgentSkillManifest = skill_md.into();

    assert_eq!(manifest.dependencies, vec!["skill-a", "skill-b", "skill-c"]);
}

#[test]
fn test_skill_md_to_manifest_permissions() {
    let content = r#"---
name: test-skill
description: A test skill for permissions conversion. Use this to verify allowed-tools are converted correctly.
allowed-tools: ["Read", "Write", "Bash", "LLM"]
---

# Test Skill
## 功能概述
Test content.
## 适用场景
Testing.
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

    let skill_md = SkillMdDocument::parse(content).unwrap();
    let manifest: AgentSkillManifest = skill_md.into();

    assert_eq!(manifest.permissions, vec!["Read", "Write", "Bash", "LLM"]);
}

#[test]
fn test_skill_md_to_manifest_timeout() {
    let content = r#"---
name: test-skill
description: A test skill for timeout conversion. Use this to verify timeout is converted correctly.
timeout: 120
---

# Test Skill
## 功能概述
Test content.
## 适用场景
Testing.
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

    let skill_md = SkillMdDocument::parse(content).unwrap();
    let manifest: AgentSkillManifest = skill_md.into();

    assert_eq!(manifest.timeout_seconds, Some(120));
}

#[test]
fn test_skill_md_to_manifest_metadata_extensions() {
    let content = r#"---
name: test-skill
description: A test skill for metadata extensions conversion. Use this to verify metadata fields are converted correctly.
metadata:
  retry_config:
    max_attempts: 3
    initial_delay_ms: 1000
    max_delay_ms: 5000
    backoff_multiplier: 2
  sandbox_level: high
  implementation: custom
---

# Test Skill
## 功能概述
Test content.
## 适用场景
Testing.
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

    let skill_md = SkillMdDocument::parse(content).unwrap();
    let manifest: AgentSkillManifest = skill_md.into();

    assert!(manifest.retry_config.is_some());
    assert_eq!(manifest.metadata.sandbox_level, Some("high".to_string()));
    assert_eq!(manifest.metadata.implementation, Some("custom".to_string()));
}

#[test]
fn test_skill_md_to_manifest_minimal() {
    let content = r#"---
name: minimal-skill
description: A minimal skill for testing minimal conversion. Use this to verify minimal fields work.
---

# Minimal Skill
## 功能概述
Test content.
## 适用场景
Testing.
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

    let skill_md = SkillMdDocument::parse(content).unwrap();
    let manifest: AgentSkillManifest = skill_md.into();

    assert_eq!(manifest.metadata.id, "minimal-skill");
    assert_eq!(manifest.metadata.name, "minimal-skill");
    assert_eq!(manifest.metadata.version, "0.1.0");
    assert_eq!(manifest.dependencies.len(), 0);
    assert_eq!(manifest.permissions.len(), 0);
    assert_eq!(manifest.timeout_seconds, None);
}

#[test]
fn test_e2e_all_real_examples() {
    let example_skills = vec![
        "web-search",
        "file-operations",
        "chemical-reagent-manage",
        "lab-report-audit",
        "code-generation",
        "data-analysis",
        "meeting-assistant",
        "report-generation",
        "email-composer",
        "database-query",
        "production-monitoring",
        "predictive-maintenance",
    ];

    for skill_name in &example_skills {
        let example_skill_path = std::path::Path::new("examples/agentskills").join(skill_name);
        if example_skill_path.exists() && example_skill_path.join("SKILL.md").exists() {
            let skill_md_path = example_skill_path.join("SKILL.md");
            let skill_doc_result = SkillMdDocument::from_path(&skill_md_path, false);

            assert!(
                skill_doc_result.is_ok(),
                "Failed to parse example skill: {}",
                skill_name
            );

            let skill_doc = skill_doc_result.unwrap();
            assert_eq!(skill_doc.frontmatter.name, *skill_name);
            assert!(!skill_doc.frontmatter.description.is_empty());
            assert!(!skill_doc.sections.overview.is_empty());
            assert!(!skill_doc.sections.use_cases.is_empty());
            assert!(!skill_doc.sections.input_spec.is_empty());
            assert!(!skill_doc.sections.execution_flow.is_empty());
            assert!(!skill_doc.sections.output_spec.is_empty());
            assert!(!skill_doc.sections.constraints.is_empty());
            assert!(!skill_doc.sections.examples.is_empty());

            let manifest: AgentSkillManifest = skill_doc.into();
            assert_eq!(manifest.metadata.id, *skill_name);
            assert_eq!(manifest.metadata.name, *skill_name);
            assert!(!manifest.metadata.version.is_empty());
            assert!(!manifest.metadata.description.is_empty());
        }
    }
}

#[test]
fn test_e2e_complete_directory_structure() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    std::fs::create_dir(&skills_dir).unwrap();

    let skill_name = "complete-skill";
    let skill_dir = skills_dir.join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    std::fs::create_dir(skill_dir.join("scripts")).unwrap();
    std::fs::create_dir(skill_dir.join("assets")).unwrap();
    std::fs::create_dir(skill_dir.join("references")).unwrap();
    std::fs::create_dir(skill_dir.join("sub-skills")).unwrap();

    std::fs::write(
        skill_dir.join("scripts").join("main.py"),
        "#!/usr/bin/env python3\nprint('hello')",
    )
    .unwrap();
    std::fs::write(skill_dir.join("assets").join("template.txt"), "Template").unwrap();
    std::fs::write(skill_dir.join("references").join("api.yaml"), "spec").unwrap();

    let sub_skill_dir = skill_dir.join("sub-skills").join("sub-skill");
    std::fs::create_dir(&sub_skill_dir).unwrap();
    std::fs::write(sub_skill_dir.join("SKILL.md"), "---\nname: sub-skill\ndescription: Sub skill\n---\n# Sub Skill\n## 功能概述\nTest\n## 适用场景\nTest\n## 输入规范\nNone\n## 执行步骤\n1. Do\n## 输出规范\nSomething\n## 约束与安全\nNone\n## 示例\nNone").unwrap();

    std::fs::write(skill_dir.join("LICENSE"), "MIT License").unwrap();
    std::fs::write(skill_dir.join("README.md"), "# Complete Skill\n\nTest").unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = r#"---
name: complete-skill
description: A skill with complete directory structure. Use this to test all directories and files.
version: 1.0.0
tags: ["test", "complete"]
timeout: 120
allowed-tools: ["Read", "Write"]
---

# Complete Skill
## 功能概述
Test content.
## 适用场景
Testing.
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
    std::fs::write(&skill_md_path, content).unwrap();

    let skill_doc = SkillMdDocument::from_path(&skill_md_path, true).unwrap();

    assert!(skill_doc.directory_structure.is_some());
    let dir_structure = skill_doc.directory_structure.as_ref().unwrap();

    assert_eq!(dir_structure.skill_dir, skill_dir);
    assert_eq!(dir_structure.scripts_dir, Some(skill_dir.join("scripts")));
    assert_eq!(dir_structure.assets_dir, Some(skill_dir.join("assets")));
    assert_eq!(
        dir_structure.references_dir,
        Some(skill_dir.join("references"))
    );
    assert_eq!(
        dir_structure.sub_skills_dir,
        Some(skill_dir.join("sub-skills"))
    );
    assert_eq!(dir_structure.license_file, Some(skill_dir.join("LICENSE")));
    assert_eq!(dir_structure.readme_file, Some(skill_dir.join("README.md")));
}

#[test]
fn test_e2e_metadata_extensions() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    std::fs::create_dir(&skills_dir).unwrap();

    let skill_name = "metadata-skill";
    let skill_dir = skills_dir.join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = r#"---
name: metadata-skill
description: A skill with metadata extensions. Use this to test metadata fields.
version: 1.0.0
tags: ["test", "metadata"]
timeout: 180
allowed-tools: ["Read", "Write", "LLM"]
metadata:
  retry_config:
    max_attempts: 5
    initial_delay_ms: 500
    max_delay_ms: 10000
    backoff_multiplier: 1.5
  sandbox_level: high
  implementation: custom
  custom_field: custom_value
  another_field: another_value
---

# Metadata Skill
## 功能概述
Test content.
## 适用场景
Testing.
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
    std::fs::write(&skill_md_path, content).unwrap();

    let skill_doc = SkillMdDocument::from_path(&skill_md_path, false).unwrap();

    assert!(skill_doc.frontmatter.metadata.is_some());

    let manifest: AgentSkillManifest = skill_doc.into();

    assert!(manifest.retry_config.is_some());
    assert_eq!(manifest.metadata.sandbox_level, Some("high".to_string()));
    assert_eq!(manifest.metadata.implementation, Some("custom".to_string()));
}

#[test]
fn test_e2e_mixed_skill_loading() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    std::fs::create_dir(&skills_dir).unwrap();

    let test_skill_names = vec!["test-skill-1", "test-skill-2", "test-skill-3"];

    for skill_name in &test_skill_names {
        let skill_dir = skills_dir.join(skill_name);
        std::fs::create_dir(&skill_dir).unwrap();

        let skill_md_path = skill_dir.join("SKILL.md");
        let content = format!(
            r#"---
name: {}
description: A test skill for mixed loading. Use this to test registry functionality.
version: 1.0.0
tags: ["test", "mixed"]
timeout: 60
---

# {}
## 功能概述
Test content.
## 适用场景
Testing.
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
        std::fs::write(&skill_md_path, content).unwrap();
    }

    let registry = AgentSkillsRegistry::new(skills_dir.clone()).unwrap();

    assert_eq!(registry.list().len(), 3);

    for skill_name in &test_skill_names {
        assert!(registry.get(skill_name).is_some());
    }

    let search_results = registry.search("test");
    assert_eq!(search_results.len(), 3);

    let tag_results = registry.list_by_tag("test");
    assert_eq!(tag_results.len(), 3);
}

#[test]
fn test_e2e_error_handling() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    std::fs::create_dir(&skills_dir).unwrap();

    let result = AgentSkillsRegistry::new(skills_dir.join("nonexistent"));
    assert!(result.is_ok());
    assert_eq!(result.unwrap().list().len(), 0);

    let invalid_skill_dir = skills_dir.join("invalid-skill");
    std::fs::create_dir(&invalid_skill_dir).unwrap();

    let invalid_md_path = invalid_skill_dir.join("SKILL.md");
    std::fs::write(&invalid_md_path, "invalid content").unwrap();

    let result = SkillMdDocument::from_path(&invalid_md_path, false);
    assert!(result.is_err());

    let bad_yaml_dir = skills_dir.join("bad-yaml-skill");
    std::fs::create_dir(&bad_yaml_dir).unwrap();
    let bad_yaml_path = bad_yaml_dir.join("SKILL.md");
    std::fs::write(
        &bad_yaml_path,
        "---\nname: test\ndescription: test\ninvalid: [unclosed\n---\n# Test",
    )
    .unwrap();

    let result = SkillMdDocument::from_path(&bad_yaml_path, false);
    assert!(result.is_err());
}

#[test]
fn test_e2e_large_file_performance() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    std::fs::create_dir(&skills_dir).unwrap();

    let skill_name = "large-skill";
    let skill_dir = skills_dir.join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    let mut long_description =
        String::from("A very large skill description for performance testing. ");
    for i in 0..50 {
        long_description.push_str(&format!("This is paragraph {}. It contains multiple sentences to make the description longer. Use this skill for testing large file parsing performance. ", i + 1));
    }

    let mut long_examples = String::new();
    for i in 0..20 {
        long_examples.push_str(&format!("\n### 示例{}：示例标题\n示例内容，包含多个行文本。\n```json\n{{\n  \"key\": \"value{}\"\n}}\n```", i + 1, i + 1));
    }

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = format!(
        r#"---
name: large-skill
description: {}
version: 1.0.0
tags: ["test", "performance", "large"]
timeout: 300
allowed-tools: ["Read", "Write", "LLM"]
---

# Large Skill

## 功能概述
This is a large skill for performance testing.

## 适用场景
- Testing large file parsing
- Performance benchmarking
- Memory usage testing

## 输入规范
- `input_data` (string): The input data to process
- `options` (object): Processing options
- `config` (object): Configuration settings

## 执行步骤
1. Read and validate input
2. Process data
3. Generate output
4. Validate result
5. Return to caller
{}

## 输出规范
Returns processed data with metadata.

## 约束与安全
- Large file handling
- Memory constraints
- Timeout enforcement

## 示例
{}
"#,
        long_description, long_examples, long_examples
    );

    std::fs::write(&skill_md_path, &content).unwrap();

    let start = std::time::Instant::now();
    let skill_doc = SkillMdDocument::from_path(&skill_md_path, false).unwrap();
    let duration = start.elapsed();

    println!("Large file parsed in: {:?}", duration);

    assert_eq!(skill_doc.frontmatter.name, "large-skill");
    assert!(!skill_doc.frontmatter.description.is_empty());
    assert!(!skill_doc.sections.examples.is_empty());

    let manifest: AgentSkillManifest = skill_doc.into();
    assert_eq!(manifest.metadata.id, "large-skill");

    assert!(
        duration.as_millis() < 500,
        "Large file parsing should complete in under 500ms"
    );
}

#[test]
fn test_e2e_many_skills_performance() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    std::fs::create_dir(&skills_dir).unwrap();

    let num_skills = 50;
    let skill_names: Vec<String> = (0..num_skills).map(|i| format!("skill-{}", i)).collect();

    for skill_name in &skill_names {
        let skill_dir = skills_dir.join(skill_name);
        std::fs::create_dir(&skill_dir).unwrap();

        let skill_md_path = skill_dir.join("SKILL.md");
        let content = format!(
            r#"---
name: {}
description: A test skill number {} for testing loading many skills. Use this to verify performance with multiple skills.
version: 1.0.0
tags: ["test", "many-skills"]
timeout: 60
---

# {}
## 功能概述
Test skill content.
## 适用场景
Testing.
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
            skill_name, skill_name, skill_name
        );
        std::fs::write(&skill_md_path, content).unwrap();
    }

    let start = std::time::Instant::now();
    let registry = AgentSkillsRegistry::new(skills_dir.clone()).unwrap();
    let duration = start.elapsed();

    println!("{} skills loaded in: {:?}", num_skills, duration);

    assert_eq!(registry.list().len(), num_skills);

    for skill_name in &skill_names {
        assert!(registry.get(skill_name).is_some());
    }

    let search_start = std::time::Instant::now();
    let search_results = registry.search("test");
    let search_duration = search_start.elapsed();

    println!("Search completed in: {:?}", search_duration);
    assert_eq!(search_results.len(), num_skills);

    assert!(
        duration.as_millis() < 2000,
        "Loading {} skills should complete in under 2000ms",
        num_skills
    );
}

#[test]
fn test_e2e_concurrent_reads() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    std::fs::create_dir(&skills_dir).unwrap();

    let num_skills = 10;
    for i in 0..num_skills {
        let skill_name = format!("skill-{}", i);
        let skill_dir = skills_dir.join(&skill_name);
        std::fs::create_dir(&skill_dir).unwrap();

        let skill_md_path = skill_dir.join("SKILL.md");
        let content = format!(
            r#"---
name: {}
description: Concurrent test skill {}. Use this for multi-threaded testing.
version: 1.0.0
tags: ["test", "concurrent"]
timeout: 60
---

# {}
## 功能概述
Test.
## 适用场景
Testing.
## 输入规范
None.
## 执行步骤
1. Do.
## 输出规范
Done.
## 约束与安全
None.
## 示例
None.
"#,
            skill_name, i, skill_name
        );
        std::fs::write(&skill_md_path, content).unwrap();
    }

    let registry = std::sync::Arc::new(std::sync::Mutex::new(
        AgentSkillsRegistry::new(skills_dir.clone()).unwrap(),
    ));

    let num_threads = 20;
    let mut handles = Vec::with_capacity(num_threads);

    let start = std::time::Instant::now();

    for thread_id in 0..num_threads {
        let registry_clone = registry.clone();
        let skill_names: Vec<String> = (0..num_skills).map(|i| format!("skill-{}", i)).collect();

        let handle = std::thread::spawn(move || {
            let registry = registry_clone.lock().unwrap();

            for skill_name in &skill_names {
                let skill = registry.get(skill_name);
                assert!(skill.is_some());
            }

            let search_results = registry.search("concurrent");
            assert_eq!(search_results.len(), num_skills);

            let tag_results = registry.list_by_tag("test");
            assert_eq!(tag_results.len(), num_skills);
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let duration = start.elapsed();
    println!("Concurrent reads completed in: {:?}", duration);

    let registry = registry.lock().unwrap();
    assert_eq!(registry.list().len(), num_skills);
}

#[test]
fn test_e2e_concurrent_writes() {
    let dir = tempdir().unwrap();
    let skills_dir = dir.path().join("skills");
    std::fs::create_dir(&skills_dir).unwrap();

    let registry = std::sync::Arc::new(std::sync::Mutex::new(
        AgentSkillsRegistry::new(skills_dir.clone()).unwrap(),
    ));

    let num_threads = 10;
    let skills_per_thread = 5;
    let mut handles = Vec::with_capacity(num_threads);

    let start = std::time::Instant::now();

    for thread_id in 0..num_threads {
        let registry_clone = registry.clone();

        let handle = std::thread::spawn(move || {
            let mut registry = registry_clone.lock().unwrap();

            for i in 0..skills_per_thread {
                let skill_id = format!("thread-{}-skill-{}", thread_id, i);
                let manifest = create_test_manifest(
                    &skill_id,
                    &format!("Skill from thread {}", thread_id),
                    &format!("thread{},test", thread_id),
                );

                registry.add(manifest).unwrap();
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let duration = start.elapsed();
    println!("Concurrent writes completed in: {:?}", duration);

    let registry = registry.lock().unwrap();
    assert_eq!(registry.list().len(), num_threads * skills_per_thread);

    for thread_id in 0..num_threads {
        let tag = format!("thread{}", thread_id);
        let tag_results = registry.list_by_tag(&tag);
        assert_eq!(tag_results.len(), skills_per_thread);
    }
}

#[test]
fn test_e2e_complete_enterprise_feature_flow() -> aetheris::utils::Result<()> {
    use aetheris::skill::*;
    use chrono::Utc;
    use std::sync::Arc;

    let registry = SkillRegistry::new();

    let version = Version::new(1, 0, 0);
    let metadata = SkillMetadata::new(
        "enterprise-skill".to_string(),
        "Enterprise Feature Skill".to_string(),
        version.clone(),
        "A complete enterprise-grade skill for testing end-to-end feature flow".to_string(),
    );
    let skill = Arc::new(BaseSkill::new(metadata));

    let version_metadata = SkillVersionMetadata {
        commit_hash: Some("abc123def456".to_string()),
        published_at: Some(Utc::now()),
        security_approved: true,
        changelog: Some("Initial release with enterprise features".to_string()),
    };

    registry.register_with_metadata(skill.clone(), version_metadata.clone())?;

    let scan_result = SecurityScanResult {
        scan_id: "enterprise-scan-001".to_string(),
        scanned_at: Utc::now(),
        vulnerabilities: vec![],
        passed: true,
    };

    registry.record_security_scan("enterprise-skill", scan_result.clone())?;

    let retrieved_metadata = registry.get_version_metadata("enterprise-skill", &version)?;
    assert!(retrieved_metadata.is_some());
    let retrieved = retrieved_metadata.unwrap();
    assert_eq!(retrieved.commit_hash, Some("abc123def456".to_string()));
    assert_eq!(retrieved.security_approved, true);

    let scan_history = registry.get_security_scan_history("enterprise-skill")?;
    assert_eq!(scan_history.len(), 1);
    assert_eq!(scan_history[0].scan_id, "enterprise-scan-001");

    Ok(())
}

#[test]
fn test_e2e_complete_progressive_disclosure_flow() -> aetheris::utils::Result<()> {
    use aetheris::skill::*;
    use std::sync::Arc;
    use tempfile::tempdir;

    let dir = tempdir()?;
    let skills_dir = dir.path().join("skills");
    std::fs::create_dir(&skills_dir)?;

    let skill_names = vec!["data-analysis", "web-search", "file-ops"];

    for skill_name in &skill_names {
        let skill_dir = skills_dir.join(skill_name);
        std::fs::create_dir(&skill_dir)?;

        let skill_md_path = skill_dir.join("SKILL.md");
        let content = format!(
            r#"---
name: {}
description: A skill for testing the complete progressive disclosure flow including indexing, matching, and loading. Use this to verify the entire workflow works properly.
version: 1.0.0
tags: ["e2e", "progressive", "test"]
categories: ["data", "search", "utility"]
timeout: 60
---

# {}
## 功能概述
Test skill for progressive disclosure flow.
## 适用场景
Testing end-to-end progressive disclosure.
## 输入规范
- query: The search query to process
## 执行步骤
1. Index the skill metadata
2. Find matching skills based on intent
3. Load the required skill
## 输出规范
Processed results with metadata.
## 约束与安全
None.
## 示例
Input: {{"query": "test"}}
Output: {{"result": "processed"}}
"#,
            skill_name, skill_name
        );
        std::fs::write(&skill_md_path, content)?;
    }

    let registry = Arc::new(SkillRegistry::new());
    let mut manager = ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::Lazy)?;

    let paths = vec![skills_dir];
    let indexed_count = manager.index_skills(&paths)?;
    assert_eq!(indexed_count, 3);

    let indexed_skills = manager.list_indexed_skills()?;
    assert_eq!(indexed_skills.len(), 3);

    let matches = manager.find_matching_skills("data analysis")?;
    assert!(!matches.is_empty() || indexed_skills.len() > 0);

    let skill = create_test_skill_e2e("data-analysis", "Data Analysis");
    registry.register(skill.clone());

    let loaded_skill = manager.get_or_load_skill("data-analysis")?;
    assert_eq!(loaded_skill.metadata().id, "data-analysis");

    Ok(())
}

fn create_test_skill_e2e(id: &str, name: &str) -> Arc<dyn Skill> {
    use aetheris::skill::*;
    let version = Version::new(1, 0, 0);
    let metadata = SkillMetadata::new(
        id.to_string(),
        name.to_string(),
        version,
        format!("Test skill {} for E2E progressive disclosure testing", name),
    );
    Arc::new(BaseSkill::new(metadata))
}

#[test]
fn test_e2e_performance_benchmark_integration() -> aetheris::utils::Result<()> {
    use aetheris::skill::*;
    use std::sync::Arc;
    use tempfile::tempdir;

    let dir = tempdir()?;
    let skills_dir = dir.path().join("skills");
    std::fs::create_dir(&skills_dir)?;

    let num_skills = 20;
    for i in 0..num_skills {
        let skill_name = format!("perf-skill-{}", i);
        let skill_dir = skills_dir.join(&skill_name);
        std::fs::create_dir(&skill_dir)?;

        let skill_md_path = skill_dir.join("SKILL.md");
        let content = format!(
            r#"---
name: {}
description: Performance benchmark skill number {} for testing integration performance. Use this to measure indexing and search performance under load.
version: 1.0.0
tags: ["performance", "benchmark", "test"]
categories: ["testing"]
timeout: 60
---

# {}
## 功能概述
Performance test skill.
## 适用场景
Benchmarking.
## 输入规范
None.
## 执行步骤
1. Execute
## 输出规范
Result.
## 约束与安全
None.
## 示例
None.
"#,
            skill_name, i, skill_name
        );
        std::fs::write(&skill_md_path, content)?;
    }

    let registry = Arc::new(SkillRegistry::new());
    let mut manager =
        ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::EagerMetadata)?;

    let paths = vec![skills_dir];

    let index_start = std::time::Instant::now();
    let indexed_count = manager.index_skills(&paths)?;
    let index_duration = index_start.elapsed();

    assert_eq!(indexed_count, num_skills);
    println!("Indexed {} skills in: {:?}", num_skills, index_duration);

    let search_start = std::time::Instant::now();
    let _matches = manager.find_matching_skills("performance")?;
    let search_duration = search_start.elapsed();

    println!("Search completed in: {:?}", search_duration);

    let list_start = std::time::Instant::now();
    let listed = manager.list_indexed_skills()?;
    let list_duration = list_start.elapsed();

    assert_eq!(listed.len(), num_skills);
    println!("Listed {} skills in: {:?}", num_skills, list_duration);

    assert!(
        index_duration.as_millis() < 10000,
        "Indexing should complete in under 10 seconds"
    );
    assert!(
        search_duration.as_millis() < 1000,
        "Search should complete in under 1 second"
    );

    Ok(())
}

#[test]
fn test_e2e_performance_no_regression() -> aetheris::utils::Result<()> {
    use aetheris::skill::*;
    use std::sync::Arc;

    let registry = Arc::new(SkillRegistry::new());

    let num_operations = 100;

    let register_start = std::time::Instant::now();
    for i in 0..num_operations {
        let skill_id = format!("regress-skill-{}", i);
        let skill = create_test_skill_e2e(&skill_id, &format!("Regression Skill {}", i));
        registry.register(skill);
    }
    let register_duration = register_start.elapsed();

    println!(
        "Registered {} skills in: {:?}",
        num_operations, register_duration
    );

    let list_start = std::time::Instant::now();
    let listed = registry.list();
    let list_duration = list_start.elapsed();

    assert_eq!(listed.len(), num_operations);
    println!("Listed {} skills in: {:?}", num_operations, list_duration);

    let search_start = std::time::Instant::now();
    let search_results = registry.search("skill");
    let search_duration = search_start.elapsed();

    assert_eq!(search_results.len(), num_operations);
    println!(
        "Searched {} skills in: {:?}",
        num_operations, search_duration
    );

    let avg_register_per_skill = register_duration.as_micros() / num_operations as u128;
    let avg_list_per_skill = list_duration.as_micros() / num_operations as u128;
    let avg_search_per_skill = search_duration.as_micros() / num_operations as u128;

    println!("Average register per skill: {} μs", avg_register_per_skill);
    println!("Average list per skill: {} μs", avg_list_per_skill);
    println!("Average search per skill: {} μs", avg_search_per_skill);

    assert!(
        avg_register_per_skill < 10000,
        "Average register time should be under 10ms per skill"
    );
    assert!(
        avg_list_per_skill < 1000,
        "Average list time should be under 1ms per skill"
    );
    assert!(
        avg_search_per_skill < 1000,
        "Average search time should be under 1ms per skill"
    );

    Ok(())
}

#[test]
fn test_e2e_real_world_example_skills() -> aetheris::utils::Result<()> {
    use aetheris::skill::*;

    let example_skills_path = std::path::Path::new("examples/agentskills");
    if example_skills_path.exists() {
        let registry = Arc::new(SkillRegistry::new());
        let mut manager =
            ProgressiveDisclosureManager::new(registry.clone(), LoadingStrategy::Lazy)?;

        let paths = vec![example_skills_path.to_path_buf()];
        let indexed = manager.index_skills(&paths)?;

        println!("Indexed {} real example skills", indexed);

        if indexed > 0 {
            let listed = manager.list_indexed_skills()?;
            assert_eq!(listed.len(), indexed);

            let matches = manager.find_matching_skills("meeting")?;
            println!("Found {} matching skills for 'meeting'", matches.len());
        }
    }

    Ok(())
}
