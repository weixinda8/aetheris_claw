use crate::utils::{AetherisError, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

const OFFICIAL_ALLOWED_TOOLS: &[&str] = &[
    "Read",
    "Write",
    "Delete",
    "Bash",
    "CodeExecutor",
    "WebSearch",
    "Browser",
    "Database",
    "CSV",
    "JSON",
    "Excel",
    "LLM",
    "Embedding",
    "RAG",
    "Vision",
    "Audio",
    "Git",
    "Docker",
    "API",
    "CI/CD",
];

const TRIGGER_WORDS: &[&str] = &[
    "use", "when", "for", "handle", "process", "manage", "analyze", "validate", "generate",
    "execute", "perform", "create", "update", "delete", "read", "write", "submit", "approve",
    "reject", "review", "audit", "使用", "当", "用于", "处理", "管理", "分析", "验证", "生成",
    "执行", "创建", "更新", "删除", "提交", "批准", "拒绝", "审查", "审核",
];

lazy_static::lazy_static! {
    static ref SEMVER_REGEX: Regex = Regex::new(
        r"^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$"
    ).unwrap();

    static ref NAME_REGEX: Regex = Regex::new(
        r"^[a-z0-9][a-z0-9-]*[a-z0-9]$"
    ).unwrap();
}

pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(AetherisError::AgentSkills(
            "name cannot be empty".to_string(),
        ));
    }

    if name.is_empty() || name.len() > 64 {
        return Err(AetherisError::AgentSkills(format!(
            "name must be between 1 and 64 characters, got {}",
            name.len()
        )));
    }

    if !NAME_REGEX.is_match(name) {
        return Err(AetherisError::AgentSkills(
            "name must be kebab-case: lowercase letters, numbers, and hyphens only, cannot start or end with hyphen, no consecutive hyphens".to_string()
        ));
    }

    Ok(())
}

pub fn validate_allowed_tools(tools: &[String]) -> Result<()> {
    let official_set: HashSet<_> = OFFICIAL_ALLOWED_TOOLS.iter().collect();

    for tool in tools {
        if !official_set.contains(&tool.as_str()) {
            return Err(AetherisError::AgentSkills(format!(
                "tool '{}' is not in the official allowed-tools list",
                tool
            )));
        }
    }

    Ok(())
}

impl SkillMdDocument {
    pub fn validate_official_rules(&self, skill_dir: impl AsRef<Path>) -> Result<()> {
        let skill_dir = skill_dir.as_ref();

        Self::validate_name_matches_directory(&self.frontmatter.name, skill_dir)?;
        Self::validate_description(&self.frontmatter.description)?;
        Self::validate_version(&self.frontmatter.version)?;
        Self::validate_allowed_tools(&self.frontmatter.allowed_tools)?;
        Self::validate_tags(&self.frontmatter.tags)?;
        Self::validate_timeout(&self.frontmatter.timeout)?;
        Self::validate_markdown_sections(&self.sections)?;

        Ok(())
    }

    fn validate_name_matches_directory(name: &str, skill_dir: &Path) -> Result<()> {
        let dir_name = skill_dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| AetherisError::AgentSkills("Invalid directory name".to_string()))?;

        if name != dir_name {
            return Err(AetherisError::AgentSkills(format!(
                "name field '{}' must match directory name '{}'",
                name, dir_name
            )));
        }

        Ok(())
    }

    fn validate_description(description: &str) -> Result<()> {
        if description.len() < 50 {
            return Err(AetherisError::AgentSkills(format!(
                "description must be at least 50 characters, got {}",
                description.len()
            )));
        }

        let has_trigger_word = TRIGGER_WORDS
            .iter()
            .any(|&word| description.to_lowercase().contains(&word.to_lowercase()));

        if !has_trigger_word {
            return Err(AetherisError::AgentSkills(
                "description must contain at least one trigger word (e.g., 'use', 'when', 'for', etc.)".to_string()
            ));
        }

        Ok(())
    }

    fn validate_version(version: &Option<String>) -> Result<()> {
        if let Some(version) = version {
            if !SEMVER_REGEX.is_match(version) {
                return Err(AetherisError::AgentSkills(format!(
                    "version '{}' is not a valid semantic version",
                    version
                )));
            }
        }
        Ok(())
    }

    fn validate_allowed_tools(allowed_tools: &Option<Vec<String>>) -> Result<()> {
        if let Some(tools) = allowed_tools {
            let official_set: HashSet<_> = OFFICIAL_ALLOWED_TOOLS.iter().collect();

            for tool in tools {
                if !official_set.contains(&tool.as_str()) {
                    return Err(AetherisError::AgentSkills(format!(
                        "tool '{}' is not in the official allowed-tools list",
                        tool
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_tags(tags: &Option<Vec<String>>) -> Result<()> {
        if let Some(tags) = tags {
            if tags.len() > 10 {
                return Err(AetherisError::AgentSkills(format!(
                    "tags must not exceed 10, got {}",
                    tags.len()
                )));
            }

            for tag in tags {
                if tag != &tag.to_lowercase() {
                    return Err(AetherisError::AgentSkills(format!(
                        "tag '{}' must be lowercase",
                        tag
                    )));
                }

                if tag.len() > 20 {
                    return Err(AetherisError::AgentSkills(format!(
                        "tag '{}' is too long (max 20 characters)",
                        tag
                    )));
                }

                if tag.contains(|c: char| !c.is_alphanumeric() && c != '-' && c != '_') {
                    return Err(AetherisError::AgentSkills(format!(
                        "tag '{}' contains invalid characters",
                        tag
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_timeout(timeout: &Option<u64>) -> Result<()> {
        if let Some(timeout) = timeout {
            if *timeout < 1 || *timeout > 3600 {
                return Err(AetherisError::AgentSkills(format!(
                    "timeout must be between 1 and 3600 seconds, got {}",
                    timeout
                )));
            }
        }
        Ok(())
    }

    fn validate_markdown_sections(sections: &SkillMdSections) -> Result<()> {
        let required_sections = [
            ("功能概述", &sections.overview),
            ("适用场景", &sections.use_cases),
            ("输入规范", &sections.input_spec),
            ("执行流程", &sections.execution_flow),
            ("输出规范", &sections.output_spec),
            ("约束与安全", &sections.constraints),
            ("示例", &sections.examples),
        ];

        for (name, content) in required_sections.iter() {
            if content.is_empty() {
                return Err(AetherisError::AgentSkills(format!(
                    "required section '{}' is missing or empty",
                    name
                )));
            }
        }

        if !sections.execution_flow.trim().starts_with("1.") {
            return Err(AetherisError::AgentSkills(
                "execution flow must be an ordered list starting with '1.'".to_string(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentSkillType {
    Builtin,
    External,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkillParameter {
    pub name: String,
    pub description: String,
    pub r#type: String,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub enum_values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkillReturn {
    pub r#type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkillExample {
    pub name: String,
    pub description: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkillMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub long_description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub skill_type: AgentSkillType,
    pub priority: Option<u8>,
    pub icon: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub issues: Option<String>,
    pub keywords: Vec<String>,
    pub deprecated: bool,
    pub deprecation_message: Option<String>,
    pub retry_config: Option<AgentSkillRetryConfig>,
    pub sandbox_level: Option<String>,
    pub implementation: Option<AgentSkillImplementation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkillManifest {
    pub metadata: AgentSkillMetadata,
    pub parameters: Vec<AgentSkillParameter>,
    pub returns: Option<AgentSkillReturn>,
    pub examples: Vec<AgentSkillExample>,
    pub dependencies: Vec<String>,
    pub env_vars: Vec<String>,
    pub permissions: Vec<String>,
    pub timeout_seconds: Option<u64>,
    pub retry_config: Option<AgentSkillRetryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkillRetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub retry_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkillImplementation {
    pub r#type: String,
    pub name: String,
}

impl Default for AgentSkillRetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 10000,
            backoff_multiplier: 2.0,
            retry_on: vec!["timeout".to_string(), "network_error".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMdFrontmatter {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub tags: Option<Vec<String>>,
    pub compatibility: Option<String>,
    pub timeout: Option<u64>,
    pub requires: Option<Vec<String>>,
    pub platforms: Option<Vec<String>>,
    #[serde(rename = "allowed-tools")]
    pub allowed_tools: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub retry_config: Option<AgentSkillRetryConfig>,
    pub sandbox_level: Option<String>,
    pub implementation: Option<AgentSkillImplementation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMdSections {
    pub overview: String,
    pub use_cases: String,
    pub input_spec: String,
    pub execution_flow: String,
    pub output_spec: String,
    pub constraints: String,
    pub examples: String,
    pub additional_sections: HashMap<String, String>,
}

/// 表示技能目录的完整结构，包括可选的子目录和子技能。
///
/// 该结构体用于发现和组织技能目录中的所有组件，包括脚本、资源、参考文档，
/// 以及最重要的——子技能。子技能通过递归扫描 `sub-skills/` 目录来发现。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDirectoryStructure {
    /// 技能的根目录路径。
    pub skill_dir: PathBuf,
    /// 脚本目录（可选），存放技能的执行脚本。
    pub scripts_dir: Option<PathBuf>,
    /// 资源目录（可选），存放技能使用的静态资源。
    pub assets_dir: Option<PathBuf>,
    /// 参考文档目录（可选），存放技能相关的参考资料。
    pub references_dir: Option<PathBuf>,
    /// 子技能目录（可选），存放该技能的子技能。
    pub sub_skills_dir: Option<PathBuf>,
    /// 子技能列表（可选），递归发现的所有子技能的目录结构。
    ///
    /// 只有当 `sub_skills_dir` 存在且包含有效的子技能目录时，
    /// 此字段才会被填充。每个子技能都有自己完整的 `SkillDirectoryStructure`。
    pub sub_skills: Option<Vec<SkillDirectoryStructure>>,
    /// LICENSE 文件路径（可选）。
    pub license_file: Option<PathBuf>,
    /// README.md 文件路径（可选）。
    pub readme_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMdDocument {
    pub frontmatter: SkillMdFrontmatter,
    pub sections: SkillMdSections,
    pub raw_markdown: String,
    pub directory_structure: Option<SkillDirectoryStructure>,
}

impl SkillDirectoryStructure {
    /// 发现指定目录的技能目录结构。
    ///
    /// 该方法会递归扫描目录，查找所有标准的技能组件，包括：
    /// - scripts/ 目录
    /// - assets/ 目录
    /// - references/ 目录
    /// - sub-skills/ 目录（递归发现子技能）
    /// - LICENSE 文件
    /// - README.md 文件
    ///
    /// # Arguments
    /// * `skill_dir` - 要发现的技能根目录
    ///
    /// # Returns
    /// 返回完整的 `SkillDirectoryStructure`，包含所有发现的组件
    ///
    /// # Errors
    /// - 当无法规范化路径时返回错误
    /// - 当检测到循环依赖时返回错误
    pub fn discover(skill_dir: impl AsRef<Path>) -> Result<Self> {
        let mut visited = HashSet::new();
        Self::discover_with_cycle_detection(skill_dir, &mut visited)
    }

    /// 内部方法：使用循环依赖检测来发现技能目录结构。
    ///
    /// # Arguments
    /// * `skill_dir` - 要发现的技能根目录
    /// * `visited` - 已访问目录的集合，用于检测循环依赖
    fn discover_with_cycle_detection(
        skill_dir: impl AsRef<Path>,
        visited: &mut HashSet<PathBuf>,
    ) -> Result<Self> {
        let skill_dir = skill_dir.as_ref().to_path_buf();
        let canonical_skill_dir = skill_dir.canonicalize().map_err(|e| {
            AetherisError::AgentSkills(format!("Failed to canonicalize path: {}", e))
        })?;

        if visited.contains(&canonical_skill_dir) {
            return Err(AetherisError::AgentSkills(format!(
                "Circular dependency detected in skill directory: {:?}",
                canonical_skill_dir
            )));
        }

        visited.insert(canonical_skill_dir.clone());
        debug!("Discovering skill directory structure: {:?}", skill_dir);

        let scripts_dir = skill_dir.join("scripts");
        let scripts_dir = if scripts_dir.is_dir() {
            Some(scripts_dir)
        } else {
            None
        };

        let assets_dir = skill_dir.join("assets");
        let assets_dir = if assets_dir.is_dir() {
            Some(assets_dir)
        } else {
            None
        };

        let references_dir = skill_dir.join("references");
        let references_dir = if references_dir.is_dir() {
            Some(references_dir)
        } else {
            None
        };

        let sub_skills_dir = skill_dir.join("sub-skills");
        let sub_skills_dir = if sub_skills_dir.is_dir() {
            Some(sub_skills_dir)
        } else {
            None
        };

        let sub_skills = if let Some(ref sub_dir) = sub_skills_dir {
            let mut sub_skills_list = Vec::new();
            if let Ok(entries) = std::fs::read_dir(sub_dir) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        let skill_md_path = entry_path.join("SKILL.md");
                        if skill_md_path.exists() {
                            info!("Found sub-skill directory: {:?}", entry_path);
                            let sub_skill_struct =
                                Self::discover_with_cycle_detection(entry_path, visited)?;
                            sub_skills_list.push(sub_skill_struct);
                        }
                    }
                }
            }
            if sub_skills_list.is_empty() {
                None
            } else {
                Some(sub_skills_list)
            }
        } else {
            None
        };

        let license_file = skill_dir.join("LICENSE");
        let license_file = if license_file.is_file() {
            Some(license_file)
        } else {
            None
        };

        let readme_file = skill_dir.join("README.md");
        let readme_file = if readme_file.is_file() {
            Some(readme_file)
        } else {
            None
        };

        visited.remove(&canonical_skill_dir);

        Ok(Self {
            skill_dir,
            scripts_dir,
            assets_dir,
            references_dir,
            sub_skills_dir,
            sub_skills,
            license_file,
            readme_file,
        })
    }

    /// 检查该技能是否有子技能。
    ///
    /// # Returns
    /// 如果有子技能返回 `true`，否则返回 `false`
    pub fn has_sub_skills(&self) -> bool {
        self.sub_skills.as_ref().is_some_and(|s| !s.is_empty())
    }

    /// 获取该技能的直接子技能数量。
    ///
    /// 注意：此方法只计算直接子技能，不包括嵌套更深的子技能。
    ///
    /// # Returns
    /// 直接子技能的数量
    pub fn sub_skills_count(&self) -> usize {
        self.sub_skills.as_ref().map_or(0, |s| s.len())
    }

    /// 获取所有子技能（包括嵌套的子技能）。
    ///
    /// 此方法会递归遍历整个子技能树，返回所有层级的子技能引用。
    ///
    /// # Returns
    /// 包含所有子技能的向量，按深度优先顺序排列
    pub fn all_sub_skills(&self) -> Vec<&SkillDirectoryStructure> {
        let mut result = Vec::new();
        if let Some(ref sub_skills) = self.sub_skills {
            for sub_skill in sub_skills {
                result.push(sub_skill);
                result.extend(sub_skill.all_sub_skills());
            }
        }
        result
    }
}

impl SkillMdDocument {
    pub fn parse(content: &str) -> Result<Self> {
        let (frontmatter_str, markdown_str) = Self::split_frontmatter(content)?;
        let frontmatter: SkillMdFrontmatter =
            serde_yaml::from_str(frontmatter_str).map_err(|e| {
                AetherisError::AgentSkills(format!("YAML Frontmatter parse error: {}", e))
            })?;

        validate_name(&frontmatter.name)?;

        if let Some(allowed_tools) = &frontmatter.allowed_tools {
            validate_allowed_tools(allowed_tools)?;
        }

        let sections = Self::parse_markdown_sections(markdown_str)?;

        Ok(Self {
            frontmatter,
            sections,
            raw_markdown: content.to_string(),
            directory_structure: None,
        })
    }

    pub fn from_path(path: impl AsRef<Path>, validate: bool) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(AetherisError::NotFound(format!(
                "SKILL.md not found: {:?}",
                path
            )));
        }
        let content = std::fs::read_to_string(path)?;
        let mut skill_doc = Self::parse(&content)?;

        let skill_dir = path.parent().ok_or_else(|| {
            AetherisError::AgentSkills("Cannot determine skill directory".to_string())
        })?;

        skill_doc.directory_structure = Some(SkillDirectoryStructure::discover(skill_dir)?);

        if validate {
            skill_doc.validate_official_rules(skill_dir)?;
        }

        Ok(skill_doc)
    }

    fn split_frontmatter(content: &str) -> Result<(&str, &str)> {
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            return Err(AetherisError::AgentSkills(
                "SKILL.md must start with YAML Frontmatter delimiter '---'".to_string(),
            ));
        }

        let after_first_delim = &trimmed[3..];
        if let Some(end_idx) = after_first_delim.find("---") {
            let frontmatter = after_first_delim[..end_idx].trim();
            let markdown = after_first_delim[end_idx + 3..].trim_start();
            Ok((frontmatter, markdown))
        } else {
            Err(AetherisError::AgentSkills(
                "Missing closing YAML Frontmatter delimiter '---'".to_string(),
            ))
        }
    }

    fn parse_markdown_sections(markdown: &str) -> Result<SkillMdSections> {
        let mut sections = SkillMdSections {
            overview: String::new(),
            use_cases: String::new(),
            input_spec: String::new(),
            execution_flow: String::new(),
            output_spec: String::new(),
            constraints: String::new(),
            examples: String::new(),
            additional_sections: HashMap::new(),
        };

        let lines: Vec<&str> = markdown.lines().collect();
        let mut current_section: Option<String> = None;
        let mut current_content = String::new();

        for line in lines {
            if line.starts_with('#') {
                if let Some(section_name) = current_section.take() {
                    sections.assign_section(&section_name, current_content.trim().to_string());
                }
                current_content = String::new();
                let header_level = line.chars().take_while(|&c| c == '#').count();
                let header_text = line[header_level..].trim();
                current_section = Some(header_text.to_lowercase());
            } else if current_section.is_some() {
                if !current_content.is_empty() {
                    current_content.push('\n');
                }
                current_content.push_str(line);
            }
        }

        if let Some(section_name) = current_section {
            sections.assign_section(&section_name, current_content.trim().to_string());
        }

        Ok(sections)
    }
}

impl SkillMdSections {
    fn assign_section(&mut self, name: &str, content: String) {
        match name {
            "功能概述" | "overview" | "功能说明" => self.overview = content,
            "适用场景" | "use cases" | "use-cases" => self.use_cases = content,
            "输入规范" | "input spec" | "input-spec" | "参数" => self.input_spec = content,
            "执行流程" | "execution flow" | "execution-flow" | "执行步骤" => {
                self.execution_flow = content
            }
            "输出规范" | "output spec" | "output-spec" => self.output_spec = content,
            "约束与安全" | "constraints" | "constraints and safety" => {
                self.constraints = content
            }
            "示例" | "examples" => self.examples = content,
            _ => {
                self.additional_sections.insert(name.to_string(), content);
            }
        }
    }
}

impl From<SkillMdDocument> for AgentSkillManifest {
    fn from(skill_md: SkillMdDocument) -> Self {
        let frontmatter = &skill_md.frontmatter;
        let sections = &skill_md.sections;

        let metadata = AgentSkillMetadata {
            id: frontmatter.name.clone(),
            name: frontmatter.name.clone(),
            version: frontmatter
                .version
                .clone()
                .unwrap_or_else(|| "0.1.0".to_string()),
            description: frontmatter.description.clone(),
            long_description: Some(sections.overview.clone()),
            author: frontmatter.author.clone(),
            license: frontmatter.license.clone(),
            tags: frontmatter.tags.clone().unwrap_or_default(),
            categories: frontmatter.tags.clone().unwrap_or_default(),
            skill_type: AgentSkillType::Custom,
            priority: None,
            icon: None,
            homepage: None,
            repository: None,
            issues: None,
            keywords: frontmatter.tags.clone().unwrap_or_default(),
            deprecated: false,
            deprecation_message: None,
            retry_config: frontmatter.retry_config.clone(),
            sandbox_level: frontmatter.sandbox_level.clone(),
            implementation: frontmatter.implementation.clone(),
        };

        let timeout_seconds = frontmatter.timeout;
        let retry_config = frontmatter
            .retry_config
            .clone()
            .or_else(|| Some(AgentSkillRetryConfig::default()));

        AgentSkillManifest {
            metadata,
            parameters: vec![],
            returns: None,
            examples: vec![],
            dependencies: frontmatter.requires.clone().unwrap_or_default(),
            env_vars: vec![],
            permissions: frontmatter.allowed_tools.clone().unwrap_or_default(),
            timeout_seconds,
            retry_config,
        }
    }
}

impl AgentSkillManifest {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(AetherisError::NotFound(format!(
                "Skill manifest not found: {:?}",
                path
            )));
        }

        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.eq_ignore_ascii_case("SKILL.md") {
                let skill_md = SkillMdDocument::from_path(path, true)?;
                return Ok(AgentSkillManifest::from(skill_md));
            }
        }

        let content = std::fs::read_to_string(path)?;
        content.parse()
    }

    fn validate(manifest: &AgentSkillManifest) -> Result<()> {
        if manifest.metadata.id.is_empty() {
            return Err(AetherisError::AgentSkills(
                "Skill ID cannot be empty".to_string(),
            ));
        }

        if manifest.metadata.name.is_empty() {
            return Err(AetherisError::AgentSkills(
                "Skill name cannot be empty".to_string(),
            ));
        }

        if manifest.metadata.version.is_empty() {
            return Err(AetherisError::AgentSkills(
                "Skill version cannot be empty".to_string(),
            ));
        }

        if manifest.metadata.description.is_empty() {
            return Err(AetherisError::AgentSkills(
                "Skill description cannot be empty".to_string(),
            ));
        }

        validate_allowed_tools(&manifest.permissions)?;

        let mut param_names = std::collections::HashSet::new();
        for param in &manifest.parameters {
            if param.name.is_empty() {
                return Err(AetherisError::AgentSkills(
                    "Parameter name cannot be empty".to_string(),
                ));
            }
            if !param_names.insert(param.name.clone()) {
                return Err(AetherisError::AgentSkills(format!(
                    "Duplicate parameter name: {}",
                    param.name
                )));
            }
        }

        Ok(())
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| AetherisError::AgentSkills(format!("JSON serialization error: {}", e)))
    }

    pub fn to_yaml(&self) -> Result<String> {
        serde_yaml::to_string(self)
            .map_err(|e| AetherisError::AgentSkills(format!("YAML serialization error: {}", e)))
    }

    pub fn save(&self, path: PathBuf) -> Result<()> {
        let content = self.to_yaml()?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn has_parameter(&self, name: &str) -> bool {
        self.parameters.iter().any(|p| p.name == name)
    }

    pub fn get_parameter(&self, name: &str) -> Option<&AgentSkillParameter> {
        self.parameters.iter().find(|p| p.name == name)
    }

    pub fn required_parameters(&self) -> Vec<&AgentSkillParameter> {
        self.parameters.iter().filter(|p| p.required).collect()
    }

    pub fn optional_parameters(&self) -> Vec<&AgentSkillParameter> {
        self.parameters.iter().filter(|p| !p.required).collect()
    }
}

impl std::str::FromStr for AgentSkillManifest {
    type Err = crate::utils::AetherisError;

    fn from_str(content: &str) -> std::result::Result<Self, Self::Err> {
        let manifest: AgentSkillManifest = if content.trim().starts_with('{') {
            serde_json::from_str(content)
                .map_err(|e| AetherisError::AgentSkills(format!("JSON parse error: {}", e)))?
        } else {
            serde_yaml::from_str(content)
                .map_err(|e| AetherisError::AgentSkills(format!("YAML parse error: {}", e)))?
        };

        Self::validate(&manifest)?;

        Ok(manifest)
    }
}

#[derive(Debug, Clone)]
pub struct AgentSkillsRegistry {
    skills_dir: PathBuf,
    skills: HashMap<String, AgentSkillManifest>,
    progressive_disclosure_manager:
        Option<std::sync::Arc<crate::skill::ProgressiveDisclosureManager>>,
    skill_state_manager: Option<std::sync::Arc<crate::skill::SkillStateManager>>,
}

impl AgentSkillsRegistry {
    pub fn new(skills_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&skills_dir)?;

        let mut registry = Self {
            skills_dir,
            skills: HashMap::new(),
            progressive_disclosure_manager: None,
            skill_state_manager: None,
        };

        registry.load_all()?;

        Ok(registry)
    }

    pub fn with_progressive_disclosure(
        mut self,
        manager: std::sync::Arc<crate::skill::ProgressiveDisclosureManager>,
    ) -> Self {
        self.progressive_disclosure_manager = Some(manager);
        self
    }

    pub fn with_skill_state_manager(
        mut self,
        manager: std::sync::Arc<crate::skill::SkillStateManager>,
    ) -> Self {
        self.skill_state_manager = Some(manager);
        self
    }

    pub fn skill_state_manager(&self) -> Option<&std::sync::Arc<crate::skill::SkillStateManager>> {
        self.skill_state_manager.as_ref()
    }

    pub fn load_all(&mut self) -> Result<()> {
        self.skills.clear();

        if let Ok(entries) = std::fs::read_dir(&self.skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let skill_md_path = path.join("SKILL.md");
                    if skill_md_path.exists() {
                        if let Ok(manifest) = AgentSkillManifest::from_path(skill_md_path) {
                            self.skills.insert(manifest.metadata.id.clone(), manifest);
                        }
                    }
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext == "yaml" || ext == "yml" || ext == "json" {
                        if let Ok(manifest) = AgentSkillManifest::from_path(path.clone()) {
                            self.skills.insert(manifest.metadata.id.clone(), manifest);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&AgentSkillManifest> {
        self.skills.get(id)
    }

    pub fn list(&self) -> Vec<&AgentSkillManifest> {
        self.skills.values().collect()
    }

    pub fn list_by_category(&self, category: &str) -> Vec<&AgentSkillManifest> {
        self.skills
            .values()
            .filter(|s| s.metadata.categories.iter().any(|c| c == category))
            .collect()
    }

    pub fn list_by_tag(&self, tag: &str) -> Vec<&AgentSkillManifest> {
        self.skills
            .values()
            .filter(|s| s.metadata.tags.iter().any(|t| t == tag))
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<&AgentSkillManifest> {
        let query_lower = query.to_lowercase();
        self.skills
            .values()
            .filter(|s| {
                s.metadata.name.to_lowercase().contains(&query_lower)
                    || s.metadata.description.to_lowercase().contains(&query_lower)
                    || s.metadata
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
                    || s.metadata
                        .keywords
                        .iter()
                        .any(|k| k.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    pub fn add(&mut self, manifest: AgentSkillManifest) -> Result<()> {
        if self.skills.contains_key(&manifest.metadata.id) {
            return Err(AetherisError::AgentSkills(format!(
                "Skill with ID '{}' already exists",
                manifest.metadata.id
            )));
        }

        let path = self
            .skills_dir
            .join(format!("{}.yaml", manifest.metadata.id));
        manifest.save(path)?;

        self.skills.insert(manifest.metadata.id.clone(), manifest);

        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Result<()> {
        if !self.skills.contains_key(id) {
            return Err(AetherisError::NotFound(format!(
                "Skill with ID '{}' not found",
                id
            )));
        }

        let path = self.skills_dir.join(format!("{}.yaml", id));
        if path.exists() {
            std::fs::remove_file(path)?;
        }

        self.skills.remove(id);

        Ok(())
    }

    pub fn skills_dir(&self) -> &PathBuf {
        &self.skills_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use tempfile::tempdir;

    #[test]
    fn test_agent_skill_manifest_parse() {
        let content = r#"
metadata:
  id: test-skill
  name: Test Skill
  version: 1.0.0
  description: A test skill
  tags:
    - test
    - example
  categories:
    - utility
  skill_type: Custom
  keywords:
    - test
    - example
  deprecated: false
parameters:
  - name: input
    description: Input parameter
    type: string
    required: true
returns:
  type: string
  description: Output result
examples:
  - name: Example 1
    description: Test example
    input: "test"
    output: "result"
dependencies: []
env_vars: []
permissions: []
"#;

        let manifest: AgentSkillManifest = content.parse().unwrap();

        assert_eq!(manifest.metadata.id, "test-skill");
        assert_eq!(manifest.metadata.name, "Test Skill");
        assert_eq!(manifest.metadata.version, "1.0.0");
        assert_eq!(manifest.parameters.len(), 1);
    }

    #[test]
    fn test_agent_skills_registry() {
        let dir = tempdir().unwrap();
        let skills_dir = dir.path().join("skills");

        let mut registry = AgentSkillsRegistry::new(skills_dir.clone()).unwrap();

        let manifest = AgentSkillManifest {
            metadata: AgentSkillMetadata {
                id: "test-skill".to_string(),
                name: "Test Skill".to_string(),
                version: "1.0.0".to_string(),
                description: "A test skill".to_string(),
                long_description: None,
                author: None,
                license: None,
                tags: vec!["test".to_string()],
                categories: vec!["utility".to_string()],
                skill_type: AgentSkillType::Custom,
                priority: None,
                icon: None,
                homepage: None,
                repository: None,
                issues: None,
                keywords: vec!["test".to_string()],
                deprecated: false,
                deprecation_message: None,
            },
            parameters: vec![],
            returns: None,
            examples: vec![],
            dependencies: vec![],
            env_vars: vec![],
            permissions: vec![],
            timeout_seconds: None,
            retry_config: None,
        };

        registry.add(manifest).unwrap();

        assert_eq!(registry.list().len(), 1);
        assert!(registry.get("test-skill").is_some());
    }

    #[test]
    fn test_skill_md_parse() {
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

## 参数
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
    fn test_skill_md_to_agent_skill_manifest() {
        let content = r#"---
name: test-skill
description: A test skill for conversion
version: 2.1.0
author: Test Author
license: MIT
tags: ["test", "conversion"]
requires: ["another-skill"]
allowed-tools: ["Read", "Write"]
timeout: 60
---

# Test Skill

## 功能概述
This is a test skill.
"#;

        let skill_md = SkillMdDocument::parse(content).unwrap();
        let manifest: AgentSkillManifest = skill_md.into();

        assert_eq!(manifest.metadata.id, "test-skill");
        assert_eq!(manifest.metadata.name, "test-skill");
        assert_eq!(manifest.metadata.version, "2.1.0");
        assert_eq!(manifest.metadata.author, Some("Test Author".to_string()));
        assert_eq!(manifest.metadata.license, Some("MIT".to_string()));
        assert_eq!(manifest.dependencies, vec!["another-skill".to_string()]);
        assert_eq!(
            manifest.permissions,
            vec!["Read".to_string(), "Write".to_string()]
        );
        assert_eq!(manifest.timeout_seconds, Some(60));
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

        let manifest = AgentSkillManifest::from_path(&skill_md_path).unwrap();
        assert_eq!(manifest.metadata.id, "test-skill");
    }

    #[test]
    fn test_skill_md_from_path_without_validation() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("simple-skill");
        std::fs::create_dir(&skill_dir).unwrap();

        let skill_md_path = skill_dir.join("SKILL.md");
        let content = r#"---
name: simple-skill
description: Simple test skill
version: 1.0.0
---

# Simple Skill

## 功能概述
Simple content.
"#;
        std::fs::write(&skill_md_path, content).unwrap();

        let skill_md = SkillMdDocument::from_path(&skill_md_path, false).unwrap();
        assert_eq!(skill_md.frontmatter.name, "simple-skill");
    }

    #[test]
    fn test_skill_md_allowed_tools_valid() {
        let content = r#"---
name: test-skill
description: A test skill with valid allowed tools for testing purposes. Use this when you need to verify allowed tools parsing.
version: 1.0.0
allowed-tools: ["Read", "Write", "LLM", "Git"]
tags: ["test", "tools"]
timeout: 60
---

# Test Skill

## 功能概述
Test skill for valid allowed tools.

## 适用场景
Testing allowed tools validation.

## 输入规范
None.

## 执行步骤
1. Parse the allowed-tools field
2. Verify all tools are valid

## 输出规范
Valid SkillMdDocument.

## 约束与安全
None.

## 示例
No examples needed.
"#;
        let skill_md = SkillMdDocument::parse(content).unwrap();
        assert_eq!(
            skill_md.frontmatter.allowed_tools,
            Some(vec![
                "Read".to_string(),
                "Write".to_string(),
                "LLM".to_string(),
                "Git".to_string()
            ])
        );
    }

    #[test]
    fn test_skill_directory_structure_discover() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("test-skill");
        std::fs::create_dir(&skill_dir).unwrap();

        let skill_md_path = skill_dir.join("SKILL.md");
        let content = r#"---
name: test-skill
description: A test skill for directory structure discovery. Use this when testing the directory structure features.
version: 1.0.0
tags: ["test", "directory"]
timeout: 60
---

# Test Skill

## 功能概述
Test skill for directory structure.

## 适用场景
Testing directory structure discovery.

## 输入规范
None.

## 执行步骤
1. Discover the directory structure
2. Verify all optional components are found

## 输出规范
Valid SkillDirectoryStructure.

## 约束与安全
None.

## 示例
No examples needed.
"#;
        std::fs::write(&skill_md_path, content).unwrap();

        std::fs::create_dir(skill_dir.join("scripts")).unwrap();
        std::fs::create_dir(skill_dir.join("assets")).unwrap();
        std::fs::create_dir(skill_dir.join("references")).unwrap();
        std::fs::create_dir(skill_dir.join("sub-skills")).unwrap();
        std::fs::write(skill_dir.join("LICENSE"), "MIT License").unwrap();
        std::fs::write(skill_dir.join("README.md"), "# Test Skill").unwrap();

        let skill_md = SkillMdDocument::from_path(&skill_md_path, true).unwrap();
        let dir_struct = skill_md.directory_structure.unwrap();

        assert!(dir_struct.scripts_dir.is_some());
        assert!(dir_struct.assets_dir.is_some());
        assert!(dir_struct.references_dir.is_some());
        assert!(dir_struct.sub_skills_dir.is_some());
        assert!(dir_struct.license_file.is_some());
        assert!(dir_struct.readme_file.is_some());
    }

    #[test]
    fn test_skill_directory_structure_empty() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("empty-skill");
        std::fs::create_dir(&skill_dir).unwrap();

        let skill_md_path = skill_dir.join("SKILL.md");
        let content = r#"---
name: empty-skill
description: An empty skill for testing directory structure with no optional components. Use this to verify minimal structure.
version: 1.0.0
tags: ["test", "empty"]
timeout: 60
---

# Empty Skill

## 功能概述
Empty test skill.

## 适用场景
Testing minimal directory structure.

## 输入规范
None.

## 执行步骤
1. Load the skill
2. Verify no optional components are present

## 输出规范
Valid SkillDirectoryStructure with empty optional fields.

## 约束与安全
None.

## 示例
No examples needed.
"#;
        std::fs::write(&skill_md_path, content).unwrap();

        let skill_md = SkillMdDocument::from_path(&skill_md_path, true).unwrap();
        let dir_struct = skill_md.directory_structure.unwrap();

        assert!(dir_struct.scripts_dir.is_none());
        assert!(dir_struct.assets_dir.is_none());
        assert!(dir_struct.references_dir.is_none());
        assert!(dir_struct.sub_skills_dir.is_none());
        assert!(dir_struct.license_file.is_none());
        assert!(dir_struct.readme_file.is_none());
    }
}
