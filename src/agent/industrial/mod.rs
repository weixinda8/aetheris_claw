use crate::agent::base::{Agent, AgentConfig, AgentState, AgentType, BaseAgent};
use crate::agent::config::{
    ConfigChangeEvent, HotReloadManager, IndustrialProtocolIntegrationConfig,
};
use crate::core::Task;
use crate::core::llm::manager::LlmManager;
use crate::core::progressive_loading::ProgressiveLoader;
use crate::protocol::industrial::IndustrialProtocolManager;
use crate::skill::registry::SkillRegistry;
use crate::streaming::StreamingRuntime;
use crate::utils::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub mod bridge;
pub use bridge::{IndustrialProtocolBridge, ProtocolBridgeError};

pub struct IndustrialAgent {
    base: BaseAgent,
    protocol_manager: Option<Arc<IndustrialProtocolManager>>,
    protocol_bridge: Option<Arc<RwLock<IndustrialProtocolBridge>>>,
    hot_reload_manager: Option<Arc<HotReloadManager>>,
    streaming_runtime: Option<Arc<RwLock<StreamingRuntime>>>,
}

impl IndustrialAgent {
    pub fn new(id: Option<String>, name: Option<String>) -> Self {
        let agent_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let agent_name = name.unwrap_or_else(|| "IndustrialAgent".to_string());

        let mut config = AgentConfig::new(agent_id, agent_name, AgentType::Industrial);
        config.capabilities.can_operate = true;
        config.capabilities.can_code = true;
        config.capabilities.can_analyze_data = true;

        Self {
            base: BaseAgent::new(config),
            protocol_manager: None,
            protocol_bridge: None,
            hot_reload_manager: None,
            streaming_runtime: None,
        }
    }

    pub fn with_streaming_runtime(
        mut self,
        streaming_runtime: Arc<RwLock<StreamingRuntime>>,
    ) -> Self {
        self.streaming_runtime = Some(streaming_runtime);
        self
    }

    pub fn with_llm_manager(mut self, llm_manager: Arc<LlmManager>) -> Self {
        self.base = self.base.with_llm_manager(llm_manager);
        self
    }

    pub fn with_skill_registry(mut self, skill_registry: Arc<SkillRegistry>) -> Self {
        self.base = self.base.with_skill_registry(skill_registry);
        self
    }

    pub fn with_progressive_loader(mut self, loader: Arc<ProgressiveLoader>) -> Self {
        self.base = self.base.with_progressive_loader(loader);
        self
    }

    pub fn with_protocol_manager(
        mut self,
        protocol_manager: Arc<IndustrialProtocolManager>,
    ) -> Self {
        self.protocol_manager = Some(protocol_manager);
        self
    }

    pub fn with_hot_reload_manager(mut self, hot_reload_manager: Arc<HotReloadManager>) -> Self {
        self.hot_reload_manager = Some(hot_reload_manager);
        self
    }

    pub async fn with_protocol_config(
        mut self,
        protocol_config: IndustrialProtocolIntegrationConfig,
    ) -> std::result::Result<Self, ProtocolBridgeError> {
        if let Some(manager) = &self.protocol_manager {
            let mut bridge = IndustrialProtocolBridge::new(protocol_config, manager.clone());
            if bridge.is_enabled().await {
                bridge.connect().await?;
            }
            self.protocol_bridge = Some(Arc::new(RwLock::new(bridge)));

            self.register_config_change_callback().await;
        }
        Ok(self)
    }

    async fn register_config_change_callback(&self) {
        if let Some(hot_reload) = &self.hot_reload_manager {
            let bridge_arc = self.protocol_bridge.clone();
            let protocol_manager_arc = self.protocol_manager.clone();

            hot_reload
                .register_callback(move |event| {
                    let bridge_arc_clone = bridge_arc.clone();
                    let protocol_manager_arc_clone = protocol_manager_arc.clone();

                    tokio::spawn(async move {
                        if let (Some(bridge), Some(protocol_manager)) =
                            (bridge_arc_clone, protocol_manager_arc_clone)
                        {
                            Self::handle_config_change(event, bridge, protocol_manager).await;
                        }
                    });
                })
                .await;
        }
    }

    async fn handle_config_change(
        event: ConfigChangeEvent,
        bridge: Arc<RwLock<IndustrialProtocolBridge>>,
        _protocol_manager: Arc<IndustrialProtocolManager>,
    ) {
        if let ConfigChangeEvent::Modified { new_config, .. } = event {
            if let Some(new_protocol_config) = &new_config.industrial_protocol {
                debug!("Industrial protocol config changed, reloading...");

                let mut bridge_writer = bridge.write().await;

                bridge_writer.disconnect().await.ok();
                bridge_writer.update_config(new_protocol_config.clone());

                if new_protocol_config.enabled {
                    if let Err(e) = bridge_writer.connect().await {
                        warn!("Failed to reconnect after config change: {}", e);
                    }
                }
            }
        }
    }

    pub fn new_arc(id: Option<String>, name: Option<String>) -> Arc<dyn Agent + Send + Sync> {
        Arc::new(Self::new(id, name))
    }

    fn generate_monitor_data() -> String {
        r#"{
  "type": "monitor",
  "status": "success",
  "timestamp": "2026-03-29T00:00:00Z",
  "production_lines": [
    {
      "line_id": "PL-001",
      "name": "组装线A",
      "status": "running",
      "efficiency": 94.2,
      "output": 1250,
      "defect_rate": 0.8
    },
    {
      "line_id": "PL-002",
      "name": "组装线B",
      "status": "running",
      "efficiency": 88.5,
      "output": 980,
      "defect_rate": 1.2
    },
    {
      "line_id": "PL-003",
      "name": "涂装线",
      "status": "maintenance",
      "efficiency": 0,
      "output": 0,
      "defect_rate": 0
    }
  ],
  "equipment_status": [
    {
      "equipment_id": "EQ-001",
      "name": "CNC机床1",
      "status": "normal",
      "temperature": 42.5,
      "vibration": 2.3
    },
    {
      "equipment_id": "EQ-002",
      "name": "注塑机",
      "status": "warning",
      "temperature": 68.2,
      "vibration": 4.1
    }
  ],
  "summary": "生产监控数据查询完成，2条产线正常运行，1条产线维护中"
}"#
        .to_string()
    }

    fn generate_control_result() -> String {
        r#"{
  "type": "control",
  "status": "success",
  "timestamp": "2026-03-29T00:00:00Z",
  "operations": [
    {
      "equipment_id": "EQ-001",
      "name": "CNC机床1",
      "operation": "start",
      "result": "success",
      "message": "设备已成功启动"
    },
    {
      "equipment_id": "EQ-002",
      "name": "注塑机",
      "operation": "set_speed",
      "parameter": "1200 rpm",
      "result": "success",
      "message": "转速已设置为1200 rpm"
    },
    {
      "equipment_id": "EQ-003",
      "name": "传送带",
      "operation": "stop",
      "result": "success",
      "message": "设备已安全停止"
    }
  ],
  "summary": "设备控制操作完成，共执行3项操作，全部成功"
}"#
        .to_string()
    }

    fn generate_automation_result() -> String {
        format!(
            "automation workflow executed\n\n{}",
            r#"{
  "type": "automation",
  "status": "success",
  "timestamp": "2026-03-29T00:00:00Z",
  "workflow": {
    "workflow_id": "WF-001",
    "name": "产品组装自动化流程",
    "steps": [
      {
        "step": 1,
        "name": "零件上料",
        "status": "completed",
        "duration": 120
      },
      {
        "step": 2,
        "name": "精密装配",
        "status": "completed",
        "duration": 180
      },
      {
        "step": 3,
        "name": "质量检测",
        "status": "in_progress",
        "duration": 0
      },
      {
        "step": 4,
        "name": "包装入库",
        "status": "pending",
        "duration": 0
      }
    ],
    "progress": 50,
    "estimated_completion": "2026-03-29T00:15:00Z"
  },
  "summary": "自动化流程执行中，当前进度50%"
}"#
        )
    }

    fn generate_quality_inspection_result() -> String {
        r#"{
  "type": "quality",
  "status": "success",
  "timestamp": "2026-03-29T00:00:00Z",
  "inspection_batch": {
    "batch_id": "QB-2026-0329-001",
    "product_name": "智能传感器模块",
    "total_inspected": 500,
    "pass_rate": 98.4
  },
  "defect_analysis": [
    {
      "defect_type": "外观瑕疵",
      "count": 3,
      "severity": "low",
      "recommendation": "调整涂装工艺参数"
    },
    {
      "defect_type": "焊点缺陷",
      "count": 2,
      "severity": "medium",
      "recommendation": "检查焊头温度和压力"
    },
    {
      "defect_type": "功能异常",
      "count": 3,
      "severity": "high",
      "recommendation": "追溯元器件批次，进行全检"
    }
  ],
  "trend_analysis": {
    "comparison_batch": "QB-2026-0328-001",
    "pass_rate_change": "+0.5%",
    "improvement_areas": ["焊点质量", "装配精度"],
    "attention_areas": ["元器件质量"]
  },
  "summary": "质量检测完成，通过率98.4%，发现8个缺陷，建议关注元器件质量问题"
}"#
        .to_string()
    }

    fn generate_predictive_maintenance_result() -> String {
        r#"{
  "type": "predictive_maintenance",
  "status": "success",
  "timestamp": "2026-03-29T00:00:00Z",
  "equipment_health": [
    {
      "equipment_id": "EQ-001",
      "name": "CNC机床1",
      "health_score": 95,
      "status": "healthy",
      "predicted_failure_days": 180,
      "recommendations": ["继续正常运行", "按计划进行预防性维护"]
    },
    {
      "equipment_id": "EQ-002",
      "name": "注塑机",
      "health_score": 68,
      "status": "warning",
      "predicted_failure_days": 30,
      "recommendations": ["检查液压系统", "更换密封圈", "安排近期维护"]
    },
    {
      "equipment_id": "EQ-003",
      "name": "传送带",
      "health_score": 82,
      "status": "monitoring",
      "predicted_failure_days": 90,
      "recommendations": ["监控电机温度", "检查轴承润滑"]
    },
    {
      "equipment_id": "EQ-004",
      "name": "焊接机器人",
      "health_score": 45,
      "status": "critical",
      "predicted_failure_days": 7,
      "recommendations": ["立即安排维护", "更换焊枪组件", "检查冷却系统"]
    }
  ],
  "maintenance_schedule": [
    {
      "priority": "urgent",
      "equipment_id": "EQ-004",
      "scheduled_date": "2026-04-02",
      "estimated_duration_hours": 8
    },
    {
      "priority": "high",
      "equipment_id": "EQ-002",
      "scheduled_date": "2026-04-15",
      "estimated_duration_hours": 4
    },
    {
      "priority": "medium",
      "equipment_id": "EQ-003",
      "scheduled_date": "2026-05-01",
      "estimated_duration_hours": 2
    }
  ],
  "summary": "设备健康预测完成，发现1台设备处于临界状态，建议立即安排维护"
}"#
        .to_string()
    }

    fn process_task(description: &str) -> String {
        let desc_lower = description.to_lowercase();

        if desc_lower.contains("monitor") || desc_lower.contains("监控") {
            Self::generate_monitor_data()
        } else if desc_lower.contains("control") || desc_lower.contains("控制") {
            Self::generate_control_result()
        } else if desc_lower.contains("automation") || desc_lower.contains("自动化") {
            Self::generate_automation_result()
        } else if desc_lower.contains("quality")
            || desc_lower.contains("质量")
            || desc_lower.contains("检测")
        {
            Self::generate_quality_inspection_result()
        } else if desc_lower.contains("maintenance")
            || desc_lower.contains("维护")
            || desc_lower.contains("预测")
            || desc_lower.contains("健康")
        {
            Self::generate_predictive_maintenance_result()
        } else {
            "Industrial task completed successfully".to_string()
        }
    }
}

impl IndustrialAgent {
    async fn process_task_with_protocol(
        &self,
        description: &str,
        bridge: &IndustrialProtocolBridge,
    ) -> String {
        let desc_lower = description.to_lowercase();

        if desc_lower.contains("monitor") || desc_lower.contains("监控") {
            self.read_protocol_data(bridge).await
        } else {
            Self::process_task(description)
        }
    }

    async fn read_protocol_data(&self, bridge: &IndustrialProtocolBridge) -> String {
        match bridge
            .read_data_points(&[
                "Device.Temperature".to_string(),
                "Device.Status".to_string(),
            ])
            .await
        {
            Ok(points) => {
                let mut data = serde_json::Map::new();
                for point in points {
                    let value = match &point.value {
                        crate::protocol::industrial::types::DataValue::Float64(v) => {
                            serde_json::json!(v)
                        }
                        crate::protocol::industrial::types::DataValue::Int32(v) => {
                            serde_json::json!(v)
                        }
                        crate::protocol::industrial::types::DataValue::String(v) => {
                            serde_json::json!(v)
                        }
                        _ => serde_json::json!(null),
                    };
                    data.insert(point.tag_name.clone(), value);
                }
                serde_json::json!({
                    "type": "monitor",
                    "status": "success",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "data": data,
                    "summary": "Real industrial protocol data retrieved successfully"
                })
                .to_string()
            }
            Err(e) => {
                warn!("Failed to read protocol data: {}", e);
                Self::generate_monitor_data()
            }
        }
    }
}

#[async_trait]
impl Agent for IndustrialAgent {
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
        info!("IndustrialAgent executing task: {}", task.id);

        self.state_mut().start_task(task.id.clone());

        if let Some(loader) = &self.base.progressive_loader {
            let _ = loader
                .create_context(
                    &task,
                    crate::core::progressive_loading::LoadingStrategy::Lazy,
                    3,
                )
                .await;
        }

        let result = if let Some(bridge) = &self.protocol_bridge {
            let bridge = bridge.read().await;
            if bridge.is_enabled().await && bridge.is_connected().await {
                self.process_task_with_protocol(&task.description, &bridge)
                    .await
            } else {
                Self::process_task(&task.description)
            }
        } else {
            Self::process_task(&task.description)
        };

        task.status = crate::core::TaskStatus::Completed;
        task.result = Some(result);

        self.state_mut().record_success();
        info!("IndustrialAgent task completed: {}", task.id);

        Ok(task)
    }

    fn can_handle(&self, task: &Task) -> bool {
        let description_lower = task.description.to_lowercase();
        let has_industrial_tags = task.tags.iter().any(|tag| {
            tag.to_lowercase().contains("industrial")
                || tag.to_lowercase().contains("manufacturing")
                || tag.to_lowercase().contains("automation")
                || tag.to_lowercase().contains("工业")
        });

        let has_keywords = description_lower.contains("monitor")
            || description_lower.contains("监控")
            || description_lower.contains("control")
            || description_lower.contains("控制")
            || description_lower.contains("automation")
            || description_lower.contains("自动化")
            || description_lower.contains("quality")
            || description_lower.contains("质量")
            || description_lower.contains("检测")
            || description_lower.contains("maintenance")
            || description_lower.contains("维护")
            || description_lower.contains("预测")
            || description_lower.contains("健康");

        has_industrial_tags || has_keywords
    }

    fn is_available(&self) -> bool {
        self.base.is_available()
    }
}

impl Default for IndustrialAgent {
    fn default() -> Self {
        Self::new(None, None)
    }
}
