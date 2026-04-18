use aetheris::skill::agentskills::SkillMdDocument;
use tempfile::tempdir;

fn create_valid_skill_content(name: &str, description: &str) -> String {
    format!(
        r#"---
name: {}
description: {}
version: 1.0.0
license: MIT
tags: ["test", "example"]
timeout: 300
allowed-tools: ["Read", "Write"]
---

# Test Skill

## 功能概述
A test skill for validation testing.

## 适用场景
Testing validation rules.

## 输入规范
No input required.

## 执行步骤
1. Step one
2. Step two

## 输出规范
Some output.

## 约束与安全
No constraints.

## 示例
Some examples.
"#,
        name, description
    )
}

#[test]
fn test_validate_description_valid() {
    let dir = tempdir().unwrap();
    let skill_name = "test-skill";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let description = "A valid skill description with at least 50 characters. Use this for testing description validation.";
    let content = create_valid_skill_content(skill_name, description);
    std::fs::write(&skill_md_path, content).unwrap();

    let result = SkillMdDocument::from_path(&skill_md_path, true);
    assert!(result.is_ok());
}

#[test]
fn test_validate_description_too_short() {
    let dir = tempdir().unwrap();
    let skill_name = "test-skill";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let description = "Too short";
    let content = create_valid_skill_content(skill_name, description);
    std::fs::write(&skill_md_path, content).unwrap();

    let result = SkillMdDocument::from_path(&skill_md_path, true);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("at least 50 characters")
    );
}

#[test]
fn test_validate_description_missing_trigger_word() {
    let dir = tempdir().unwrap();
    let skill_name = "test-skill";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let description = "This is a description without any trigger words. It should fail validation because it doesn't contain the required keywords.";
    let content = create_valid_skill_content(skill_name, description);
    std::fs::write(&skill_md_path, content).unwrap();

    let result = SkillMdDocument::from_path(&skill_md_path, true);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("must contain at least one trigger word")
    );
}

#[test]
fn test_validate_version_valid() {
    let valid_versions = vec!["1.0.0", "0.1.0", "2.3.4-alpha", "1.2.3+build.123", "v1.0.0"];

    for version in valid_versions {
        let dir = tempdir().unwrap();
        let skill_name = "test-skill";
        let skill_dir = dir.path().join(skill_name);
        std::fs::create_dir(&skill_dir).unwrap();

        let skill_md_path = skill_dir.join("SKILL.md");
        let description = "A test skill for version validation. Use this to verify version parsing works correctly.";
        let content = format!(
            r#"---
name: {}
description: {}
version: {}
license: MIT
tags: ["test"]
timeout: 300
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
"#,
            skill_name, description, version
        );
        std::fs::write(&skill_md_path, content).unwrap();

        let result = SkillMdDocument::from_path(&skill_md_path, true);
        assert!(result.is_ok(), "Version '{}' should be valid", version);
    }
}

#[test]
fn test_validate_version_invalid() {
    let invalid_versions = vec!["invalid", "1.0", "1", "1.2.3.4", "a.b.c"];

    for version in invalid_versions {
        let dir = tempdir().unwrap();
        let skill_name = "test-skill";
        let skill_dir = dir.path().join(skill_name);
        std::fs::create_dir(&skill_dir).unwrap();

        let skill_md_path = skill_dir.join("SKILL.md");
        let description = "A test skill for version validation. Use this to verify version parsing works correctly.";
        let content = format!(
            r#"---
name: {}
description: {}
version: {}
license: MIT
tags: ["test"]
timeout: 300
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
"#,
            skill_name, description, version
        );
        std::fs::write(&skill_md_path, content).unwrap();

        let result = SkillMdDocument::from_path(&skill_md_path, true);
        assert!(result.is_err(), "Version '{}' should be invalid", version);
    }
}

#[test]
fn test_validate_tags_valid() {
    let valid_tag_sets = vec![
        vec!["test"],
        vec!["test", "example"],
        vec![
            "test", "example", "tag1", "tag2", "tag3", "tag4", "tag5", "tag6", "tag7", "tag8",
        ],
    ];

    for tags in valid_tag_sets {
        let dir = tempdir().unwrap();
        let skill_name = "test-skill";
        let skill_dir = dir.path().join(skill_name);
        std::fs::create_dir(&skill_dir).unwrap();

        let skill_md_path = skill_dir.join("SKILL.md");
        let tags_str: Vec<String> = tags.iter().map(|s| format!("\"{}\"", s)).collect();
        let description =
            "A test skill for tags validation. Use this to verify tag validation works correctly.";
        let content = format!(
            r#"---
name: {}
description: {}
version: 1.0.0
tags: [{}]
timeout: 300
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
"#,
            skill_name,
            description,
            tags_str.join(", ")
        );
        std::fs::write(&skill_md_path, content).unwrap();

        let result = SkillMdDocument::from_path(&skill_md_path, true);
        assert!(result.is_ok(), "Tags {:?} should be valid", tags);
    }
}

#[test]
fn test_validate_tags_too_many() {
    let dir = tempdir().unwrap();
    let skill_name = "test-skill";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let tags: Vec<&str> = (0..11).map(|i| format!("tag{}", i)).collect();
    let tags_str: Vec<String> = tags.iter().map(|s| format!("\"{}\"", s)).collect();
    let description =
        "A test skill for tags validation. Use this to verify tag validation works correctly.";
    let content = format!(
        r#"---
name: {}
description: {}
version: 1.0.0
tags: [{}]
timeout: 300
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
"#,
        skill_name,
        description,
        tags_str.join(", ")
    );
    std::fs::write(&skill_md_path, content).unwrap();

    let result = SkillMdDocument::from_path(&skill_md_path, true);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("must not exceed 10")
    );
}

#[test]
fn test_validate_timeout_valid() {
    let valid_timeouts = vec![1, 60, 300, 3600];

    for timeout in valid_timeouts {
        let dir = tempdir().unwrap();
        let skill_name = "test-skill";
        let skill_dir = dir.path().join(skill_name);
        std::fs::create_dir(&skill_dir).unwrap();

        let skill_md_path = skill_dir.join("SKILL.md");
        let description = "A test skill for timeout validation. Use this to verify timeout validation works correctly.";
        let content = format!(
            r#"---
name: {}
description: {}
version: 1.0.0
timeout: {}
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
"#,
            skill_name, description, timeout
        );
        std::fs::write(&skill_md_path, content).unwrap();

        let result = SkillMdDocument::from_path(&skill_md_path, true);
        assert!(result.is_ok(), "Timeout {} should be valid", timeout);
    }
}

#[test]
fn test_validate_timeout_invalid() {
    let invalid_timeouts = vec![0, 3601];

    for timeout in invalid_timeouts {
        let dir = tempdir().unwrap();
        let skill_name = "test-skill";
        let skill_dir = dir.path().join(skill_name);
        std::fs::create_dir(&skill_dir).unwrap();

        let skill_md_path = skill_dir.join("SKILL.md");
        let description = "A test skill for timeout validation. Use this to verify timeout validation works correctly.";
        let content = format!(
            r#"---
name: {}
description: {}
version: 1.0.0
timeout: {}
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
"#,
            skill_name, description, timeout
        );
        std::fs::write(&skill_md_path, content).unwrap();

        let result = SkillMdDocument::from_path(&skill_md_path, true);
        assert!(result.is_err(), "Timeout {} should be invalid", timeout);
    }
}

#[test]
fn test_validate_markdown_sections_complete() {
    let dir = tempdir().unwrap();
    let skill_name = "test-skill";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let description = "A test skill with complete sections. Use this to verify all required sections are present.";
    let content = create_valid_skill_content(skill_name, description);
    std::fs::write(&skill_md_path, content).unwrap();

    let result = SkillMdDocument::from_path(&skill_md_path, true);
    assert!(result.is_ok());
}

#[test]
fn test_validate_markdown_sections_missing() {
    let dir = tempdir().unwrap();
    let skill_name = "test-skill";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let description = "A test skill with missing sections. Use this to verify validation catches missing sections.";
    let content = format!(
        r#"---
name: {}
description: {}
version: 1.0.0
timeout: 300
---

# Test Skill

## 功能概述
Test content.
"#,
        skill_name, description
    );
    std::fs::write(&skill_md_path, content).unwrap();

    let result = SkillMdDocument::from_path(&skill_md_path, true);
    assert!(result.is_err());
}

#[test]
fn test_validate_execution_flow_ordered_list() {
    let dir = tempdir().unwrap();
    let skill_name = "test-skill";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let description = "A test skill with invalid execution flow. Use this to verify validation.";
    let content = format!(
        r#"---
name: {}
description: {}
version: 1.0.0
timeout: 300
---

# Test Skill

## 功能概述
Test content.

## 适用场景
Testing.

## 输入规范
None.

## 执行步骤
- Step one (not ordered)
- Step two

## 输出规范
Something.

## 约束与安全
None.

## 示例
None.
"#,
        skill_name, description
    );
    std::fs::write(&skill_md_path, content).unwrap();

    let result = SkillMdDocument::from_path(&skill_md_path, true);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("must be an ordered list starting with '1.'")
    );
}
