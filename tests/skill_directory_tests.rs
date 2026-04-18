use aetheris::skill::agentskills::SkillMdDocument;
use tempfile::tempdir;

#[test]
fn test_directory_structure_minimal() {
    let dir = tempdir().unwrap();
    let skill_name = "minimal-skill";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = r#"---
name: minimal-skill
description: A minimal skill for testing directory structure. Use this to verify basic discovery works.
version: 1.0.0
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
    std::fs::write(&skill_md_path, content).unwrap();

    let skill_doc = SkillMdDocument::from_path(&skill_md_path, true).unwrap();

    assert!(skill_doc.directory_structure.is_some());
    let dir_structure = skill_doc.directory_structure.unwrap();

    assert_eq!(dir_structure.skill_dir, skill_dir);
    assert!(dir_structure.scripts_dir.is_none());
    assert!(dir_structure.assets_dir.is_none());
    assert!(dir_structure.references_dir.is_none());
    assert!(dir_structure.sub_skills_dir.is_none());
    assert!(dir_structure.license_file.is_none());
    assert!(dir_structure.readme_file.is_none());
}

#[test]
fn test_directory_structure_full() {
    let dir = tempdir().unwrap();
    let skill_name = "full-skill";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    std::fs::create_dir(skill_dir.join("scripts")).unwrap();
    std::fs::create_dir(skill_dir.join("assets")).unwrap();
    std::fs::create_dir(skill_dir.join("references")).unwrap();
    std::fs::create_dir(skill_dir.join("sub-skills")).unwrap();
    std::fs::write(skill_dir.join("LICENSE"), "MIT License").unwrap();
    std::fs::write(skill_dir.join("README.md"), "# Test Skill").unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = r#"---
name: full-skill
description: A full skill with complete directory structure. Use this to verify all directories are discovered.
version: 1.0.0
---

# Full Skill

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
    let dir_structure = skill_doc.directory_structure.unwrap();

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
fn test_directory_structure_with_scripts() {
    let dir = tempdir().unwrap();
    let skill_name = "skill-with-scripts";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    std::fs::create_dir(skill_dir.join("scripts")).unwrap();
    std::fs::write(
        skill_dir.join("scripts").join("main.py"),
        "#!/usr/bin/env python3\nprint('hello')",
    )
    .unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = r#"---
name: skill-with-scripts
description: A skill with scripts directory. Use this to verify scripts are discovered.
version: 1.0.0
---

# Skill with Scripts

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
    let dir_structure = skill_doc.directory_structure.unwrap();

    assert_eq!(dir_structure.scripts_dir, Some(skill_dir.join("scripts")));
}

#[test]
fn test_directory_structure_with_assets() {
    let dir = tempdir().unwrap();
    let skill_name = "skill-with-assets";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    std::fs::create_dir(skill_dir.join("assets")).unwrap();
    std::fs::write(
        skill_dir.join("assets").join("template.txt"),
        "Template content",
    )
    .unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = r#"---
name: skill-with-assets
description: A skill with assets directory. Use this to verify assets are discovered.
version: 1.0.0
---

# Skill with Assets

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
    let dir_structure = skill_doc.directory_structure.unwrap();

    assert_eq!(dir_structure.assets_dir, Some(skill_dir.join("assets")));
}

#[test]
fn test_directory_structure_with_references() {
    let dir = tempdir().unwrap();
    let skill_name = "skill-with-references";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    std::fs::create_dir(skill_dir.join("references")).unwrap();
    std::fs::write(
        skill_dir.join("references").join("api-spec.yaml"),
        "spec: content",
    )
    .unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = r#"---
name: skill-with-references
description: A skill with references directory. Use this to verify references are discovered.
version: 1.0.0
---

# Skill with References

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
    let dir_structure = skill_doc.directory_structure.unwrap();

    assert_eq!(
        dir_structure.references_dir,
        Some(skill_dir.join("references"))
    );
}

#[test]
fn test_directory_structure_with_sub_skills() {
    let dir = tempdir().unwrap();
    let skill_name = "skill-with-sub-skills";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    let sub_skill_dir = skill_dir.join("sub-skills");
    std::fs::create_dir(&sub_skill_dir).unwrap();
    std::fs::create_dir(sub_skill_dir.join("sub-skill-1")).unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = r#"---
name: skill-with-sub-skills
description: A skill with sub-skills directory. Use this to verify sub-skills are discovered.
version: 1.0.0
---

# Skill with Sub-skills

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
    let dir_structure = skill_doc.directory_structure.unwrap();

    assert_eq!(
        dir_structure.sub_skills_dir,
        Some(skill_dir.join("sub-skills"))
    );
}

#[test]
fn test_directory_structure_with_license() {
    let dir = tempdir().unwrap();
    let skill_name = "skill-with-license";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    std::fs::write(skill_dir.join("LICENSE"), "Apache-2.0 License").unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = r#"---
name: skill-with-license
description: A skill with LICENSE file. Use this to verify LICENSE is discovered.
version: 1.0.0
---

# Skill with License

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
    let dir_structure = skill_doc.directory_structure.unwrap();

    assert_eq!(dir_structure.license_file, Some(skill_dir.join("LICENSE")));
}

#[test]
fn test_directory_structure_with_readme() {
    let dir = tempdir().unwrap();
    let skill_name = "skill-with-readme";
    let skill_dir = dir.path().join(skill_name);
    std::fs::create_dir(&skill_dir).unwrap();

    std::fs::write(
        skill_dir.join("README.md"),
        "# Test Skill\n\nThis is a test skill.",
    )
    .unwrap();

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = r#"---
name: skill-with-readme
description: A skill with README.md file. Use this to verify README is discovered.
version: 1.0.0
---

# Skill with README

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
    let dir_structure = skill_doc.directory_structure.unwrap();

    assert_eq!(dir_structure.readme_file, Some(skill_dir.join("README.md")));
}
