use crate::core::llm::{ChatRequest, ChatResponse, LlmAdapter, LlmConfig, LlmProvider, TokenUsage};
use crate::utils::{AetherisError, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use tracing::{debug, error, warn};

/// Token 成本记录
///
/// 记录单次 LLM 调用的 token 使用和成本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCostRecord {
    /// 记录 ID
    pub id: String,
    /// 记录时间戳
    pub timestamp: DateTime<Utc>,
    /// LLM 提供商
    pub provider: LlmProvider,
    /// 使用的模型
    pub model: String,
    /// 任务 ID（可选）
    pub task_id: Option<String>,
    /// 用户 ID（可选）
    pub user_id: Option<String>,
    /// 提示词 token 数
    pub prompt_tokens: u32,
    /// 完成 token 数
    pub completion_tokens: u32,
    /// 总 token 数
    pub total_tokens: u32,
    /// 计算的成本
    pub cost: f64,
}

/// Token 成本模型配置
///
/// 配置特定模型的 token 单价
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCostModelConfig {
    /// 每 1k 提示词 token 的成本
    pub prompt_cost_per_1k: f64,
    /// 每 1k 完成 token 的成本
    pub completion_cost_per_1k: f64,
}

impl Default for TokenCostModelConfig {
    fn default() -> Self {
        Self {
            prompt_cost_per_1k: 0.001,
            completion_cost_per_1k: 0.002,
        }
    }
}

/// Token 预算配置
///
/// 配置全局、任务级和用户级的 token 预算
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    /// 全局预算
    pub global_budget: f64,
    /// 全局已花费
    pub global_spent: f64,
    /// 每任务预算
    pub per_task_budget: f64,
    /// 每任务已花费
    pub per_task_spent: HashMap<String, f64>,
    /// 每用户预算
    pub per_user_budget: f64,
    /// 每用户已花费
    pub per_user_spent: HashMap<String, f64>,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            global_budget: 100.0,
            global_spent: 0.0,
            per_task_budget: 10.0,
            per_task_spent: HashMap::new(),
            per_user_budget: 50.0,
            per_user_spent: HashMap::new(),
        }
    }
}

/// 预算告警级别
///
/// 表示预算告警的严重程度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BudgetAlertLevel {
    /// 警告级别（使用 70%）
    Warning,
    /// 严重级别（使用 90%）
    Critical,
    /// 超限级别（超过 100%）
    Exceeded,
}

impl fmt::Display for BudgetAlertLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BudgetAlertLevel::Warning => write!(f, "warning"),
            BudgetAlertLevel::Critical => write!(f, "critical"),
            BudgetAlertLevel::Exceeded => write!(f, "exceeded"),
        }
    }
}

/// 预算告警
///
/// 表示预算即将或已经超限的告警信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAlert {
    /// 告警级别
    pub level: BudgetAlertLevel,
    /// 告警消息
    pub message: String,
    /// 告警时间戳
    pub timestamp: DateTime<Utc>,
    /// 预算类型（global/per_task/per_user）
    pub budget_type: String,
    /// 已花费金额
    pub spent: f64,
    /// 预算金额
    pub budget: f64,
    /// 使用率（0.0-1.0+）
    pub percentage: f64,
}

/// 告警处理器 trait
///
/// 用于处理预算告警的 trait
pub trait AlertHandler: Send + Sync {
    /// 处理预算告警
    ///
    /// # Arguments
    ///
    /// * `alert` - 预算告警信息
    fn handle_alert(&self, alert: BudgetAlert);
}

/// 日志告警处理器
///
/// 将告警信息记录到日志中
pub struct LogAlertHandler;

impl AlertHandler for LogAlertHandler {
    fn handle_alert(&self, alert: BudgetAlert) {
        match alert.level {
            BudgetAlertLevel::Warning => {
                warn!("{}", alert.message);
            }
            BudgetAlertLevel::Critical => {
                error!("{}", alert.message);
            }
            BudgetAlertLevel::Exceeded => {
                error!("{}", alert.message);
            }
        }
    }
}

/// Token 成本管理器
///
/// 管理 token 使用统计、预算控制和告警
///
/// # Examples
///
/// ```
/// use aetheris::core::llm::token_cost::TokenCostManager;
///
/// let manager = TokenCostManager::new()
///     .with_global_budget(200.0)
///     .with_per_task_budget(20.0);
/// ```
pub struct TokenCostManager {
    records: DashMap<String, TokenCostRecord>,
    model_costs: DashMap<String, TokenCostModelConfig>,
    budget: parking_lot::RwLock<TokenBudget>,
    alert_handlers: Vec<Arc<dyn AlertHandler>>,
    alert_threshold_warning: f64,
    alert_threshold_critical: f64,
}

impl Default for TokenCostManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenCostManager {
    /// 创建一个新的 Token 成本管理器
    ///
    /// 预配置了常用模型（GPT-4、GPT-4 Turbo、GPT-3.5 Turbo）的成本
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::TokenCostManager;
    ///
    /// let manager = TokenCostManager::new();
    /// ```
    pub fn new() -> Self {
        let model_costs = DashMap::new();
        model_costs.insert(
            "gpt-4".to_string(),
            TokenCostModelConfig {
                prompt_cost_per_1k: 0.03,
                completion_cost_per_1k: 0.06,
            },
        );
        model_costs.insert(
            "gpt-4-turbo".to_string(),
            TokenCostModelConfig {
                prompt_cost_per_1k: 0.01,
                completion_cost_per_1k: 0.03,
            },
        );
        model_costs.insert(
            "gpt-3.5-turbo".to_string(),
            TokenCostModelConfig {
                prompt_cost_per_1k: 0.001,
                completion_cost_per_1k: 0.002,
            },
        );

        Self {
            records: DashMap::new(),
            model_costs,
            budget: parking_lot::RwLock::new(TokenBudget::default()),
            alert_handlers: vec![Arc::new(LogAlertHandler)],
            alert_threshold_warning: 0.7,
            alert_threshold_critical: 0.9,
        }
    }

    /// 设置全局预算
    ///
    /// # Arguments
    ///
    /// * `budget` - 全局预算金额
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::TokenCostManager;
    ///
    /// let manager = TokenCostManager::new()
    ///     .with_global_budget(500.0);
    /// ```
    pub fn with_global_budget(self, budget: f64) -> Self {
        self.budget.write().global_budget = budget;
        self
    }

    /// 设置每任务预算
    ///
    /// # Arguments
    ///
    /// * `budget` - 每任务预算金额
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::TokenCostManager;
    ///
    /// let manager = TokenCostManager::new()
    ///     .with_per_task_budget(50.0);
    /// ```
    pub fn with_per_task_budget(self, budget: f64) -> Self {
        self.budget.write().per_task_budget = budget;
        self
    }

    /// 设置每用户预算
    ///
    /// # Arguments
    ///
    /// * `budget` - 每用户预算金额
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::TokenCostManager;
    ///
    /// let manager = TokenCostManager::new()
    ///     .with_per_user_budget(100.0);
    /// ```
    pub fn with_per_user_budget(self, budget: f64) -> Self {
        self.budget.write().per_user_budget = budget;
        self
    }

    /// 设置告警阈值
    ///
    /// # Arguments
    ///
    /// * `warning` - 警告级别阈值（0.0-1.0）
    /// * `critical` - 严重级别阈值（0.0-1.0）
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::TokenCostManager;
    ///
    /// let manager = TokenCostManager::new()
    ///     .with_alert_thresholds(0.6, 0.85);
    /// ```
    pub fn with_alert_thresholds(mut self, warning: f64, critical: f64) -> Self {
        self.alert_threshold_warning = warning;
        self.alert_threshold_critical = critical;
        self
    }

    /// 添加告警处理器
    ///
    /// # Arguments
    ///
    /// * `handler` - 告警处理器
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::{TokenCostManager, LogAlertHandler};
    /// use std::sync::Arc;
    ///
    /// let mut manager = TokenCostManager::new();
    /// manager.add_alert_handler(Arc::new(LogAlertHandler));
    /// ```
    pub fn add_alert_handler(&mut self, handler: Arc<dyn AlertHandler>) {
        self.alert_handlers.push(handler);
    }

    /// 注册模型成本配置
    ///
    /// # Arguments
    ///
    /// * `model` - 模型名称
    /// * `config` - 成本配置
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::{TokenCostManager, TokenCostModelConfig};
    ///
    /// let manager = TokenCostManager::new();
    /// let config = TokenCostModelConfig {
    ///     prompt_cost_per_1k: 0.02,
    ///     completion_cost_per_1k: 0.04,
    /// };
    /// manager.register_model_cost("my-model".to_string(), config);
    /// ```
    pub fn register_model_cost(&self, model: String, config: TokenCostModelConfig) {
        self.model_costs.insert(model, config);
    }

    /// 获取模型成本配置
    ///
    /// # Arguments
    ///
    /// * `model` - 模型名称
    ///
    /// # Returns
    ///
    /// 返回模型的成本配置，如果未找到则返回默认配置
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::TokenCostManager;
    ///
    /// let manager = TokenCostManager::new();
    /// let config = manager.get_model_cost("gpt-4");
    /// ```
    pub fn get_model_cost(&self, model: &str) -> TokenCostModelConfig {
        self.model_costs
            .get(model)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    fn calculate_cost(&self, model: &str, usage: &TokenUsage) -> f64 {
        let config = self.get_model_cost(model);
        let prompt_cost = (usage.prompt_tokens as f64 / 1000.0) * config.prompt_cost_per_1k;
        let completion_cost =
            (usage.completion_tokens as f64 / 1000.0) * config.completion_cost_per_1k;
        prompt_cost + completion_cost
    }

    fn trigger_alert(&self, alert: BudgetAlert) {
        for handler in &self.alert_handlers {
            handler.handle_alert(alert.clone());
        }
    }

    fn check_and_trigger_alerts(&self, spent: f64, budget: f64, budget_type: &str) {
        if budget <= 0.0 {
            return;
        }
        let percentage = spent / budget;

        let level = if percentage >= 1.0 {
            BudgetAlertLevel::Exceeded
        } else if percentage >= self.alert_threshold_critical {
            BudgetAlertLevel::Critical
        } else if percentage >= self.alert_threshold_warning {
            BudgetAlertLevel::Warning
        } else {
            return;
        };

        let message = format!(
            "{} budget {}: {:.2}% used ({:.4} / {:.4})",
            budget_type,
            level,
            percentage * 100.0,
            spent,
            budget
        );

        self.trigger_alert(BudgetAlert {
            level,
            message,
            timestamp: Utc::now(),
            budget_type: budget_type.to_string(),
            spent,
            budget,
            percentage,
        });
    }

    /// 检查是否可以继续调用
    ///
    /// # Arguments
    ///
    /// * `task_id` - 任务 ID（可选）
    /// * `user_id` - 用户 ID（可选）
    ///
    /// # Returns
    ///
    /// 如果预算未超限返回 true，否则返回 false
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::TokenCostManager;
    ///
    /// let manager = TokenCostManager::new();
    /// if manager.can_proceed(Some("task-1"), Some("user-1")) {
    ///     // 继续调用
    /// }
    /// ```
    pub fn can_proceed(&self, task_id: Option<&str>, user_id: Option<&str>) -> bool {
        let budget = self.budget.read();
        if budget.global_spent >= budget.global_budget {
            return false;
        }

        if let Some(task_id) = task_id {
            if let Some(spent) = budget.per_task_spent.get(task_id) {
                if *spent >= budget.per_task_budget {
                    return false;
                }
            }
        }

        if let Some(user_id) = user_id {
            if let Some(spent) = budget.per_user_spent.get(user_id) {
                if *spent >= budget.per_user_budget {
                    return false;
                }
            }
        }

        true
    }

    /// 记录 token 使用情况
    ///
    /// # Arguments
    ///
    /// * `provider` - LLM 提供商
    /// * `model` - 模型名称
    /// * `task_id` - 任务 ID（可选）
    /// * `user_id` - 用户 ID（可选）
    /// * `usage` - Token 使用统计
    ///
    /// # Returns
    ///
    /// 返回创建的成本记录
    ///
    /// # Errors
    ///
    /// 如果记录过程出错，返回错误
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::TokenCostManager;
    /// use aetheris::core::llm::{LlmProvider, TokenUsage};
    ///
    /// let manager = TokenCostManager::new();
    /// let usage = TokenUsage {
    ///     prompt_tokens: 100,
    ///     completion_tokens: 200,
    ///     total_tokens: 300,
    /// };
    /// let record = manager.record_usage(
    ///     LlmProvider::OpenAi,
    ///     "gpt-4".to_string(),
    ///     Some("task-1".to_string()),
    ///     Some("user-1".to_string()),
    ///     usage
    /// ).unwrap();
    /// ```
    pub fn record_usage(
        &self,
        provider: LlmProvider,
        model: String,
        task_id: Option<String>,
        user_id: Option<String>,
        usage: TokenUsage,
    ) -> Result<TokenCostRecord> {
        let cost = self.calculate_cost(&model, &usage);

        let mut budget = self.budget.write();
        budget.global_spent += cost;

        if let Some(task_id) = task_id.clone() {
            let spent = budget.per_task_spent.entry(task_id).or_insert(0.0);
            *spent += cost;
        }

        if let Some(user_id) = user_id.clone() {
            let spent = budget.per_user_spent.entry(user_id).or_insert(0.0);
            *spent += cost;
        }

        let global_spent = budget.global_spent;
        let global_budget = budget.global_budget;
        let per_task_budget = budget.per_task_budget;
        let per_user_budget = budget.per_user_budget;
        drop(budget);

        self.check_and_trigger_alerts(global_spent, global_budget, "global");

        if let Some(task_id) = &task_id {
            let budget = self.budget.read();
            if let Some(spent) = budget.per_task_spent.get(task_id) {
                self.check_and_trigger_alerts(*spent, per_task_budget, "per_task");
            }
        }

        if let Some(user_id) = &user_id {
            let budget = self.budget.read();
            if let Some(spent) = budget.per_user_spent.get(user_id) {
                self.check_and_trigger_alerts(*spent, per_user_budget, "per_user");
            }
        }

        let record = TokenCostRecord {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            provider,
            model,
            task_id,
            user_id,
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
            cost,
        };

        self.records.insert(record.id.clone(), record.clone());

        debug!("Token usage recorded: {:?}", record);

        Ok(record)
    }

    /// 获取总花费
    ///
    /// # Returns
    ///
    /// 返回全局总花费
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::TokenCostManager;
    ///
    /// let manager = TokenCostManager::new();
    /// let total = manager.get_total_cost();
    /// ```
    pub fn get_total_cost(&self) -> f64 {
        self.budget.read().global_spent
    }

    /// 获取总 token 使用量
    ///
    /// # Returns
    ///
    /// 返回所有记录的总 token 数
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::TokenCostManager;
    ///
    /// let manager = TokenCostManager::new();
    /// let tokens = manager.get_total_tokens();
    /// ```
    pub fn get_total_tokens(&self) -> u64 {
        self.records.iter().map(|r| r.total_tokens as u64).sum()
    }

    /// 按模型获取统计
    ///
    /// # Arguments
    ///
    /// * `model` - 模型名称
    ///
    /// # Returns
    ///
    /// 返回 (总 token 数, 总花费) 的元组
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::TokenCostManager;
    ///
    /// let manager = TokenCostManager::new();
    /// let (tokens, cost) = manager.get_stats_by_model("gpt-4");
    /// ```
    pub fn get_stats_by_model(&self, model: &str) -> (u64, f64) {
        let mut tokens = 0u64;
        let mut cost = 0.0;

        for record in self.records.iter() {
            if record.model == model {
                tokens += record.total_tokens as u64;
                cost += record.cost;
            }
        }

        (tokens, cost)
    }

    /// 按提供商获取统计
    ///
    /// # Arguments
    ///
    /// * `provider` - LLM 提供商
    ///
    /// # Returns
    ///
    /// 返回 (总 token 数, 总花费) 的元组
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::TokenCostManager;
    /// use aetheris::core::llm::LlmProvider;
    ///
    /// let manager = TokenCostManager::new();
    /// let (tokens, cost) = manager.get_stats_by_provider(&LlmProvider::OpenAi);
    /// ```
    pub fn get_stats_by_provider(&self, provider: &LlmProvider) -> (u64, f64) {
        let mut tokens = 0u64;
        let mut cost = 0.0;

        for record in self.records.iter() {
            if record.provider == *provider {
                tokens += record.total_tokens as u64;
                cost += record.cost;
            }
        }

        (tokens, cost)
    }

    /// 按时间范围获取统计
    ///
    /// # Arguments
    ///
    /// * `start` - 开始时间
    /// * `end` - 结束时间
    ///
    /// # Returns
    ///
    /// 返回 (总 token 数, 总花费) 的元组
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::TokenCostManager;
    /// use chrono::{Utc, Duration};
    ///
    /// let manager = TokenCostManager::new();
    /// let end = Utc::now();
    /// let start = end - Duration::hours(24);
    /// let (tokens, cost) = manager.get_stats_by_time_range(start, end);
    /// ```
    pub fn get_stats_by_time_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> (u64, f64) {
        let mut tokens = 0u64;
        let mut cost = 0.0;

        for record in self.records.iter() {
            if record.timestamp >= start && record.timestamp <= end {
                tokens += record.total_tokens as u64;
                cost += record.cost;
            }
        }

        (tokens, cost)
    }

    /// 获取所有成本记录
    ///
    /// # Returns
    ///
    /// 返回所有成本记录的列表
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::TokenCostManager;
    ///
    /// let manager = TokenCostManager::new();
    /// let records = manager.get_all_records();
    /// ```
    pub fn get_all_records(&self) -> Vec<TokenCostRecord> {
        self.records.iter().map(|r| r.clone()).collect()
    }

    /// 重置全局预算
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::token_cost::TokenCostManager;
    ///
    /// let manager = TokenCostManager::new();
    /// manager.reset_global_budget();
    /// ```
    pub fn reset_global_budget(&self) {
        self.budget.write().global_spent = 0.0;
    }
}

/// 带 Token 成本控制的 LLM 适配器
///
/// 包装其他 LLM 适配器，添加 token 成本追踪和预算控制
///
/// # Examples
///
/// ```
/// use aetheris::core::llm::{MockLlmAdapter, LlmAdapter};
/// use aetheris::core::llm::token_cost::{TokenCostLlmAdapter, TokenCostManager};
/// use std::sync::Arc;
///
/// let mock = Arc::new(MockLlmAdapter::new());
/// let cost_manager = Arc::new(TokenCostManager::new());
/// let cost_adapter = TokenCostLlmAdapter::new(mock, cost_manager);
/// ```
pub struct TokenCostLlmAdapter {
    inner: Arc<dyn LlmAdapter>,
    cost_manager: Arc<TokenCostManager>,
    task_id: Option<String>,
    user_id: Option<String>,
}

impl TokenCostLlmAdapter {
    /// 创建一个新的带成本控制的适配器
    ///
    /// # Arguments
    ///
    /// * `inner` - 要包装的 LLM 适配器
    /// * `cost_manager` - Token 成本管理器
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{MockLlmAdapter, LlmAdapter};
    /// use aetheris::core::llm::token_cost::{TokenCostLlmAdapter, TokenCostManager};
    /// use std::sync::Arc;
    ///
    /// let mock = Arc::new(MockLlmAdapter::new());
    /// let cost_manager = Arc::new(TokenCostManager::new());
    /// let cost_adapter = TokenCostLlmAdapter::new(mock, cost_manager);
    /// ```
    pub fn new(inner: Arc<dyn LlmAdapter>, cost_manager: Arc<TokenCostManager>) -> Self {
        Self {
            inner,
            cost_manager,
            task_id: None,
            user_id: None,
        }
    }

    /// 设置任务 ID
    ///
    /// # Arguments
    ///
    /// * `task_id` - 任务 ID
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{MockLlmAdapter, LlmAdapter};
    /// use aetheris::core::llm::token_cost::{TokenCostLlmAdapter, TokenCostManager};
    /// use std::sync::Arc;
    ///
    /// let mock = Arc::new(MockLlmAdapter::new());
    /// let cost_manager = Arc::new(TokenCostManager::new());
    /// let cost_adapter = TokenCostLlmAdapter::new(mock, cost_manager)
    ///     .with_task_id("task-1".to_string());
    /// ```
    pub fn with_task_id(mut self, task_id: String) -> Self {
        self.task_id = Some(task_id);
        self
    }

    /// 设置用户 ID
    ///
    /// # Arguments
    ///
    /// * `user_id` - 用户 ID
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{MockLlmAdapter, LlmAdapter};
    /// use aetheris::core::llm::token_cost::{TokenCostLlmAdapter, TokenCostManager};
    /// use std::sync::Arc;
    ///
    /// let mock = Arc::new(MockLlmAdapter::new());
    /// let cost_manager = Arc::new(TokenCostManager::new());
    /// let cost_adapter = TokenCostLlmAdapter::new(mock, cost_manager)
    ///     .with_user_id("user-1".to_string());
    /// ```
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// 获取成本管理器
    ///
    /// # Returns
    ///
    /// 返回成本管理器的 Arc 引用
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheris::core::llm::{MockLlmAdapter, LlmAdapter};
    /// use aetheris::core::llm::token_cost::{TokenCostLlmAdapter, TokenCostManager};
    /// use std::sync::Arc;
    ///
    /// let mock = Arc::new(MockLlmAdapter::new());
    /// let cost_manager = Arc::new(TokenCostManager::new());
    /// let cost_adapter = TokenCostLlmAdapter::new(mock, cost_manager.clone());
    /// let manager = cost_adapter.cost_manager();
    /// ```
    pub fn cost_manager(&self) -> Arc<TokenCostManager> {
        Arc::clone(&self.cost_manager)
    }
}

#[async_trait]
impl LlmAdapter for TokenCostLlmAdapter {
    fn provider(&self) -> LlmProvider {
        self.inner.provider()
    }

    fn config(&self) -> &LlmConfig {
        self.inner.config()
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        if !self
            .cost_manager
            .can_proceed(self.task_id.as_deref(), self.user_id.as_deref())
        {
            return Err(AetherisError::TokenBudgetExceeded(
                "Token budget exceeded".to_string(),
            ));
        }

        let response = self.inner.chat(request.clone()).await?;

        if let Some(usage) = &response.usage {
            self.cost_manager.record_usage(
                self.provider(),
                request.model.clone(),
                self.task_id.clone(),
                self.user_id.clone(),
                usage.clone(),
            )?;
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::llm::adapter::ChatMessage;
    use crate::core::llm::mock::MockLlmAdapter;

    #[test]
    fn test_token_cost_manager_new() {
        let manager = TokenCostManager::new();
        let budget = manager.budget.read();
        assert_eq!(budget.global_budget, 100.0);
        assert_eq!(budget.per_task_budget, 10.0);
        assert_eq!(budget.per_user_budget, 50.0);
    }

    #[test]
    fn test_register_model_cost() {
        let manager = TokenCostManager::new();
        let config = TokenCostModelConfig {
            prompt_cost_per_1k: 0.01,
            completion_cost_per_1k: 0.02,
        };
        manager.register_model_cost("test-model".to_string(), config.clone());
        let retrieved = manager.get_model_cost("test-model");
        assert_eq!(retrieved.prompt_cost_per_1k, 0.01);
        assert_eq!(retrieved.completion_cost_per_1k, 0.02);
    }

    #[test]
    fn test_calculate_cost() {
        let manager = TokenCostManager::new();
        let usage = TokenUsage {
            prompt_tokens: 1000,
            completion_tokens: 1000,
            total_tokens: 2000,
        };
        let cost = manager.calculate_cost("gpt-4", &usage);
        assert_eq!(cost, 0.09);
    }

    #[test]
    fn test_record_usage() {
        let manager = TokenCostManager::new();
        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 200,
            total_tokens: 300,
        };
        let record = manager
            .record_usage(
                LlmProvider::OpenAi,
                "gpt-4".to_string(),
                Some("task-1".to_string()),
                Some("user-1".to_string()),
                usage,
            )
            .unwrap();
        assert!(!record.id.is_empty());
        assert_eq!(record.provider, LlmProvider::OpenAi);
        assert_eq!(record.model, "gpt-4");
        assert!(record.cost > 0.0);
    }

    #[test]
    fn test_can_proceed() {
        let manager = TokenCostManager::new();
        assert!(manager.can_proceed(None, None));
    }

    #[test]
    fn test_get_stats_by_model() {
        let manager = TokenCostManager::new();
        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 200,
            total_tokens: 300,
        };
        manager
            .record_usage(
                LlmProvider::OpenAi,
                "gpt-4".to_string(),
                None,
                None,
                usage.clone(),
            )
            .unwrap();
        manager
            .record_usage(LlmProvider::OpenAi, "gpt-4".to_string(), None, None, usage)
            .unwrap();
        let (tokens, cost) = manager.get_stats_by_model("gpt-4");
        assert_eq!(tokens, 600);
        assert!(cost > 0.0);
    }

    #[tokio::test]
    async fn test_token_cost_llm_adapter() {
        let mock_adapter = Arc::new(MockLlmAdapter::default());
        let cost_manager = Arc::new(TokenCostManager::new());
        let adapter = TokenCostLlmAdapter::new(mock_adapter, cost_manager.clone());
        let messages = vec![ChatMessage::user("Hello".to_string())];
        let request = ChatRequest::new("gpt-4".to_string(), messages);
        let response = adapter.chat(request).await.unwrap();
        assert!(!response.id.is_empty());
        assert!(cost_manager.get_total_cost() > 0.0);
    }
}
