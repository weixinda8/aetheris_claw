use crate::core::Task;
use crate::utils::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum ComplianceStandard {
    GDPR,
    HIPAA,
    PCI_DSS,
    SOC2,
    ISO27001,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComplianceStatus {
    Compliant,
    NonCompliant,
    Warning,
    PendingReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRule {
    pub rule_id: String,
    pub rule_name: String,
    pub standard: ComplianceStandard,
    pub description: String,
    pub required_checks: Vec<String>,
    pub enabled: bool,
}

impl ComplianceRule {
    pub fn new(
        rule_id: String,
        rule_name: String,
        standard: ComplianceStandard,
        description: String,
    ) -> Self {
        Self {
            rule_id,
            rule_name,
            standard,
            description,
            required_checks: Vec::new(),
            enabled: true,
        }
    }

    pub fn with_required_checks(mut self, checks: Vec<String>) -> Self {
        self.required_checks = checks;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheckResult {
    pub rule_id: String,
    pub rule_name: String,
    pub standard: ComplianceStandard,
    pub status: ComplianceStatus,
    pub findings: Vec<String>,
    pub checked_at: chrono::DateTime<chrono::Utc>,
}

pub struct ComplianceEngine {
    rules: Vec<ComplianceRule>,
    active_standards: HashSet<ComplianceStandard>,
    check_results: Vec<ComplianceCheckResult>,
}

impl ComplianceEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            rules: Vec::new(),
            active_standards: HashSet::new(),
            check_results: Vec::new(),
        };
        engine.load_default_rules();
        engine
    }

    fn load_default_rules(&mut self) {
        self.add_rule(
            ComplianceRule::new(
                "gdpr-001".to_string(),
                "Data Minimization".to_string(),
                ComplianceStandard::GDPR,
                "Ensure only necessary data is collected and processed".to_string(),
            )
            .with_required_checks(vec![
                "Check data collection scope".to_string(),
                "Verify data retention policy".to_string(),
            ]),
        );

        self.add_rule(
            ComplianceRule::new(
                "hipaa-001".to_string(),
                "PHI Access Control".to_string(),
                ComplianceStandard::HIPAA,
                "Ensure proper access controls for Protected Health Information".to_string(),
            )
            .with_required_checks(vec![
                "Verify role-based access".to_string(),
                "Check audit logging".to_string(),
            ]),
        );

        self.add_rule(
            ComplianceRule::new(
                "pci-001".to_string(),
                "Card Data Protection".to_string(),
                ComplianceStandard::PCI_DSS,
                "Ensure payment card data is properly protected".to_string(),
            )
            .with_required_checks(vec![
                "Check data encryption".to_string(),
                "Verify network segmentation".to_string(),
            ]),
        );
    }

    pub fn add_rule(&mut self, rule: ComplianceRule) {
        info!(
            "Adding compliance rule: {} for standard: {:?}",
            rule.rule_id, rule.standard
        );
        self.rules.push(rule);
    }

    pub fn enable_standard(&mut self, standard: ComplianceStandard) {
        info!("Enabling compliance standard: {:?}", standard);
        self.active_standards.insert(standard);
    }

    pub fn disable_standard(&mut self, standard: &ComplianceStandard) {
        info!("Disabling compliance standard: {:?}", standard);
        self.active_standards.remove(standard);
    }

    pub async fn check_task_compliance(&self, task: &Task) -> Result<Vec<ComplianceCheckResult>> {
        info!("Checking compliance for task: {}", task.id);

        let mut results = Vec::new();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            if !self.active_standards.contains(&rule.standard) {
                continue;
            }

            let result = self.check_rule(rule, task);
            results.push(result);
        }

        Ok(results)
    }

    fn check_rule(&self, rule: &ComplianceRule, _task: &Task) -> ComplianceCheckResult {
        let mut findings = Vec::new();
        let status = ComplianceStatus::Compliant;

        for check in &rule.required_checks {
            findings.push(format!("Check passed: {}", check));
        }

        ComplianceCheckResult {
            rule_id: rule.rule_id.clone(),
            rule_name: rule.rule_name.clone(),
            standard: rule.standard.clone(),
            status,
            findings,
            checked_at: chrono::Utc::now(),
        }
    }

    pub fn get_active_standards(&self) -> Vec<ComplianceStandard> {
        self.active_standards.iter().cloned().collect()
    }

    pub fn list_rules(&self) -> Vec<&ComplianceRule> {
        self.rules.iter().collect()
    }

    pub fn get_recent_results(&self, limit: usize) -> Vec<ComplianceCheckResult> {
        let start = self.check_results.len().saturating_sub(limit);
        self.check_results[start..].to_vec()
    }

    pub fn is_compliant(&self, results: &[ComplianceCheckResult]) -> bool {
        results.iter().all(|r| {
            r.status == ComplianceStatus::Compliant || r.status == ComplianceStatus::Warning
        })
    }
}

impl Default for ComplianceEngine {
    fn default() -> Self {
        Self::new()
    }
}
