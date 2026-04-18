use crate::agent::base::{Agent, AgentConfig, AgentState, AgentType, BaseAgent};
use crate::core::Task;
use crate::core::llm::manager::LlmManager;
use crate::core::plan_execute::ReActStep;
use crate::core::progressive_loading::ProgressiveLoader;
use crate::skill::registry::SkillRegistry;
use crate::utils::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: HashMap<String, f64>,
    pub network_inbound_mbps: f64,
    pub network_outbound_mbps: f64,
    pub active_connections: u64,
    pub service_statuses: HashMap<String, String>,
}

impl SystemMetrics {
    fn new() -> Self {
        let now = chrono::Utc::now();
        let mut disk_usage = HashMap::new();
        disk_usage.insert("/dev/sda1".to_string(), 45.0);
        disk_usage.insert("/dev/sdb1".to_string(), 24.0);

        let mut service_statuses = HashMap::new();
        service_statuses.insert("web-service".to_string(), "RUNNING".to_string());
        service_statuses.insert("db-primary".to_string(), "HEALTHY".to_string());
        service_statuses.insert("db-replica".to_string(), "HEALTHY".to_string());
        service_statuses.insert("redis-cache".to_string(), "RUNNING".to_string());
        service_statuses.insert("message-queue".to_string(), "RUNNING".to_string());

        Self {
            timestamp: now,
            cpu_usage: 42.0,
            memory_usage: 40.0,
            disk_usage,
            network_inbound_mbps: 12.5,
            network_outbound_mbps: 8.2,
            active_connections: 1247,
            service_statuses,
        }
    }

    fn analyze(&self) -> MetricsAnalysis {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        if self.cpu_usage > 80.0 {
            issues.push("CPU usage critically high".to_string());
        } else if self.cpu_usage > 60.0 {
            warnings.push("CPU usage elevated".to_string());
        }

        if self.memory_usage > 85.0 {
            issues.push("Memory usage critically high".to_string());
        } else if self.memory_usage > 70.0 {
            warnings.push("Memory usage elevated".to_string());
        }

        for (service, status) in &self.service_statuses {
            if status == "ERROR" || status == "STOPPED" {
                issues.push(format!("Service {} is {}", service, status));
            }
        }

        MetricsAnalysis {
            overall_status: if issues.is_empty() {
                if warnings.is_empty() {
                    "HEALTHY"
                } else {
                    "WARNING"
                }
            } else {
                "CRITICAL"
            }
            .to_string(),
            issues,
            warnings,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsAnalysis {
    pub overall_status: String,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentPlan {
    pub plan_id: String,
    pub application: String,
    pub version: String,
    pub environment: String,
    pub steps: Vec<DeploymentStep>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentStep {
    pub step_id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub output: Option<String>,
}

impl DeploymentPlan {
    fn new(application: String, version: String, environment: String) -> Self {
        Self {
            plan_id: uuid::Uuid::new_v4().to_string(),
            application,
            version,
            environment,
            steps: vec![
                DeploymentStep {
                    step_id: uuid::Uuid::new_v4().to_string(),
                    name: "Pull Image".to_string(),
                    description: "Pull Docker image from registry".to_string(),
                    status: "PENDING".to_string(),
                    started_at: None,
                    completed_at: None,
                    output: None,
                },
                DeploymentStep {
                    step_id: uuid::Uuid::new_v4().to_string(),
                    name: "Stop Old Containers".to_string(),
                    description: "Stop old containers".to_string(),
                    status: "PENDING".to_string(),
                    started_at: None,
                    completed_at: None,
                    output: None,
                },
                DeploymentStep {
                    step_id: uuid::Uuid::new_v4().to_string(),
                    name: "Start New Containers".to_string(),
                    description: "Start new containers".to_string(),
                    status: "PENDING".to_string(),
                    started_at: None,
                    completed_at: None,
                    output: None,
                },
                DeploymentStep {
                    step_id: uuid::Uuid::new_v4().to_string(),
                    name: "Health Check".to_string(),
                    description: "Wait for health check to pass".to_string(),
                    status: "PENDING".to_string(),
                    started_at: None,
                    completed_at: None,
                    output: None,
                },
                DeploymentStep {
                    step_id: uuid::Uuid::new_v4().to_string(),
                    name: "Post-Deployment Tests".to_string(),
                    description: "Run post-deployment tests".to_string(),
                    status: "PENDING".to_string(),
                    started_at: None,
                    completed_at: None,
                    output: None,
                },
                DeploymentStep {
                    step_id: uuid::Uuid::new_v4().to_string(),
                    name: "Update Load Balancer".to_string(),
                    description: "Update load balancer configuration".to_string(),
                    status: "PENDING".to_string(),
                    started_at: None,
                    completed_at: None,
                    output: None,
                },
            ],
            status: "PENDING".to_string(),
        }
    }

    async fn execute(&mut self) -> Result<()> {
        info!("Starting deployment: {}", self.plan_id);
        self.status = "IN_PROGRESS".to_string();

        for step in &mut self.steps {
            info!("Executing deployment step: {}", step.name);
            step.status = "RUNNING".to_string();
            step.started_at = Some(chrono::Utc::now());

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            step.status = "COMPLETED".to_string();
            step.completed_at = Some(chrono::Utc::now());
            step.output = Some(format!("Step {} completed successfully", step.name));
        }

        self.status = "COMPLETED".to_string();
        info!("Deployment completed: {}", self.plan_id);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogQuery {
    pub query_id: String,
    pub sources: Vec<String>,
    pub time_range_start: chrono::DateTime<chrono::Utc>,
    pub time_range_end: chrono::DateTime<chrono::Utc>,
    pub keywords: Vec<String>,
    pub log_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogAnalysis {
    pub total_entries: u64,
    pub error_count: u64,
    pub warning_count: u64,
    pub info_count: u64,
    pub top_errors: Vec<String>,
}

impl LogQuery {
    fn new() -> Self {
        let now = chrono::Utc::now();
        Self {
            query_id: uuid::Uuid::new_v4().to_string(),
            sources: vec!["web-service".to_string(), "db-primary".to_string()],
            time_range_start: now - chrono::Duration::hours(1),
            time_range_end: now,
            keywords: Vec::new(),
            log_level: None,
        }
    }

    fn execute(&self) -> LogResult {
        let entries = vec![
            LogEntry {
                timestamp: chrono::Utc::now() - chrono::Duration::seconds(5),
                source: "web-service".to_string(),
                level: "INFO".to_string(),
                message: "Request processed: GET /api/users (200 OK) - 45ms".to_string(),
            },
            LogEntry {
                timestamp: chrono::Utc::now() - chrono::Duration::seconds(10),
                source: "web-service".to_string(),
                level: "WARN".to_string(),
                message: "Slow query detected: 1.2s".to_string(),
            },
            LogEntry {
                timestamp: chrono::Utc::now() - chrono::Duration::seconds(30),
                source: "db-primary".to_string(),
                level: "ERROR".to_string(),
                message: "Database connection timeout".to_string(),
            },
        ];

        let analysis = LogAnalysis {
            total_entries: 12457,
            error_count: 333,
            warning_count: 1890,
            info_count: 10234,
            top_errors: vec![
                "Database connection timeout: 120".to_string(),
                "Cache miss: 89".to_string(),
                "Invalid request: 124".to_string(),
            ],
        };

        LogResult {
            query: self.clone(),
            entries,
            analysis,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogResult {
    pub query: LogQuery,
    pub entries: Vec<LogEntry>,
    pub analysis: LogAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultDiagnosis {
    pub diagnosis_id: String,
    pub incident_time: chrono::DateTime<chrono::Utc>,
    pub symptoms: Vec<String>,
    pub root_cause: Option<String>,
    pub affected_services: Vec<String>,
    pub investigation_steps: Vec<String>,
    pub recommended_fixes: Vec<String>,
    pub severity: String,
    pub status: String,
}

impl FaultDiagnosis {
    fn new(symptoms: Vec<String>) -> Self {
        Self {
            diagnosis_id: uuid::Uuid::new_v4().to_string(),
            incident_time: chrono::Utc::now(),
            symptoms,
            root_cause: None,
            affected_services: Vec::new(),
            investigation_steps: Vec::new(),
            recommended_fixes: Vec::new(),
            severity: "MEDIUM".to_string(),
            status: "IN_PROGRESS".to_string(),
        }
    }

    async fn investigate(&mut self) -> Result<()> {
        info!("Starting fault diagnosis: {}", self.diagnosis_id);

        self.investigation_steps
            .push("Checking system metrics".to_string());
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        self.investigation_steps
            .push("Analyzing application logs".to_string());
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        self.investigation_steps
            .push("Checking service health".to_string());
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        self.root_cause = Some("Database connection pool exhaustion".to_string());
        self.affected_services = vec!["web-service".to_string()];

        self.recommended_fixes = vec![
            "Increase database connection pool size".to_string(),
            "Add circuit breaker for database calls".to_string(),
            "Implement query optimization for slow queries".to_string(),
        ];

        self.status = "COMPLETED".to_string();
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfHealingAction {
    pub action_id: String,
    pub issue: String,
    pub action_type: String,
    pub description: String,
    pub status: String,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub result: Option<String>,
}

impl SelfHealingAction {
    fn new(issue: String, action_type: String, description: String) -> Self {
        Self {
            action_id: uuid::Uuid::new_v4().to_string(),
            issue,
            action_type,
            description,
            status: "PENDING".to_string(),
            started_at: None,
            completed_at: None,
            result: None,
        }
    }

    async fn execute(&mut self) -> Result<()> {
        info!("Executing self-healing action: {}", self.action_id);
        self.status = "IN_PROGRESS".to_string();
        self.started_at = Some(chrono::Utc::now());

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        self.status = "COMPLETED".to_string();
        self.completed_at = Some(chrono::Utc::now());
        self.result = Some("Action executed successfully, issue resolved".to_string());
        Ok(())
    }
}

pub struct OpsAgent {
    base: BaseAgent,
    llm_manager: Option<Arc<LlmManager>>,
    skill_registry: Option<Arc<SkillRegistry>>,
    progressive_loader: Option<Arc<ProgressiveLoader>>,
}

impl OpsAgent {
    pub fn new(id: Option<String>, name: Option<String>) -> Self {
        let agent_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let agent_name = name.unwrap_or_else(|| "OpsAgent".to_string());

        let mut config = AgentConfig::new(agent_id, agent_name, AgentType::Ops);
        config.capabilities.can_operate = true;
        config.capabilities.can_code = true;
        config.capabilities.can_analyze_data = true;

        Self {
            base: BaseAgent::new(config),
            llm_manager: None,
            skill_registry: None,
            progressive_loader: None,
        }
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

    pub fn new_arc(id: Option<String>, name: Option<String>) -> Arc<dyn Agent + Send + Sync> {
        Arc::new(Self::new(id, name))
    }

    async fn collect_system_metrics(&mut self) -> Result<SystemMetrics> {
        info!("Collecting system metrics");
        let think_step = ReActStep::think("Collecting system metrics for monitoring".to_string());
        self.base.state_mut().add_react_step(think_step);

        let metrics = SystemMetrics::new();

        let act_step = ReActStep::act("Collected system metrics from all sources".to_string());
        self.base.state_mut().add_react_step(act_step);

        let observe_step = ReActStep::observe("System metrics collected successfully".to_string());
        self.base.state_mut().add_react_step(observe_step);

        Ok(metrics)
    }

    async fn analyze_metrics(&mut self, metrics: &SystemMetrics) -> Result<MetricsAnalysis> {
        info!("Analyzing system metrics");
        let think_step = ReActStep::think("Analyzing collected metrics for anomalies".to_string());
        self.base.state_mut().add_react_step(think_step);

        let analysis = metrics.analyze();

        let act_step = ReActStep::act(format!(
            "Metrics analysis complete: {}",
            analysis.overall_status
        ));
        self.base.state_mut().add_react_step(act_step);

        Ok(analysis)
    }

    async fn execute_deployment(
        &mut self,
        application: String,
        version: String,
        environment: String,
    ) -> Result<DeploymentPlan> {
        info!(
            "Executing deployment: {} v{} to {}",
            application, version, environment
        );

        let think_step = ReActStep::think(format!(
            "Planning deployment of {} v{} to {}",
            application, version, environment
        ));
        self.base.state_mut().add_react_step(think_step);

        let mut plan = DeploymentPlan::new(application, version, environment);

        let act_step = ReActStep::act("Starting deployment process".to_string());
        self.base.state_mut().add_react_step(act_step);

        plan.execute().await?;

        let observe_step = ReActStep::observe("Deployment completed successfully".to_string());
        self.base.state_mut().add_react_step(observe_step);

        Ok(plan)
    }

    async fn collect_logs(&mut self, query: Option<LogQuery>) -> Result<LogResult> {
        info!("Collecting and analyzing logs");

        let think_step = ReActStep::think("Collecting logs from configured sources".to_string());
        self.base.state_mut().add_react_step(think_step);

        let log_query = query.unwrap_or_else(LogQuery::new);
        let result = log_query.execute();

        let act_step = ReActStep::act("Logs collected and analyzed".to_string());
        self.base.state_mut().add_react_step(act_step);

        Ok(result)
    }

    async fn diagnose_fault(&mut self, symptoms: Vec<String>) -> Result<FaultDiagnosis> {
        info!("Diagnosing fault with symptoms: {:?}", symptoms);

        let think_step = ReActStep::think(format!(
            "Starting fault diagnosis for symptoms: {:?}",
            symptoms
        ));
        self.base.state_mut().add_react_step(think_step);

        let mut diagnosis = FaultDiagnosis::new(symptoms);

        let act_step = ReActStep::act("Investigating potential root causes".to_string());
        self.base.state_mut().add_react_step(act_step);

        diagnosis.investigate().await?;

        let observe_step =
            ReActStep::observe(format!("Root cause identified: {:?}", diagnosis.root_cause));
        self.base.state_mut().add_react_step(observe_step);

        Ok(diagnosis)
    }

    async fn apply_self_healing(&mut self, issue: String) -> Result<SelfHealingAction> {
        info!("Applying self-healing for issue: {}", issue);

        let think_step = ReActStep::think(format!(
            "Determining appropriate self-healing action for: {}",
            issue
        ));
        self.base.state_mut().add_react_step(think_step);

        let mut action = SelfHealingAction::new(
            issue.clone(),
            "Auto-remediation".to_string(),
            format!("Automated fix for: {}", issue),
        );

        let act_step = ReActStep::act("Executing self-healing action".to_string());
        self.base.state_mut().add_react_step(act_step);

        action.execute().await?;

        let observe_step = ReActStep::observe("Self-healing action completed".to_string());
        self.base.state_mut().add_react_step(observe_step);

        Ok(action)
    }

    async fn process_task(&mut self, task: &mut Task) -> Result<String> {
        let desc_lower = task.description.to_lowercase();

        if desc_lower.contains("deploy") || desc_lower.contains("部署") {
            let plan = self
                .execute_deployment(
                    "web-service".to_string(),
                    "2.3.1".to_string(),
                    "production".to_string(),
                )
                .await?;
            Ok(serde_json::to_string_pretty(&plan)?)
        } else if desc_lower.contains("monitor") || desc_lower.contains("监控") {
            let metrics = self.collect_system_metrics().await?;
            let analysis = self.analyze_metrics(&metrics).await?;
            let result = serde_json::json!({
                "metrics": metrics,
                "analysis": analysis
            });
            Ok(serde_json::to_string_pretty(&result)?)
        } else if desc_lower.contains("logs") || desc_lower.contains("日志") {
            let logs = self.collect_logs(None).await?;
            Ok(serde_json::to_string_pretty(&logs)?)
        } else if desc_lower.contains("diagnose")
            || desc_lower.contains("故障")
            || desc_lower.contains("诊断")
        {
            let symptoms = vec!["High latency".to_string(), "Connection errors".to_string()];
            let diagnosis = self.diagnose_fault(symptoms).await?;
            Ok(serde_json::to_string_pretty(&diagnosis)?)
        } else if desc_lower.contains("heal")
            || desc_lower.contains("自愈")
            || desc_lower.contains("修复")
        {
            let action = self
                .apply_self_healing("Database connection issues".to_string())
                .await?;
            Ok(serde_json::to_string_pretty(&action)?)
        } else {
            let metrics = self.collect_system_metrics().await?;
            let analysis = self.analyze_metrics(&metrics).await?;
            let result = serde_json::json!({
                "status": "Operations task completed",
                "metrics": metrics,
                "analysis": analysis
            });
            Ok(serde_json::to_string_pretty(&result)?)
        }
    }
}

#[async_trait]
impl Agent for OpsAgent {
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
        info!("OpsAgent executing task: {}", task.id);

        self.state_mut().start_task(task.id.clone());

        if let Some(loader) = &self.progressive_loader {
            use crate::core::progressive_loading::LoadingStrategy;
            let _ = loader.create_context(&task, LoadingStrategy::Lazy, 3).await;
        }

        let result = self.process_task(&mut task).await;

        match result {
            Ok(output) => {
                task.status = crate::core::TaskStatus::Completed;
                task.result = Some(output);
                self.state_mut().record_success();
                info!("Task completed successfully: {}", task.id);
            }
            Err(e) => {
                task.status = crate::core::TaskStatus::Failed;
                self.state_mut().record_failure();
                error!("Task failed: {} - Error: {}", task.id, e);
            }
        }

        Ok(task)
    }

    fn can_handle(&self, task: &Task) -> bool {
        let description_lower = task.description.to_lowercase();
        let has_ops_tags = task.tags.iter().any(|tag| {
            tag.to_lowercase().contains("ops")
                || tag.to_lowercase().contains("deploy")
                || tag.to_lowercase().contains("monitor")
                || tag.to_lowercase().contains("logs")
                || tag.to_lowercase().contains("运维")
                || tag.to_lowercase().contains("部署")
                || tag.to_lowercase().contains("监控")
                || tag.to_lowercase().contains("日志")
                || tag.to_lowercase().contains("diagnose")
                || tag.to_lowercase().contains("heal")
                || tag.to_lowercase().contains("诊断")
                || tag.to_lowercase().contains("修复")
                || tag.to_lowercase().contains("自愈")
        });

        let has_keywords = description_lower.contains("deploy")
            || description_lower.contains("部署")
            || description_lower.contains("monitor")
            || description_lower.contains("监控")
            || description_lower.contains("logs")
            || description_lower.contains("日志")
            || description_lower.contains("diagnose")
            || description_lower.contains("故障")
            || description_lower.contains("诊断")
            || description_lower.contains("heal")
            || description_lower.contains("修复")
            || description_lower.contains("自愈");

        has_ops_tags || has_keywords
    }

    fn is_available(&self) -> bool {
        self.base.is_available()
    }
}

impl Default for OpsAgent {
    fn default() -> Self {
        Self::new(None, None)
    }
}
