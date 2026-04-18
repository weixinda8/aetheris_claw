use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DSLFormat {
    YAML,
    JSON,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDSL {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub tasks: Vec<TaskDefinition>,
    pub dependencies: Vec<DependencyDefinition>,
    pub conditions: Option<Vec<ConditionDefinition>>,
    pub loops: Option<Vec<LoopDefinition>>,
    pub exception_handling: Option<ExceptionHandlingConfig>,
    pub metadata: Option<serde_json::Value>,
}

impl WorkflowDSL {
    pub fn new(name: String, version: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            version,
            description: None,
            tasks: Vec::new(),
            dependencies: Vec::new(),
            conditions: None,
            loops: None,
            exception_handling: None,
            metadata: None,
        }
    }

    pub fn from_yaml(yaml_str: &str) -> Result<Self, WorkflowDSLError> {
        serde_yaml::from_str(yaml_str).map_err(WorkflowDSLError::YamlParseError)
    }

    pub fn from_json(json_str: &str) -> Result<Self, WorkflowDSLError> {
        serde_json::from_str(json_str).map_err(WorkflowDSLError::JsonParseError)
    }

    pub fn to_yaml(&self) -> Result<String, WorkflowDSLError> {
        serde_yaml::to_string(self).map_err(WorkflowDSLError::YamlSerializeError)
    }

    pub fn to_json(&self) -> Result<String, WorkflowDSLError> {
        serde_json::to_string_pretty(self).map_err(WorkflowDSLError::JsonSerializeError)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub id: String,
    pub name: String,
    pub task_type: String,
    pub description: Option<String>,
    pub agent_id: Option<String>,
    pub required_capabilities: Option<Vec<String>>,
    pub inputs: Option<HashMap<String, String>>,
    pub outputs: Option<HashMap<String, String>>,
    pub timeout_seconds: Option<u64>,
    pub retry_config: Option<RetryConfig>,
    pub metadata: Option<serde_json::Value>,
}

impl TaskDefinition {
    pub fn new(id: String, name: String, task_type: String) -> Self {
        Self {
            id,
            name,
            task_type,
            description: None,
            agent_id: None,
            required_capabilities: None,
            inputs: None,
            outputs: None,
            timeout_seconds: None,
            retry_config: None,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyDefinition {
    pub source_task_id: String,
    pub target_task_id: String,
    pub condition: Option<String>,
    pub data_mapping: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionDefinition {
    pub id: String,
    pub expression: String,
    pub true_branch: Vec<String>,
    pub false_branch: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopDefinition {
    pub id: String,
    pub task_ids: Vec<String>,
    pub max_iterations: Option<u32>,
    pub condition: Option<String>,
    pub iterator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionHandlingConfig {
    pub default_strategy: ExceptionStrategy,
    pub task_strategies: Option<HashMap<String, ExceptionStrategy>>,
    pub on_error_tasks: Option<Vec<String>>,
    pub finally_tasks: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExceptionStrategy {
    Fail,
    Retry,
    Skip,
    Custom(String),
}

impl Default for ExceptionStrategy {
    fn default() -> Self {
        ExceptionStrategy::Fail
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub delay_seconds: u64,
    pub backoff_multiplier: Option<f64>,
    pub retryable_errors: Option<Vec<String>>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            delay_seconds: 1,
            backoff_multiplier: Some(2.0),
            retryable_errors: None,
        }
    }
}

#[derive(Debug)]
pub enum WorkflowDSLError {
    YamlParseError(serde_yaml::Error),
    YamlSerializeError(serde_yaml::Error),
    JsonParseError(serde_json::Error),
    JsonSerializeError(serde_json::Error),
    ValidationError(Vec<String>),
}

impl std::fmt::Display for WorkflowDSLError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowDSLError::YamlParseError(e) => write!(f, "YAML parse error: {}", e),
            WorkflowDSLError::YamlSerializeError(e) => write!(f, "YAML serialize error: {}", e),
            WorkflowDSLError::JsonParseError(e) => write!(f, "JSON parse error: {}", e),
            WorkflowDSLError::JsonSerializeError(e) => write!(f, "JSON serialize error: {}", e),
            WorkflowDSLError::ValidationError(errors) => {
                write!(f, "Validation errors: {}", errors.join(", "))
            }
        }
    }
}

impl std::error::Error for WorkflowDSLError {}

pub struct WorkflowDSLValidator;

impl WorkflowDSLValidator {
    pub fn validate(workflow: &WorkflowDSL) -> Result<ValidationResult, WorkflowDSLError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if workflow.tasks.is_empty() {
            errors.push("Workflow must contain at least one task".to_string());
        }

        let task_ids: HashSet<_> = workflow.tasks.iter().map(|t| t.id.clone()).collect();

        for task in &workflow.tasks {
            if task.id.is_empty() {
                errors.push(format!("Task '{}' has empty ID", task.name));
            }
            if task.name.is_empty() {
                errors.push(format!("Task with ID '{}' has empty name", task.id));
            }
            if task.task_type.is_empty() {
                errors.push(format!("Task '{}' has empty type", task.name));
            }
        }

        for dep in &workflow.dependencies {
            if !task_ids.contains(&dep.source_task_id) {
                errors.push(format!(
                    "Dependency source task '{}' not found",
                    dep.source_task_id
                ));
            }
            if !task_ids.contains(&dep.target_task_id) {
                errors.push(format!(
                    "Dependency target task '{}' not found",
                    dep.target_task_id
                ));
            }
        }

        if let Some(conditions) = &workflow.conditions {
            for cond in conditions {
                for task_id in &cond.true_branch {
                    if !task_ids.contains(task_id) {
                        errors.push(format!(
                            "Condition '{}' references unknown task '{}' in true branch",
                            cond.id, task_id
                        ));
                    }
                }
                if let Some(false_branch) = &cond.false_branch {
                    for task_id in false_branch {
                        if !task_ids.contains(task_id) {
                            errors.push(format!(
                                "Condition '{}' references unknown task '{}' in false branch",
                                cond.id, task_id
                            ));
                        }
                    }
                }
            }
        }

        if let Some(loops) = &workflow.loops {
            for loop_def in loops {
                for task_id in &loop_def.task_ids {
                    if !task_ids.contains(task_id) {
                        errors.push(format!(
                            "Loop '{}' references unknown task '{}'",
                            loop_def.id, task_id
                        ));
                    }
                }
            }
        }

        if let Some(exception_config) = &workflow.exception_handling {
            if let Some(task_strategies) = &exception_config.task_strategies {
                for task_id in task_strategies.keys() {
                    if !task_ids.contains(task_id) {
                        warnings.push(format!(
                            "Exception strategy references unknown task '{}'",
                            task_id
                        ));
                    }
                }
            }
            if let Some(on_error_tasks) = &exception_config.on_error_tasks {
                for task_id in on_error_tasks {
                    if !task_ids.contains(task_id) {
                        errors.push(format!("On-error task '{}' not found", task_id));
                    }
                }
            }
            if let Some(finally_tasks) = &exception_config.finally_tasks {
                for task_id in finally_tasks {
                    if !task_ids.contains(task_id) {
                        errors.push(format!("Finally task '{}' not found", task_id));
                    }
                }
            }
        }

        if Self::has_cyclic_dependency(workflow) {
            errors.push("Workflow has cyclic dependencies".to_string());
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    fn has_cyclic_dependency(workflow: &WorkflowDSL) -> bool {
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();

        let mut adjacency_list: HashMap<String, Vec<String>> = HashMap::new();
        for dep in &workflow.dependencies {
            adjacency_list
                .entry(dep.source_task_id.clone())
                .or_default()
                .push(dep.target_task_id.clone());
        }

        for task in &workflow.tasks {
            if Self::dfs(
                &task.id,
                &adjacency_list,
                &mut visited,
                &mut recursion_stack,
            ) {
                return true;
            }
        }

        false
    }

    fn dfs(
        task_id: &str,
        adjacency_list: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        recursion_stack: &mut HashSet<String>,
    ) -> bool {
        if recursion_stack.contains(task_id) {
            return true;
        }
        if visited.contains(task_id) {
            return false;
        }

        visited.insert(task_id.to_string());
        recursion_stack.insert(task_id.to_string());

        if let Some(neighbors) = adjacency_list.get(task_id) {
            for neighbor in neighbors {
                if Self::dfs(neighbor, adjacency_list, visited, recursion_stack) {
                    return true;
                }
            }
        }

        recursion_stack.remove(task_id);
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}
