use crate::core::Task;
use crate::utils::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CritiqueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Critique {
    pub critique_id: String,
    pub task_id: String,
    pub severity: CritiqueSeverity,
    pub category: String,
    pub description: String,
    pub suggestion: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Critique {
    pub fn new(
        task_id: String,
        severity: CritiqueSeverity,
        category: String,
        description: String,
    ) -> Self {
        Self {
            critique_id: uuid::Uuid::new_v4().to_string(),
            task_id,
            severity,
            category,
            description,
            suggestion: String::new(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = suggestion;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetric {
    pub task_id: String,
    pub duration_seconds: f64,
    pub token_used: Option<u64>,
    pub steps_taken: u32,
    pub success: bool,
    pub retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Improvement {
    pub improvement_id: String,
    pub task_id: String,
    pub description: String,
    pub before_state: String,
    pub after_state: String,
    pub applied_at: Option<chrono::DateTime<chrono::Utc>>,
    pub effectiveness_score: f64,
}

impl Improvement {
    pub fn new(
        task_id: String,
        description: String,
        before_state: String,
        after_state: String,
    ) -> Self {
        Self {
            improvement_id: uuid::Uuid::new_v4().to_string(),
            task_id,
            description,
            before_state,
            after_state,
            applied_at: None,
            effectiveness_score: 0.0,
        }
    }

    pub fn mark_applied(&mut self) {
        self.applied_at = Some(chrono::Utc::now());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub report_id: String,
    pub task_id: String,
    pub success: bool,
    pub duration_seconds: f64,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub critiques: Vec<Critique>,
    pub lessons_learned: Vec<String>,
    pub improvements: Vec<Improvement>,
    pub metrics: ExecutionMetric,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl ExecutionReport {
    pub fn new(task_id: String, success: bool) -> Self {
        let now = chrono::Utc::now();
        let task_id_clone = task_id.clone();
        Self {
            report_id: uuid::Uuid::new_v4().to_string(),
            task_id,
            success,
            duration_seconds: 0.0,
            start_time: None,
            end_time: None,
            critiques: Vec::new(),
            lessons_learned: Vec::new(),
            improvements: Vec::new(),
            metrics: ExecutionMetric {
                task_id: task_id_clone,
                duration_seconds: 0.0,
                token_used: None,
                steps_taken: 0,
                success,
                retries: 0,
            },
            created_at: now,
        }
    }

    pub fn with_duration(
        mut self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self.duration_seconds = (end - start).num_milliseconds() as f64 / 1000.0;
        self.metrics.duration_seconds = self.duration_seconds;
        self
    }

    pub fn add_critique(&mut self, critique: Critique) {
        self.critiques.push(critique);
    }

    pub fn add_lesson(&mut self, lesson: String) {
        self.lessons_learned.push(lesson);
    }

    pub fn add_improvement(&mut self, improvement: Improvement) {
        self.improvements.push(improvement);
    }

    pub fn get_critiques_by_severity(&self, severity: &CritiqueSeverity) -> Vec<&Critique> {
        self.critiques
            .iter()
            .filter(|c| c.severity == *severity)
            .collect()
    }

    pub fn has_errors(&self, severity: &CritiqueSeverity) -> bool {
        self.critiques.iter().any(|c| c.severity == *severity)
    }
}

pub struct ExecutionHistory {
    reports: Arc<DashMap<String, ExecutionReport>>,
    task_reports: Arc<DashMap<String, Vec<String>>>,
}

impl ExecutionHistory {
    pub fn new() -> Self {
        Self {
            reports: Arc::new(DashMap::new()),
            task_reports: Arc::new(DashMap::new()),
        }
    }

    pub fn store_report(&self, report: ExecutionReport) {
        let report_id = report.report_id.clone();
        let task_id = report.task_id.clone();

        self.reports.insert(report_id.clone(), report);

        self.task_reports
            .entry(task_id)
            .or_default()
            .push(report_id);
    }

    pub fn get_report(&self, report_id: &str) -> Option<ExecutionReport> {
        self.reports.get(report_id).map(|r| r.value().clone())
    }

    pub fn get_reports_for_task(&self, task_id: &str) -> Vec<ExecutionReport> {
        self.task_reports
            .get(task_id)
            .map(|report_ids| {
                report_ids
                    .iter()
                    .filter_map(|id| self.reports.get(id).map(|r| r.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_latest_report_for_task(&self, task_id: &str) -> Option<ExecutionReport> {
        self.get_reports_for_task(task_id).into_iter().last()
    }
}

impl Default for ExecutionHistory {
    fn default() -> Self {
        Self::new()
    }
}

type CritiqueRuleFn = Box<dyn Fn(&Task, &ExecutionReport) -> Vec<Critique> + Send + Sync>;

pub struct Reflector {
    history: Arc<ExecutionHistory>,
    critique_rules: Vec<CritiqueRuleFn>,
}

impl Reflector {
    pub fn new() -> Self {
        Self {
            history: Arc::new(ExecutionHistory::new()),
            critique_rules: Vec::new(),
        }
    }

    pub fn with_history(mut self, history: ExecutionHistory) -> Self {
        self.history = Arc::new(history);
        self
    }

    pub fn register_critique_rule<F>(&mut self, rule: F)
    where
        F: Fn(&Task, &ExecutionReport) -> Vec<Critique> + Send + Sync + 'static,
    {
        self.critique_rules.push(Box::new(rule));
    }

    pub async fn analyze(&self, task: &Task) -> Result<ExecutionReport> {
        info!("Analyzing task execution: {}", task.id);

        let success = task.status == crate::core::TaskStatus::Completed;
        let mut report = ExecutionReport::new(task.id.clone(), success);

        let default_critiques = self.generate_default_critiques(task, &report);
        for critique in default_critiques {
            report.add_critique(critique);
        }

        for rule in &self.critique_rules {
            let critiques = rule(task, &report);
            for critique in critiques {
                report.add_critique(critique);
            }
        }

        let lessons = self.generate_lessons(task, &report);
        for lesson in lessons {
            report.add_lesson(lesson);
        }

        self.history.store_report(report.clone());

        Ok(report)
    }

    fn generate_default_critiques(&self, task: &Task, _report: &ExecutionReport) -> Vec<Critique> {
        let mut critiques = Vec::new();

        if task.description.len() < 20 {
            critiques.push(
                Critique::new(
                    task.id.clone(),
                    CritiqueSeverity::Warning,
                    "TaskDescription".to_string(),
                    "Task description is too short".to_string(),
                )
                .with_suggestion("Provide more detailed task description".to_string()),
            );
        }

        if task.priority > 7 {
            critiques.push(Critique::new(
                task.id.clone(),
                CritiqueSeverity::Info,
                "TaskPriority".to_string(),
                "Task has high priority".to_string(),
            ));
        }

        critiques
    }

    fn generate_lessons(&self, _task: &Task, report: &ExecutionReport) -> Vec<String> {
        let mut lessons = Vec::new();

        if report.success {
            lessons.push("Task completed successfully".to_string());
        } else {
            lessons.push("Task failed, review error logs".to_string());
        }

        if report.duration_seconds > 10.0 {
            lessons.push("Consider optimizing long-running tasks".to_string());
        }

        for critique in &report.critiques {
            if !critique.suggestion.is_empty() {
                lessons.push(format!("Suggestion: {}", critique.suggestion));
            }
        }

        lessons
    }

    pub async fn optimize(&self, report: &ExecutionReport) -> Result<Vec<Improvement>> {
        info!("Optimizing based on report: {}", report.report_id);

        let mut improvements = Vec::new();

        for critique in &report.critiques {
            if !critique.suggestion.is_empty() {
                let improvement = Improvement::new(
                    report.task_id.clone(),
                    critique.description.clone(),
                    format!("Before: {}", critique.description),
                    format!("After: {}", critique.suggestion),
                );
                improvements.push(improvement);
            }
        }

        Ok(improvements)
    }

    pub async fn generate_strategy_optimization(&self, task: &Task) -> Result<Vec<String>> {
        info!("Generating strategy optimization for task: {}", task.id);

        let mut optimizations = Vec::new();
        let previous_reports = self.history.get_reports_for_task(&task.id);

        if !previous_reports.is_empty() {
            let success_rate = previous_reports.iter().filter(|r| r.success).count() as f64
                / previous_reports.len() as f64;

            if success_rate < 0.5 {
                optimizations.push("Consider simplifying task approach".to_string());
                optimizations.push("Break task into smaller steps".to_string());
            }

            let avg_duration = previous_reports
                .iter()
                .map(|r| r.duration_seconds)
                .sum::<f64>()
                / previous_reports.len() as f64;

            if avg_duration > 30.0 {
                optimizations.push("Consider parallel execution".to_string());
            }
        }

        Ok(optimizations)
    }

    pub fn get_history(&self) -> Arc<ExecutionHistory> {
        self.history.clone()
    }
}

impl Default for Reflector {
    fn default() -> Self {
        Self::new()
    }
}
