use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskComplexity {
    Simple,
    Medium,
    Complex,
    VeryComplex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub complexity: TaskComplexity,
    pub estimated_duration_minutes: u32,
    pub required_capabilities: Vec<String>,
    pub dependencies: Vec<String>,
    pub priority: u8,
    pub metadata: Option<serde_json::Value>,
}

impl SubTask {
    pub fn new(name: String, description: String, complexity: TaskComplexity) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            complexity,
            estimated_duration_minutes: 0,
            required_capabilities: Vec::new(),
            dependencies: Vec::new(),
            priority: 0,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposedTask {
    pub original_task_id: String,
    pub original_description: String,
    pub sub_tasks: Vec<SubTask>,
    pub requires_human_review: bool,
    pub review_notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub task_type: String,
    pub sub_task_patterns: Vec<SubTaskPattern>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskPattern {
    pub name_template: String,
    pub description_template: String,
    pub complexity: TaskComplexity,
    pub estimated_duration_minutes: u32,
    pub required_capabilities: Vec<String>,
    pub dependency_templates: Vec<String>,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilityMatch {
    pub agent_id: String,
    pub capability_score: f64,
    pub matched_capabilities: Vec<String>,
    pub missing_capabilities: Vec<String>,
}

#[async_trait]
pub trait TaskDecomposer: Send + Sync {
    async fn decompose_task(
        &self,
        task_description: &str,
        options: DecompositionOptions,
    ) -> crate::utils::Result<DecomposedTask>;

    async fn validate_decomposition(
        &self,
        decomposed: &DecomposedTask,
    ) -> crate::utils::Result<DecompositionValidationResult>;

    async fn suggest_agents(
        &self,
        sub_task: &SubTask,
        available_agents: &[AgentInfo],
    ) -> crate::utils::Result<Vec<AgentCapabilityMatch>>;

    fn add_template(&mut self, template: DecompositionTemplate) -> crate::utils::Result<()>;

    fn get_template(&self, template_id: &str) -> Option<&DecompositionTemplate>;

    fn list_templates(&self, task_type: Option<&str>) -> Vec<&DecompositionTemplate>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub agent_id: String,
    pub capabilities: Vec<String>,
    pub current_load: f64,
    pub performance_score: f64,
    pub location: Option<String>,
    pub cost_per_minute: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionOptions {
    pub max_sub_tasks: Option<usize>,
    pub min_confidence: Option<f64>,
    pub require_human_review: bool,
    pub use_templates: bool,
    pub preferred_template_ids: Option<Vec<String>>,
    pub available_agents: Option<Vec<AgentInfo>>,
}

impl Default for DecompositionOptions {
    fn default() -> Self {
        Self {
            max_sub_tasks: Some(20),
            min_confidence: Some(0.7),
            require_human_review: false,
            use_templates: true,
            preferred_template_ids: None,
            available_agents: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}

pub struct LlmTaskDecomposer {
    templates: HashMap<String, DecompositionTemplate>,
}

impl LlmTaskDecomposer {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    fn analyze_task_complexity(&self, description: &str) -> TaskComplexity {
        let word_count = description.split_whitespace().count();
        let lower_desc = description.to_lowercase();
        let has_complex_terms = lower_desc.contains("integrate")
            || lower_desc.contains("coordinate")
            || lower_desc.contains("orchestrate")
            || lower_desc.contains("synchronize");

        match (word_count, has_complex_terms) {
            (0..=20, false) => TaskComplexity::Simple,
            (21..=50, false) => TaskComplexity::Medium,
            (51..=100, _) => TaskComplexity::Complex,
            _ => TaskComplexity::VeryComplex,
        }
    }

    fn generate_basic_subtasks(
        &self,
        description: &str,
        complexity: &TaskComplexity,
    ) -> Vec<SubTask> {
        let mut sub_tasks = Vec::new();

        match complexity {
            TaskComplexity::Simple => {
                sub_tasks.push(SubTask::new(
                    "Execute Task".to_string(),
                    description.to_string(),
                    TaskComplexity::Simple,
                ));
            }
            TaskComplexity::Medium => {
                sub_tasks.push(SubTask::new(
                    "Analyze Requirements".to_string(),
                    "Analyze the task requirements and constraints".to_string(),
                    TaskComplexity::Simple,
                ));
                sub_tasks.push(SubTask::new(
                    "Execute Main Task".to_string(),
                    description.to_string(),
                    TaskComplexity::Medium,
                ));
                sub_tasks.push(SubTask::new(
                    "Verify Results".to_string(),
                    "Verify the task results are correct".to_string(),
                    TaskComplexity::Simple,
                ));
            }
            TaskComplexity::Complex | TaskComplexity::VeryComplex => {
                sub_tasks.push(SubTask::new(
                    "Requirements Analysis".to_string(),
                    "Detailed analysis of task requirements and constraints".to_string(),
                    TaskComplexity::Medium,
                ));
                sub_tasks.push(SubTask::new(
                    "Task Planning".to_string(),
                    "Create detailed execution plan".to_string(),
                    TaskComplexity::Medium,
                ));
                sub_tasks.push(SubTask::new(
                    "Resource Allocation".to_string(),
                    "Allocate necessary resources for task execution".to_string(),
                    TaskComplexity::Medium,
                ));
                sub_tasks.push(SubTask::new(
                    "Task Execution".to_string(),
                    description.to_string(),
                    TaskComplexity::Complex,
                ));
                sub_tasks.push(SubTask::new(
                    "Quality Assurance".to_string(),
                    "Perform quality checks on the results".to_string(),
                    TaskComplexity::Medium,
                ));
                sub_tasks.push(SubTask::new(
                    "Result Delivery".to_string(),
                    "Package and deliver the final results".to_string(),
                    TaskComplexity::Simple,
                ));
            }
        }

        let ids: Vec<String> = sub_tasks.iter().map(|t| t.id.clone()).collect();
        for (i, task) in sub_tasks.iter_mut().enumerate() {
            task.priority = i as u8;
            if i > 0 {
                task.dependencies.push(ids[i - 1].clone());
            }
        }

        sub_tasks
    }

    fn infer_dependencies(&self, sub_tasks: &mut [SubTask]) {
        let mut capability_groups: HashMap<String, Vec<usize>> = HashMap::new();

        for (idx, task) in sub_tasks.iter().enumerate() {
            for capability in &task.required_capabilities {
                capability_groups
                    .entry(capability.clone())
                    .or_default()
                    .push(idx);
            }
        }
    }
}

#[async_trait]
impl TaskDecomposer for LlmTaskDecomposer {
    async fn decompose_task(
        &self,
        task_description: &str,
        options: DecompositionOptions,
    ) -> crate::utils::Result<DecomposedTask> {
        let complexity = self.analyze_task_complexity(task_description);
        let mut sub_tasks = self.generate_basic_subtasks(task_description, &complexity);

        if let Some(max) = options.max_sub_tasks {
            if sub_tasks.len() > max {
                sub_tasks.truncate(max);
            }
        }

        self.infer_dependencies(&mut sub_tasks);

        let confidence = match complexity {
            TaskComplexity::Simple => 0.95,
            TaskComplexity::Medium => 0.85,
            TaskComplexity::Complex => 0.75,
            TaskComplexity::VeryComplex => 0.65,
        };

        let requires_human_review = options.require_human_review || confidence < 0.7;

        Ok(DecomposedTask {
            original_task_id: Uuid::new_v4().to_string(),
            original_description: task_description.to_string(),
            sub_tasks,
            requires_human_review,
            review_notes: if requires_human_review {
                Some("Please review the task decomposition before execution".to_string())
            } else {
                None
            },
            created_at: chrono::Utc::now(),
            confidence,
        })
    }

    async fn validate_decomposition(
        &self,
        decomposed: &DecomposedTask,
    ) -> crate::utils::Result<DecompositionValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        if decomposed.sub_tasks.is_empty() {
            errors.push("No subtasks defined".to_string());
        }

        let task_ids: HashSet<_> = decomposed.sub_tasks.iter().map(|t| t.id.clone()).collect();
        for task in &decomposed.sub_tasks {
            for dep in &task.dependencies {
                if !task_ids.contains(dep) {
                    errors.push(format!("Invalid dependency: {}", dep));
                }
            }
        }

        if decomposed.confidence < 0.7 {
            warnings.push("Low confidence in decomposition".to_string());
            suggestions
                .push("Consider using a decomposition template or manual review".to_string());
        }

        Ok(DecompositionValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            suggestions,
        })
    }

    async fn suggest_agents(
        &self,
        sub_task: &SubTask,
        available_agents: &[AgentInfo],
    ) -> crate::utils::Result<Vec<AgentCapabilityMatch>> {
        let mut matches = Vec::new();

        for agent in available_agents {
            let agent_capabilities: HashSet<_> = agent.capabilities.iter().cloned().collect();
            let required_capabilities: HashSet<_> =
                sub_task.required_capabilities.iter().cloned().collect();

            let matched: Vec<_> = agent_capabilities
                .intersection(&required_capabilities)
                .cloned()
                .collect();
            let missing: Vec<_> = required_capabilities
                .difference(&agent_capabilities)
                .cloned()
                .collect();

            let score = if required_capabilities.is_empty() {
                1.0
            } else {
                matched.len() as f64 / required_capabilities.len() as f64
            };

            matches.push(AgentCapabilityMatch {
                agent_id: agent.agent_id.clone(),
                capability_score: score,
                matched_capabilities: matched,
                missing_capabilities: missing,
            });
        }

        matches.sort_by(|a, b| b.capability_score.partial_cmp(&a.capability_score).unwrap());

        Ok(matches)
    }

    fn add_template(&mut self, template: DecompositionTemplate) -> crate::utils::Result<()> {
        self.templates.insert(template.id.clone(), template);
        Ok(())
    }

    fn get_template(&self, template_id: &str) -> Option<&DecompositionTemplate> {
        self.templates.get(template_id)
    }

    fn list_templates(&self, task_type: Option<&str>) -> Vec<&DecompositionTemplate> {
        self.templates
            .values()
            .filter(|t| task_type.map(|tt| t.task_type == tt).unwrap_or(true))
            .collect()
    }
}

impl Default for LlmTaskDecomposer {
    fn default() -> Self {
        Self::new()
    }
}
