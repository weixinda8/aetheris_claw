use aetheris::skill::agentskills::{SkillMdDocument, validate_allowed_tools, validate_name};
use std::time::Instant;
use tempfile::tempdir;

#[test]
fn test_skill_md_parse_valid() {
    let content = r#"---
name: expense-report
description: File and validate employee expense reports according to company policy. Use when asked about expense submissions, reimbursement rules, or spending limits.
version: 1.0.0
license: Apache-2.0
tags: ["finance", "expense", "report"]
timeout: 300
allowed-tools: ["Read", "Write", "Bash"]
metadata:
  emoji: "📊"
---

# 费用报销技能

## 功能概述
一句话清晰说明本技能做什么、适用场景。

## 适用场景
员工提交费用报销、财务审核报销单等场景。

## 输入规范
- `employee_id` (string): 员工 ID
- `expense_date` (string): 报销日期
- `amount` (number): 报销金额

## 执行步骤
1. 第一步：验证员工身份
2. 第二步：检查报销金额是否符合公司政策
3. 第三步：生成报销记录并保存

## 输出规范
返回报销确认信息，包含报销ID和状态。

## 约束与安全
- 只能处理公司内部员工的报销
- 报销金额上限为10000元
- 所有操作需要记录审计日志

## 示例
### 示例1：正常报销
员工ID: E12345，金额: 500元，日期: 2026-04-07
"#;

    let start = Instant::now();
    let skill_md = SkillMdDocument::parse(content).unwrap();
    let duration = start.elapsed();

    assert_eq!(skill_md.frontmatter.name, "expense-report");
    assert_eq!(skill_md.frontmatter.version, Some("1.0.0".to_string()));
    assert_eq!(skill_md.frontmatter.license, Some("Apache-2.0".to_string()));
    assert_eq!(skill_md.frontmatter.timeout, Some(300));
    assert_eq!(
        skill_md.frontmatter.tags,
        Some(vec![
            "finance".to_string(),
            "expense".to_string(),
            "report".to_string()
        ])
    );
    assert!(skill_md.frontmatter.allowed_tools.is_some());
    assert!(!skill_md.sections.overview.is_empty());
    assert!(!skill_md.sections.execution_flow.is_empty());
    assert!(!skill_md.sections.examples.is_empty());

    println!("SKILL.md parsed in: {:?}", duration);
    assert!(
        duration.as_millis() < 50,
        "Parser performance exceeded 50ms limit"
    );
}

#[test]
fn test_skill_md_parse_missing_frontmatter() {
    let content = r#"# 费用报销技能

## 功能概述
内容...
"#;

    let result = SkillMdDocument::parse(content);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("must start with YAML Frontmatter delimiter")
    );
}

#[test]
fn test_skill_md_parse_missing_closing_frontmatter() {
    let content = r#"---
name: test-skill
description: Test skill description with enough characters to pass validation. Use this for testing.

# 费用报销技能
"#;

    let result = SkillMdDocument::parse(content);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Missing closing YAML Frontmatter delimiter")
    );
}

#[test]
fn test_skill_md_parse_invalid_yaml() {
    let content = r#"---
name: test-skill
description: Test skill
invalid: [unclosed
---

# 测试技能
"#;

    let result = SkillMdDocument::parse(content);
    assert!(result.is_err());
}

#[test]
fn test_skill_md_parse_minimal_valid() {
    let content = r#"---
name: minimal-skill
description: A minimal skill for testing purposes. Use this to verify basic parsing works correctly.
---

# Minimal Skill

## 功能概述
Minimal content.

## 适用场景
Testing minimal skills.

## 输入规范
No input.

## 执行步骤
1. Do something.

## 输出规范
Some output.

## 约束与安全
No constraints.

## 示例
No examples.
"#;

    let skill_md = SkillMdDocument::parse(content).unwrap();
    assert_eq!(skill_md.frontmatter.name, "minimal-skill");
}

#[test]
fn test_skill_md_from_path() {
    let dir = tempdir().unwrap();
    let skill_dir = dir.path().join("test-skill");
    std::fs::create_dir(&skill_dir).unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = r#"---
name: test-skill
description: A test skill for validating the from_path method. Use this when you need to test path-based skill loading.
version: 1.0.0
tags: ["test", "validation"]
timeout: 60
---

# Test Skill

## 功能概述
This is a test skill for path loading.

## 适用场景
Testing skill loading from file system paths.

## 输入规范
No specific input required for this test.

## 执行步骤
1. Load the skill from the given path
2. Parse the YAML frontmatter
3. Parse the Markdown sections
4. Validate all required fields

## 输出规范
Returns a valid SkillMdDocument instance.

## 约束与安全
This is a test skill with no real constraints.

## 示例
### 示例1：基本加载
Load skill from path and verify it works correctly.
"#;
    std::fs::write(&skill_md_path, content).unwrap();

    let skill_md = SkillMdDocument::from_path(&skill_md_path, true).unwrap();
    assert_eq!(skill_md.frontmatter.name, "test-skill");
    assert!(skill_md.directory_structure.is_some());
}

#[test]
fn test_skill_md_from_path_nonexistent() {
    let dir = tempdir().unwrap();
    let skill_md_path = dir.path().join("nonexistent").join("SKILL.md");

    let result = SkillMdDocument::from_path(&skill_md_path, false);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("SKILL.md not found")
    );
}

#[test]
fn test_validate_name_valid() {
    assert!(validate_name("valid-skill-name").is_ok());
    assert!(validate_name("a").is_ok());
    assert!(validate_name("skill123").is_ok());
    assert!(validate_name("my-awesome-skill").is_ok());
}

#[test]
fn test_validate_name_invalid() {
    assert!(validate_name("").is_err());
    assert!(validate_name("InvalidName").is_err());
    assert!(validate_name("-invalid").is_err());
    assert!(validate_name("invalid-").is_err());
    assert!(validate_name("invalid--name").is_err());
    assert!(validate_name("invalid name").is_err());
    assert!(validate_name(&"a".repeat(65)).is_err());
}

#[test]
fn test_validate_allowed_tools_valid() {
    let tools = vec!["Read".to_string(), "Write".to_string(), "LLM".to_string()];
    assert!(validate_allowed_tools(&tools).is_ok());
}

#[test]
fn test_validate_allowed_tools_invalid() {
    let tools = vec!["Read".to_string(), "InvalidTool".to_string()];
    let result = validate_allowed_tools(&tools);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("InvalidTool"));
}
