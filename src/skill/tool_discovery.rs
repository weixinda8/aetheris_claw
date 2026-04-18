use crate::skill::{PermissionLevel, Skill, SkillEvaluation, SkillMetadata, Version};
use crate::utils::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub scan_paths: Vec<String>,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub auto_register: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            scan_paths: vec!["./skills".to_string()],
            include_patterns: vec![
                "*.toml".to_string(),
                "*.yaml".to_string(),
                "*.yml".to_string(),
            ],
            exclude_patterns: Vec::new(),
            auto_register: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationCriteria {
    pub relevance_weight: f64,
    pub performance_weight: f64,
    pub reliability_weight: f64,
    pub min_relevance_score: f64,
    pub min_performance_score: f64,
    pub min_reliability_score: f64,
}

impl Default for EvaluationCriteria {
    fn default() -> Self {
        Self {
            relevance_weight: 0.4,
            performance_weight: 0.3,
            reliability_weight: 0.3,
            min_relevance_score: 0.3,
            min_performance_score: 0.3,
            min_reliability_score: 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionContext {
    pub task_description: String,
    pub required_tags: Vec<String>,
    pub required_categories: Vec<String>,
    pub preferred_call_mode: Option<crate::skill::CallMode>,
    pub required_permissions: Vec<String>,
    pub max_permission_level: PermissionLevel,
    pub prefer_active: bool,
    pub prefer_non_deprecated: bool,
    pub min_version: Option<Version>,
    pub evaluation_criteria: EvaluationCriteria,
}

impl Default for SelectionContext {
    fn default() -> Self {
        Self {
            task_description: String::new(),
            required_tags: Vec::new(),
            required_categories: Vec::new(),
            preferred_call_mode: None,
            required_permissions: Vec::new(),
            max_permission_level: PermissionLevel::Public,
            prefer_active: true,
            prefer_non_deprecated: true,
            min_version: None,
            evaluation_criteria: EvaluationCriteria::default(),
        }
    }
}

pub struct ToolDiscovery {
    config: DiscoveryConfig,
    skill_registry: Arc<crate::skill::registry::SkillRegistry>,
}

impl ToolDiscovery {
    pub fn new(
        config: DiscoveryConfig,
        skill_registry: Arc<crate::skill::registry::SkillRegistry>,
    ) -> Self {
        Self {
            config,
            skill_registry,
        }
    }

    pub fn with_default_config(skill_registry: Arc<crate::skill::registry::SkillRegistry>) -> Self {
        Self::new(DiscoveryConfig::default(), skill_registry)
    }

    pub async fn discover(&self) -> Result<Vec<SkillMetadata>> {
        info!("Starting tool discovery with config: {:?}", self.config);

        let discovered_skills = Vec::new();

        for path in &self.config.scan_paths {
            debug!("Scanning path: {}", path);

            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        let pattern = format!("*.{}", ext_str);

                        if self.config.include_patterns.contains(&pattern) {
                            debug!("Found potential skill file: {:?}", path);
                        }
                    }
                }
            }
        }

        Ok(discovered_skills)
    }

    pub async fn evaluate_skill(
        &self,
        skill: &Arc<dyn Skill>,
        context: &SelectionContext,
    ) -> Result<SkillEvaluation> {
        let metadata = skill.metadata();
        let mut evaluation = SkillEvaluation::new(metadata.id.clone(), metadata.version.clone());

        evaluation.relevance_score = self.calculate_relevance_score(metadata, context);
        evaluation.performance_score = self.calculate_performance_score(metadata);
        evaluation.reliability_score = self.calculate_reliability_score(metadata);
        evaluation.calculate_overall();

        evaluation.evaluation_criteria.insert(
            "relevance_weight".to_string(),
            context.evaluation_criteria.relevance_weight.to_string(),
        );
        evaluation.evaluation_criteria.insert(
            "performance_weight".to_string(),
            context.evaluation_criteria.performance_weight.to_string(),
        );
        evaluation.evaluation_criteria.insert(
            "reliability_weight".to_string(),
            context.evaluation_criteria.reliability_weight.to_string(),
        );

        Ok(evaluation)
    }

    pub async fn evaluate(
        &self,
        skills: &[Arc<dyn Skill>],
        context: &SelectionContext,
    ) -> Result<Vec<SkillEvaluation>> {
        let mut evaluations = Vec::with_capacity(skills.len());

        for skill in skills {
            let evaluation = self.evaluate_skill(skill, context).await?;
            evaluations.push(evaluation);
        }

        Ok(evaluations)
    }

    fn calculate_relevance_score(
        &self,
        metadata: &SkillMetadata,
        context: &SelectionContext,
    ) -> f64 {
        let mut score = 0.0;
        let mut max_score = 0.0;

        if !context.task_description.is_empty() {
            max_score += 0.3;
            let task_lower = context.task_description.to_lowercase();
            let name_match = metadata.name.to_lowercase().contains(&task_lower);
            let desc_match = metadata.description.to_lowercase().contains(&task_lower);
            if name_match || desc_match {
                score += 0.3;
            } else if metadata
                .tags
                .iter()
                .any(|t| task_lower.contains(&t.to_lowercase()))
            {
                score += 0.15;
            }
        }

        if !context.required_tags.is_empty() {
            max_score += 0.3;
            let matching_tags: usize = context
                .required_tags
                .iter()
                .filter(|t| metadata.tags.contains(t))
                .count();
            score += 0.3 * (matching_tags as f64 / context.required_tags.len() as f64);
        }

        if !context.required_categories.is_empty() {
            max_score += 0.2;
            let matching_categories: usize = context
                .required_categories
                .iter()
                .filter(|c| metadata.categories.contains(c))
                .count();
            score += 0.2 * (matching_categories as f64 / context.required_categories.len() as f64);
        }

        if let Some(preferred_mode) = &context.preferred_call_mode {
            max_score += 0.1;
            if metadata.call_mode == *preferred_mode {
                score += 0.1;
            }
        }

        if context.prefer_active && metadata.is_active {
            max_score += 0.05;
            score += 0.05;
        }

        if context.prefer_non_deprecated && !metadata.is_deprecated {
            max_score += 0.05;
            score += 0.05;
        }

        if max_score == 0.0 {
            0.5
        } else {
            score / max_score
        }
    }

    fn calculate_performance_score(&self, _metadata: &SkillMetadata) -> f64 {
        0.8
    }

    fn calculate_reliability_score(&self, metadata: &SkillMetadata) -> f64 {
        let mut score: f64 = 0.5;

        if metadata.version.major > 0 || metadata.version.minor > 0 {
            score += 0.2;
        }

        if metadata.input_schema.is_some() {
            score += 0.1;
        }

        if metadata.output_schema.is_some() {
            score += 0.1;
        }

        if metadata.example_input.is_some() && metadata.example_output.is_some() {
            score += 0.1;
        }

        score.min(1.0)
    }

    pub async fn select_best_skill(
        &self,
        skills: &[Arc<dyn Skill>],
        context: &SelectionContext,
    ) -> Result<Option<Arc<dyn Skill>>> {
        if skills.is_empty() {
            return Ok(None);
        }

        let evaluations = self.evaluate(skills, context).await?;

        let filtered_evaluations: Vec<_> = evaluations
            .into_iter()
            .filter(|e| {
                e.relevance_score >= context.evaluation_criteria.min_relevance_score
                    && e.performance_score >= context.evaluation_criteria.min_performance_score
                    && e.reliability_score >= context.evaluation_criteria.min_reliability_score
            })
            .collect();

        if filtered_evaluations.is_empty() {
            return Ok(None);
        }

        let best_evaluation = filtered_evaluations
            .into_iter()
            .max_by(|a, b| {
                a.overall_score
                    .partial_cmp(&b.overall_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        let best_skill = skills
            .iter()
            .find(|s| {
                s.metadata().id == best_evaluation.skill_id
                    && s.metadata().version == best_evaluation.version
            })
            .cloned();

        Ok(best_skill)
    }

    pub async fn select_skills(
        &self,
        skills: &[Arc<dyn Skill>],
        context: &SelectionContext,
        limit: usize,
    ) -> Result<Vec<(Arc<dyn Skill>, SkillEvaluation)>> {
        let evaluations = self.evaluate(skills, context).await?;

        let mut skill_evaluations: Vec<_> = skills
            .iter()
            .zip(evaluations.into_iter())
            .filter(|(_, e)| {
                e.relevance_score >= context.evaluation_criteria.min_relevance_score
                    && e.performance_score >= context.evaluation_criteria.min_performance_score
                    && e.reliability_score >= context.evaluation_criteria.min_reliability_score
            })
            .collect();

        skill_evaluations.sort_by(|(_, a), (_, b)| {
            b.overall_score
                .partial_cmp(&a.overall_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(skill_evaluations
            .into_iter()
            .take(limit)
            .map(|(skill, eval)| (skill.clone(), eval))
            .collect())
    }

    pub fn filter_skills_by_permission(
        &self,
        skills: &[Arc<dyn Skill>],
        context: &SelectionContext,
    ) -> Vec<Arc<dyn Skill>> {
        let max_level = context.max_permission_level.clone();
        skills
            .iter()
            .filter(|skill| {
                let metadata = skill.metadata().clone();

                if metadata.permission_level > max_level {
                    return false;
                }

                for required_perm in &context.required_permissions {
                    if !metadata.required_permissions.contains(required_perm) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect()
    }

    pub async fn discover_and_select(
        &self,
        context: &SelectionContext,
        limit: usize,
    ) -> Result<Vec<(Arc<dyn Skill>, SkillEvaluation)>> {
        let _discovered = self.discover().await?;

        let registered_skills: Vec<_> = self
            .skill_registry
            .list()
            .into_iter()
            .filter_map(|m| self.skill_registry.get(&m.id))
            .collect();

        let filtered_skills = self.filter_skills_by_permission(&registered_skills, context);

        self.select_skills(&filtered_skills, context, limit).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SkillConfig {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub long_description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub author: Option<String>,
    pub call_mode: Option<String>,
    pub permission_level: Option<String>,
    pub required_permissions: Option<Vec<String>>,
}

impl From<SkillConfig> for SkillMetadata {
    fn from(config: SkillConfig) -> Self {
        let version =
            Version::from_string(&config.version).unwrap_or_else(|_| Version::new(0, 1, 0));

        let call_mode = match config.call_mode.as_deref() {
            Some("api") => crate::skill::CallMode::Api,
            Some("database") => crate::skill::CallMode::Database,
            Some("image") => crate::skill::CallMode::Image,
            Some("audio") => crate::skill::CallMode::Audio,
            Some("hybrid") => crate::skill::CallMode::Hybrid,
            _ => crate::skill::CallMode::Text,
        };

        let permission_level = match config.permission_level.as_deref() {
            Some("internal") => PermissionLevel::Internal,
            Some("restricted") => PermissionLevel::Restricted,
            Some("admin") => PermissionLevel::Admin,
            _ => PermissionLevel::Public,
        };

        SkillMetadata::new(config.id, config.name, version, config.description)
            .with_tags(config.tags.unwrap_or_default())
            .with_call_mode(call_mode)
            .with_permission_level(permission_level)
    }
}
