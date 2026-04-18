use aetheris::skill::agentskills::{SkillMdDocument, validate_name};
use tempfile::tempdir;

#[test]
fn test_validate_name_kebab_case_valid() {
    let valid_names = vec![
        "my-skill",
        "skill-123",
        "a",
        "very-long-skill-name-that-is-still-valid",
        "skill-with-many-hyphens",
        "123-skill",
        "skill-123-abc",
    ];

    for name in valid_names {
        assert!(validate_name(name).is_ok(), "{} should be valid", name);
    }
}

#[test]
fn test_validate_name_invalid_cases() {
    let invalid_names = vec![
        "",
        "MySkill",
        "my_skill",
        "my skill",
        "-invalid",
        "invalid-",
        "invalid--name",
        "invalid---name",
        "invalid/name",
        "invalid.name",
        "invalid,name",
        "invalid;name",
        "invalid:name",
        "invalid@name",
        "invalid#name",
        "invalid$name",
        "invalid%name",
        "invalid^name",
        "invalid&name",
        "invalid*name",
        "invalid(name",
        "invalid)name",
        "invalid=name",
        "invalid+name",
        "invalid?name",
        "invalid>name",
        "invalid<name",
        "invalid[",
        "invalid]",
        "invalid{",
        "invalid}",
        "invalid|",
        "invalid\\name",
        "invalid`name",
        "invalid~name",
        "invalid!name",
        &"a".repeat(65),
    ];

    for name in invalid_names {
        assert!(validate_name(name).is_err(), "{} should be invalid", name);
    }
}

#[test]
fn test_validate_name_length_bounds() {
    assert!(validate_name(&"a".repeat(1)).is_ok());
    assert!(validate_name(&"a".repeat(64)).is_ok());

    assert!(validate_name(&"a".repeat(0)).is_err());
    assert!(validate_name(&"a".repeat(65)).is_err());
}

#[test]
fn test_name_matches_directory_valid() {
    let dir = tempdir().unwrap();
    let skill_name = "valid-skill-name";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = format!(
        r#"---
name: {}
description: A valid skill with matching directory name. Use this for testing.
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
        skill_name
    );
    std::fs::write(&skill_md_path, content).unwrap();

    let result = SkillMdDocument::from_path(&skill_md_path, true);
    assert!(result.is_ok());
}

#[test]
fn test_name_matches_directory_invalid() {
    let dir = tempdir().unwrap();
    let dir_name = "directory-name";
    let skill_dir = dir.path().join(dir_name);
    std::fs::create_dir(&skill_dir).unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = r#"---
name: different-name
description: A skill with mismatched directory name. Use this for testing.
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
    std::fs::write(&skill_md_path, content).unwrap();

    let result = SkillMdDocument::from_path(&skill_md_path, true);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("must match directory name")
    );
}
