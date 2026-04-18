use crate::agent::base::{Agent, AgentConfig, AgentState, AgentType, BaseAgent};
use crate::core::Task;
use crate::core::llm::manager::LlmManager;
use crate::core::plan_execute::ReActStep;
use crate::core::progressive_loading::{LoadingStrategy, ProgressiveLoader};
use crate::skill::registry::SkillRegistry;
use crate::utils::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub sections: Vec<DocumentSection>,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSection {
    pub title: String,
    pub content: String,
    pub level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub period: String,
    pub metrics: Vec<ReportMetric>,
    pub sections: Vec<ReportSection>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetric {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub change: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    pub id: String,
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub body: String,
    pub attachments: Vec<String>,
    pub status: EmailStatus,
    pub sent_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EmailStatus {
    Draft,
    Queued,
    Sent,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: String,
    pub title: String,
    pub description: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub location: Option<String>,
    pub participants: Vec<String>,
    pub reminders: Vec<Reminder>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub time: chrono::DateTime<chrono::Utc>,
    pub method: ReminderMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReminderMethod {
    Email,
    Notification,
    Popup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collaboration {
    pub id: String,
    pub name: String,
    pub description: String,
    pub members: Vec<String>,
    pub tasks: Vec<CollaborationTask>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationTask {
    pub id: String,
    pub title: String,
    pub description: String,
    pub assignee: String,
    pub status: TaskStatus,
    pub due_date: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
    Blocked,
}

pub struct OfficeAgent {
    base: BaseAgent,
    llm_manager: Option<Arc<LlmManager>>,
    skill_registry: Option<Arc<SkillRegistry>>,
    progressive_loader: Option<Arc<ProgressiveLoader>>,
}

impl OfficeAgent {
    pub fn new(id: Option<String>, name: Option<String>) -> Self {
        let agent_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let agent_name = name.unwrap_or_else(|| "OfficeAgent".to_string());

        let mut config = AgentConfig::new(agent_id, agent_name, AgentType::Office);
        config.capabilities.can_document = true;
        config.capabilities.can_communicate = true;
        config.capabilities.can_collaborate = true;

        Self {
            base: BaseAgent::new(config),
            llm_manager: None,
            skill_registry: None,
            progressive_loader: None,
        }
    }

    pub fn new_arc(id: Option<String>, name: Option<String>) -> Arc<dyn Agent + Send + Sync> {
        <dyn Agent + Send + Sync>::from_arc(Self::new(id, name))
    }

    pub fn with_llm_manager(mut self, llm_manager: Arc<LlmManager>) -> Self {
        self.llm_manager = Some(llm_manager);
        self
    }

    pub fn with_skill_registry(mut self, skill_registry: Arc<SkillRegistry>) -> Self {
        self.skill_registry = Some(skill_registry);
        self
    }

    pub fn with_progressive_loader(mut self, loader: Arc<ProgressiveLoader>) -> Self {
        self.progressive_loader = Some(loader);
        self
    }

    async fn generate_document(
        &self,
        title: &str,
        sections: Vec<DocumentSection>,
        keywords: Vec<String>,
    ) -> Result<Document> {
        info!("Generating document: {}", title);

        let content = sections
            .iter()
            .map(|s| format!("#{}\n{}", "#".repeat(s.level as usize), s.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        Ok(Document {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            content,
            author: self.base.config().name.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            sections,
            keywords,
        })
    }

    async fn generate_report(
        &self,
        title: &str,
        period: &str,
        metrics: Vec<ReportMetric>,
        sections: Vec<ReportSection>,
    ) -> Result<Report> {
        info!("Generating report: {}", title);

        let summary = format!("Report {} covering period {}", title, period);

        Ok(Report {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            summary,
            period: period.to_string(),
            metrics,
            sections,
            created_at: chrono::Utc::now(),
        })
    }

    async fn compose_email(
        &self,
        from: &str,
        to: Vec<String>,
        subject: &str,
        body: &str,
    ) -> Result<Email> {
        info!("Composing email: {}", subject);

        Ok(Email {
            id: uuid::Uuid::new_v4().to_string(),
            from: from.to_string(),
            to,
            cc: Vec::new(),
            subject: subject.to_string(),
            body: body.to_string(),
            attachments: Vec::new(),
            status: EmailStatus::Draft,
            sent_at: None,
            created_at: chrono::Utc::now(),
        })
    }

    async fn create_schedule(
        &self,
        title: &str,
        description: &str,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Schedule> {
        info!("Creating schedule: {}", title);

        Ok(Schedule {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            description: description.to_string(),
            start_time,
            end_time,
            location: None,
            participants: Vec::new(),
            reminders: Vec::new(),
            created_at: chrono::Utc::now(),
        })
    }

    async fn create_collaboration(
        &self,
        name: &str,
        description: &str,
        members: Vec<String>,
    ) -> Result<Collaboration> {
        info!("Creating collaboration: {}", name);

        Ok(Collaboration {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            members,
            tasks: Vec::new(),
            created_at: chrono::Utc::now(),
        })
    }

    async fn think(
        &mut self,
        task: &Task,
        iteration: usize,
    ) -> Result<crate::agent::base::ThinkResult> {
        info!("OfficeAgent thinking about task: {}", task.id);

        self.base.state_mut().status = crate::agent::base::AgentStatus::Thinking;

        let thought = format!(
            "Iteration {}: Analyzing office task '{}' with description '{}'",
            iteration + 1,
            task.id,
            task.description
        );

        let step = ReActStep::think(thought.clone());
        self.base.state_mut().add_react_step(step);

        let is_complete = iteration >= self.base.config().max_react_iterations - 1;

        Ok(crate::agent::base::ThinkResult {
            thought,
            is_complete,
            task_type: None,
        })
    }

    async fn act(&mut self, task: &Task, thought: &str) -> Result<crate::agent::base::ActResult> {
        info!("OfficeAgent acting on task: {}", task.id);

        self.base.state_mut().status = crate::agent::base::AgentStatus::Acting;

        let content = task.description.to_lowercase();
        let tags_lower: Vec<String> = task.tags.iter().map(|t| t.to_lowercase()).collect();

        let output = if content.contains("document")
            || content.contains("文档")
            || tags_lower
                .iter()
                .any(|t| t.contains("document") || t.contains("文档"))
        {
            let sections = vec![
                DocumentSection {
                    title: "简介".to_string(),
                    content: "这是文档的简介部分。".to_string(),
                    level: 1,
                },
                DocumentSection {
                    title: "主要内容".to_string(),
                    content: "这是文档的主要内容部分。".to_string(),
                    level: 1,
                },
                DocumentSection {
                    title: "结论".to_string(),
                    content: "这是文档的结论部分。".to_string(),
                    level: 1,
                },
            ];
            let doc = self
                .generate_document("新文档", sections, vec!["文档".to_string()])
                .await?;
            serde_json::to_string(&doc)?
        } else if content.contains("report")
            || content.contains("报告")
            || tags_lower
                .iter()
                .any(|t| t.contains("report") || t.contains("报告"))
        {
            let metrics = vec![ReportMetric {
                name: "收入".to_string(),
                value: 1000000.0,
                unit: "元".to_string(),
                change: Some(10.5),
            }];
            let sections = vec![ReportSection {
                title: "执行摘要".to_string(),
                content: "这是报告的执行摘要。".to_string(),
            }];
            let report = self
                .generate_report("季度报告", "2025 Q1", metrics, sections)
                .await?;
            serde_json::to_string(&report)?
        } else if content.contains("email")
            || content.contains("邮件")
            || tags_lower
                .iter()
                .any(|t| t.contains("email") || t.contains("邮件"))
        {
            let email = self
                .compose_email(
                    &self.base.config().name,
                    vec!["recipient@example.com".to_string()],
                    "新邮件",
                    "这是一封自动生成的邮件。",
                )
                .await?;
            serde_json::to_string(&email)?
        } else if content.contains("schedule")
            || content.contains("日程")
            || tags_lower
                .iter()
                .any(|t| t.contains("schedule") || t.contains("日程"))
        {
            let now = chrono::Utc::now();
            let schedule = self
                .create_schedule("会议", "团队周会", now, now + chrono::Duration::hours(1))
                .await?;
            serde_json::to_string(&schedule)?
        } else if content.contains("collaboration")
            || content.contains("协作")
            || tags_lower
                .iter()
                .any(|t| t.contains("collaboration") || t.contains("协作"))
        {
            let collab = self
                .create_collaboration(
                    "项目协作",
                    "新项目协作空间",
                    vec!["user1".to_string(), "user2".to_string()],
                )
                .await?;
            serde_json::to_string(&collab)?
        } else {
            "Office task completed successfully".to_string()
        };

        let action = format!("Executing office action based on: {}", thought);
        let step = ReActStep::act(action.clone());
        self.base.state_mut().add_react_step(step);

        Ok(crate::agent::base::ActResult {
            action,
            success: true,
            output,
        })
    }

    async fn observe(
        &mut self,
        task: &Task,
        act_result: crate::agent::base::ActResult,
    ) -> Result<()> {
        info!("OfficeAgent observing results for task: {}", task.id);

        self.base.state_mut().status = crate::agent::base::AgentStatus::Observing;

        let observation = format!(
            "Observed: Action '{}' completed with result",
            act_result.action
        );

        let step = ReActStep::observe(observation);
        self.base.state_mut().add_react_step(step);

        Ok(())
    }

    async fn should_stop(&self, _task: &Task) -> Result<bool> {
        Ok(self.base.state().react_steps.len() >= self.base.config().max_react_iterations * 3)
    }

    async fn react_loop(&mut self, task: &mut Task) -> Result<()> {
        info!("Starting OfficeAgent ReAct loop for task: {}", task.id);

        let max_iterations = self.base.config().max_react_iterations;

        for iteration in 0..max_iterations {
            debug!("ReAct iteration {}/{}", iteration + 1, max_iterations);

            let think_result = self.think(task, iteration).await?;

            if think_result.is_complete {
                info!("Task marked as complete during think phase");
                break;
            }

            let action_result = self.act(task, &think_result.thought).await?;

            self.observe(task, action_result).await?;

            if self.should_stop(task).await? {
                break;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Agent for OfficeAgent {
    fn config(&self) -> &AgentConfig {
        self.base.config()
    }

    fn state(&self) -> &AgentState {
        self.base.state()
    }

    fn state_mut(&mut self) -> &mut AgentState {
        self.base.state_mut()
    }

    async fn execute(&mut self, mut task: Task) -> Result<Task> {
        info!("OfficeAgent executing task: {}", task.id);

        self.base.state_mut().start_task(task.id.clone());

        if let Some(loader) = &self.progressive_loader {
            let _ = loader.create_context(&task, LoadingStrategy::Lazy, 3).await;
        }

        let result = self.react_loop(&mut task).await;

        match result {
            Ok(_) => {
                let content = task.description.to_lowercase();
                let tags_lower: Vec<String> = task.tags.iter().map(|t| t.to_lowercase()).collect();

                let output = if content.contains("document")
                    || content.contains("文档")
                    || tags_lower
                        .iter()
                        .any(|t| t.contains("document") || t.contains("文档"))
                {
                    let sections = vec![
                        DocumentSection {
                            title: "简介".to_string(),
                            content: "这是文档的简介部分。".to_string(),
                            level: 1,
                        },
                        DocumentSection {
                            title: "主要内容".to_string(),
                            content: "这是文档的主要内容部分。".to_string(),
                            level: 1,
                        },
                        DocumentSection {
                            title: "结论".to_string(),
                            content: "这是文档的结论部分。".to_string(),
                            level: 1,
                        },
                    ];
                    let doc = self
                        .generate_document("新文档", sections, vec!["文档".to_string()])
                        .await?;
                    serde_json::to_string(&doc)?
                } else if content.contains("report")
                    || content.contains("报告")
                    || tags_lower
                        .iter()
                        .any(|t| t.contains("report") || t.contains("报告"))
                {
                    let metrics = vec![ReportMetric {
                        name: "收入".to_string(),
                        value: 1000000.0,
                        unit: "元".to_string(),
                        change: Some(10.5),
                    }];
                    let sections = vec![ReportSection {
                        title: "执行摘要".to_string(),
                        content: "这是报告的执行摘要。".to_string(),
                    }];
                    let report = self
                        .generate_report("季度报告", "2025 Q1", metrics, sections)
                        .await?;
                    serde_json::to_string(&report)?
                } else if content.contains("email")
                    || content.contains("邮件")
                    || tags_lower
                        .iter()
                        .any(|t| t.contains("email") || t.contains("邮件"))
                {
                    let email = self
                        .compose_email(
                            &self.base.config().name,
                            vec!["recipient@example.com".to_string()],
                            "新邮件",
                            "这是一封自动生成的邮件。",
                        )
                        .await?;
                    serde_json::to_string(&email)?
                } else if content.contains("schedule")
                    || content.contains("日程")
                    || tags_lower
                        .iter()
                        .any(|t| t.contains("schedule") || t.contains("日程"))
                {
                    let now = chrono::Utc::now();
                    let schedule = self
                        .create_schedule("会议", "团队周会", now, now + chrono::Duration::hours(1))
                        .await?;
                    serde_json::to_string(&schedule)?
                } else if content.contains("collaboration")
                    || content.contains("协作")
                    || tags_lower
                        .iter()
                        .any(|t| t.contains("collaboration") || t.contains("协作"))
                {
                    let collab = self
                        .create_collaboration(
                            "项目协作",
                            "新项目协作空间",
                            vec!["user1".to_string(), "user2".to_string()],
                        )
                        .await?;
                    serde_json::to_string(&collab)?
                } else {
                    "Office task completed successfully".to_string()
                };

                task.status = crate::core::TaskStatus::Completed;
                task.result = Some(output);
                self.base.state_mut().record_success();
                info!("Task completed successfully: {}", task.id);
            }
            Err(e) => {
                task.status = crate::core::TaskStatus::Failed;
                self.base.state_mut().record_failure();
                warn!("Task failed: {} - Error: {}", task.id, e);
            }
        }

        Ok(task)
    }

    fn can_handle(&self, task: &Task) -> bool {
        task.tags.iter().any(|tag| {
            tag.to_lowercase().contains("office")
                || tag.to_lowercase().contains("document")
                || tag.to_lowercase().contains("report")
                || tag.to_lowercase().contains("email")
                || tag.to_lowercase().contains("schedule")
                || tag.to_lowercase().contains("collaboration")
                || tag.to_lowercase().contains("办公")
                || tag.to_lowercase().contains("文档")
                || tag.to_lowercase().contains("报告")
                || tag.to_lowercase().contains("邮件")
                || tag.to_lowercase().contains("日程")
                || tag.to_lowercase().contains("协作")
        })
    }

    fn is_available(&self) -> bool {
        self.base.is_available()
    }
}

impl Default for OfficeAgent {
    fn default() -> Self {
        Self::new(None, None)
    }
}
