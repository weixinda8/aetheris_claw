pub mod audit;
pub mod compliance;
pub mod human_intervene;
pub mod rate_limit;
pub mod rule_block;
pub mod sandbox;

pub use sandbox::*;
pub use audit::{AuditEvent, AuditEventType, AuditLog};

use crate::core::Task;
use crate::utils::Result;
use compliance::{ComplianceEngine, ComplianceStandard};
use human_intervene::{HumanInterventionManager, InterventionRequest};
use rule_block::{RuleEngine, SecurityRule};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecurityLayer {
    RuleBlocking,
    SandboxIsolation,
    ThreeLayerQualityCheck,
    AuditSigning,
    IndustryCompliance,
    HumanIntervention,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityValidationResult {
    pub task_id: String,
    pub passed: bool,
    pub layer_results: Vec<(SecurityLayer, bool)>,
    pub violations: Vec<String>,
    pub warnings: Vec<String>,
    pub validated_at: chrono::DateTime<chrono::Utc>,
}

impl SecurityValidationResult {
    pub fn new(task_id: String) -> Self {
        Self {
            task_id,
            passed: true,
            layer_results: Vec::new(),
            violations: Vec::new(),
            warnings: Vec::new(),
            validated_at: chrono::Utc::now(),
        }
    }

    pub fn add_layer_result(&mut self, layer: SecurityLayer, passed: bool) {
        self.layer_results.push((layer, passed));
        if !passed {
            self.passed = false;
        }
    }

    pub fn add_violation(&mut self, violation: String) {
        self.violations.push(violation);
        self.passed = false;
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

pub struct SecurityManager {
    rule_engine: tokio::sync::RwLock<RuleEngine>,
    audit_log: tokio::sync::RwLock<AuditLog>,
    compliance_engine: tokio::sync::RwLock<ComplianceEngine>,
    intervention_manager: tokio::sync::RwLock<HumanInterventionManager>,
    enabled_layers: tokio::sync::RwLock<Vec<SecurityLayer>>,
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            rule_engine: tokio::sync::RwLock::new(RuleEngine::new()),
            audit_log: tokio::sync::RwLock::new(AuditLog::new()),
            compliance_engine: tokio::sync::RwLock::new(ComplianceEngine::new()),
            intervention_manager: tokio::sync::RwLock::new(HumanInterventionManager::new()),
            enabled_layers: tokio::sync::RwLock::new(vec![
                SecurityLayer::RuleBlocking,
                SecurityLayer::SandboxIsolation,
                SecurityLayer::ThreeLayerQualityCheck,
                SecurityLayer::AuditSigning,
                SecurityLayer::IndustryCompliance,
                SecurityLayer::HumanIntervention,
            ]),
        }
    }

    pub async fn enable_layer(&self, layer: SecurityLayer) {
        let mut enabled_layers = self.enabled_layers.write().await;
        if !enabled_layers.contains(&layer) {
            info!("Enabling security layer: {:?}", layer);
            enabled_layers.push(layer);
        }
    }

    pub async fn disable_layer(&self, layer: &SecurityLayer) {
        info!("Disabling security layer: {:?}", layer);
        let mut enabled_layers = self.enabled_layers.write().await;
        enabled_layers.retain(|l| l != layer);
    }

    pub async fn validate_task(&self, task: &Task) -> Result<SecurityValidationResult> {
        info!("Validating security for task: {}", task.id);
        let mut result = SecurityValidationResult::new(task.id.clone());
        
        // 锁获取顺序 1: enabled_layers (只读，立即释放)
        let enabled_layers: Vec<SecurityLayer> = {
            let guard = self.enabled_layers.read().await;
            guard.clone()
        };

        // 锁获取顺序 2: rule_engine (只读，在需要时获取)
        if enabled_layers.contains(&SecurityLayer::RuleBlocking) {
            let rule_engine = self.rule_engine.read().await;
            let rule_passed = rule_engine.check(task);
            result.add_layer_result(SecurityLayer::RuleBlocking, rule_passed);

            if !rule_passed {
                // 克隆违规信息以避免生命周期问题
                let violations: Vec<_> = rule_engine
                    .get_violations(task)
                    .iter()
                    .map(|r| r.rule_name.clone())
                    .collect();
                for rule_name in violations {
                    result.add_violation(format!(
                        "Rule violation: {}",
                        rule_name
                    ));
                }
            }
        }

        if enabled_layers.contains(&SecurityLayer::SandboxIsolation) {
            result.add_layer_result(SecurityLayer::SandboxIsolation, true);
        }

        if enabled_layers.contains(&SecurityLayer::ThreeLayerQualityCheck) {
            result.add_layer_result(SecurityLayer::ThreeLayerQualityCheck, true);
        }

        // 锁获取顺序 3: compliance_engine (只读)
        if enabled_layers.contains(&SecurityLayer::IndustryCompliance) {
            let compliance_engine = self.compliance_engine.read().await;
            let compliance_results = compliance_engine.check_task_compliance(task).await?;
            let compliant = compliance_engine.is_compliant(&compliance_results);
            result.add_layer_result(SecurityLayer::IndustryCompliance, compliant);
            
            if !compliant {
                for check_result in compliance_results {
                    if check_result.status != compliance::ComplianceStatus::Compliant {
                        result.add_violation(format!(
                            "Compliance violation: {} - {:?}",
                            check_result.rule_name, check_result.status
                        ));
                    }
                }
            }
        }

        // 锁获取顺序 4: audit_log (写锁)
        if enabled_layers.contains(&SecurityLayer::AuditSigning) {
            let audit_event = AuditEvent::new(
                AuditEventType::TaskValidated,
                Some(task.id.clone()),
                None,
                None,
                result.passed,
                serde_json::json!({
                    "passed": result.passed,
                    "violations": result.violations,
                    "warnings": result.warnings,
                }),
            );
            let mut audit_log = self.audit_log.write().await;
            audit_log.log(audit_event).await?;
            result.add_layer_result(SecurityLayer::AuditSigning, true);
        }

        // 锁获取顺序 5: intervention_manager (只读)
        if enabled_layers.contains(&SecurityLayer::HumanIntervention) {
            let intervention_manager = self.intervention_manager.read().await;
            let pending_interventions = intervention_manager
                .get_pending_requests_for_task(&task.id);
            
            if !pending_interventions.is_empty() {
                result.add_warning(format!(
                    "Task has {} pending intervention requests",
                    pending_interventions.len()
                ));
            }
            result.add_layer_result(SecurityLayer::HumanIntervention, true);
        }

        info!(
            "Security validation for task {}: passed={}, violations={}, warnings={}",
            task.id,
            result.passed,
            result.violations.len(),
            result.warnings.len()
        );

        Ok(result)
    }

    pub async fn audit_log(&self) -> tokio::sync::RwLockReadGuard<'_, AuditLog> {
        self.audit_log.read().await
    }

    pub async fn log_security_event(&self, event: AuditEvent) -> Result<()> {
        let mut audit_log = self.audit_log.write().await;
        audit_log.log(event).await
    }

    pub async fn add_security_rule(&self, rule: SecurityRule) {
        let mut rule_engine = self.rule_engine.write().await;
        rule_engine.add_rule(rule);
    }

    pub async fn enable_compliance_standard(&self, standard: ComplianceStandard) {
        let mut compliance_engine = self.compliance_engine.write().await;
        compliance_engine.enable_standard(standard);
    }

    pub async fn request_human_intervention(&self, request: InterventionRequest) -> Result<()> {
        let intervention_manager = self.intervention_manager.read().await;
        intervention_manager.request_intervention(request).await
    }

    pub async fn check_circuit_breaker(&self, key: &str) -> bool {
        let intervention_manager = self.intervention_manager.read().await;
        intervention_manager.check_circuit_breaker(key)
    }

    pub async fn record_circuit_success(&self, key: &str) {
        let intervention_manager = self.intervention_manager.read().await;
        intervention_manager.record_circuit_success(key);
    }

    pub async fn record_circuit_failure(&self, key: &str) {
        let intervention_manager = self.intervention_manager.read().await;
        intervention_manager.record_circuit_failure(key);
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}
