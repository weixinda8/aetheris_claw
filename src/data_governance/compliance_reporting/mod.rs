use crate::data_governance::DataLineage;
use crate::security::{AuditEvent, AuditEventType};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ComplianceStandard {
    FDA,
    GDPR,
    CybersecurityLevel2,
    ISO27001,
    SOC2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ReportFormat {
    PDF,
    HTML,
    JSON,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReportType {
    SecurityAudit,
    DataGovernance,
    ComplianceCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub standard: ComplianceStandard,
    pub format: ReportFormat,
    pub report_type: ReportType,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ReportTemplate {
    pub fn new(
        name: String,
        description: String,
        standard: ComplianceStandard,
        format: ReportFormat,
        report_type: ReportType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            standard,
            format,
            report_type,
            content: String::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_content(mut self, content: String) -> Self {
        self.content = content;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub standard: ComplianceStandard,
    pub format: ReportFormat,
    pub report_type: ReportType,
    pub content: String,
    pub template_id: Option<Uuid>,
    pub generated_by: String,
    pub generated_at: DateTime<Utc>,
    pub signed_by: Option<String>,
    pub signed_at: Option<DateTime<Utc>>,
    pub signature: Option<String>,
    pub period_start: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
    pub data_lineage: Option<DataLineage>,
    pub audit_events: Option<Vec<AuditEvent>>,
}

impl ComplianceReport {
    pub fn new(
        name: String,
        description: String,
        standard: ComplianceStandard,
        format: ReportFormat,
        report_type: ReportType,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            standard,
            format,
            report_type,
            content: String::new(),
            template_id: None,
            generated_by: String::new(),
            generated_at: Utc::now(),
            signed_by: None,
            signed_at: None,
            signature: None,
            period_start: None,
            period_end: None,
            data_lineage: None,
            audit_events: None,
        }
    }

    pub fn with_content(mut self, content: String) -> Self {
        self.content = content;
        self
    }

    pub fn with_template(mut self, template_id: Uuid) -> Self {
        self.template_id = Some(template_id);
        self
    }

    pub fn with_period(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.period_start = Some(start);
        self.period_end = Some(end);
        self
    }

    pub fn with_data_lineage(mut self, lineage: DataLineage) -> Self {
        self.data_lineage = Some(lineage);
        self
    }

    pub fn with_audit_events(mut self, events: Vec<AuditEvent>) -> Self {
        self.audit_events = Some(events);
        self
    }

    pub fn sign(&mut self, signer: String, signature: String) {
        self.signed_by = Some(signer);
        self.signed_at = Some(Utc::now());
        self.signature = Some(signature);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledReport {
    pub id: Uuid,
    pub name: String,
    pub template_id: Uuid,
    pub cron_expression: String,
    pub next_run: DateTime<Utc>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ScheduledReport {
    pub fn new(name: String, template_id: Uuid, cron_expression: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            template_id,
            cron_expression,
            next_run: now + Duration::hours(1),
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheck {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub standard: ComplianceStandard,
    pub check_type: String,
    pub status: CheckStatus,
    pub details: String,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CheckStatus {
    Passed,
    Failed,
    Warning,
    NotApplicable,
}

/// 报告生成配置
/// 
/// 用于配置所有类型的合规报告生成
/// 
/// # 示例
/// 
/// ```
/// let config = ReportConfig::minimal(
///     "Security Audit".to_string(),
///     "Quarterly security audit".to_string(),
///     ComplianceStandard::ISO27001,
///     ReportFormat::PDF,
///     ReportType::SecurityAudit,
///     "Security Team".to_string(),
/// );
/// ```
#[derive(Debug, Clone)]
pub struct ReportConfig {
    /// 报告名称
    pub name: String,
    /// 报告描述
    pub description: String,
    /// 适用的合规标准
    pub standard: ComplianceStandard,
    /// 报告输出格式
    pub format: ReportFormat,
    /// 报告类型
    pub report_type: ReportType,
    /// 可选的报告模板ID
    pub template_id: Option<Uuid>,
    /// 报告生成者标识
    pub generated_by: String,
    /// 可选的报告周期开始时间
    pub period_start: Option<DateTime<Utc>>,
    /// 可选的报告周期结束时间
    pub period_end: Option<DateTime<Utc>>,
    /// 可选的数据血缘信息
    pub data_lineage: Option<DataLineage>,
    /// 可选的审计事件列表
    pub audit_events: Option<Vec<AuditEvent>>,
}

impl ReportConfig {
    /// 创建一个最小化的报告配置
    /// 
    /// # 参数
    /// - `name`: 报告名称
    /// - `description`: 报告描述
    /// - `standard`: 合规标准
    /// - `format`: 报告格式
    /// - `report_type`: 报告类型
    /// - `generated_by`: 报告生成者
    /// 
    /// # 返回
    /// 预配置的 ReportConfig，所有可选字段设为 None
    pub fn minimal(
        name: String,
        description: String,
        standard: ComplianceStandard,
        format: ReportFormat,
        report_type: ReportType,
        generated_by: String,
    ) -> Self {
        Self {
            name,
            description,
            standard,
            format,
            report_type,
            template_id: None,
            generated_by,
            period_start: None,
            period_end: None,
            data_lineage: None,
            audit_events: None,
        }
    }
}

pub struct ComplianceReportGenerator {
    templates: HashMap<Uuid, ReportTemplate>,
    reports: HashMap<Uuid, ComplianceReport>,
    scheduled_reports: HashMap<Uuid, ScheduledReport>,
    checks: HashMap<Uuid, ComplianceCheck>,
}

impl ComplianceReportGenerator {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            reports: HashMap::new(),
            scheduled_reports: HashMap::new(),
            checks: HashMap::new(),
        }
    }

    pub fn add_template(&mut self, template: ReportTemplate) {
        self.templates.insert(template.id, template);
    }

    pub fn get_template(&self, template_id: &Uuid) -> Option<&ReportTemplate> {
        self.templates.get(template_id)
    }

    pub fn list_templates(&self) -> Vec<&ReportTemplate> {
        self.templates.values().collect()
    }

    pub fn add_check(&mut self, check: ComplianceCheck) {
        self.checks.insert(check.id, check);
    }

    pub fn get_check(&self, check_id: &Uuid) -> Option<&ComplianceCheck> {
        self.checks.get(check_id)
    }

    pub fn list_checks(&self) -> Vec<&ComplianceCheck> {
        self.checks.values().collect()
    }

    pub fn generate_security_audit_report(&mut self, config: ReportConfig) -> ComplianceReport {
        let mut report = ComplianceReport::new(
            config.name,
            config.description,
            config.standard,
            config.format,
            ReportType::SecurityAudit,
        );
        report.generated_by = config.generated_by;
        report.period_start = config.period_start;
        report.period_end = config.period_end;

        if let Some(events) = config.audit_events {
            report = report.with_audit_events(events);
        }

        report.content = self.generate_security_audit_content(&report);
        self.reports.insert(report.id, report.clone());
        report
    }

    pub fn generate_data_governance_report(&mut self, config: ReportConfig) -> ComplianceReport {
        let mut report = ComplianceReport::new(
            config.name,
            config.description,
            config.standard,
            config.format,
            ReportType::DataGovernance,
        );
        report.generated_by = config.generated_by;
        report.period_start = config.period_start;
        report.period_end = config.period_end;

        if let Some(lineage) = config.data_lineage {
            report = report.with_data_lineage(lineage);
        }

        report.content = self.generate_data_governance_content(&report);
        self.reports.insert(report.id, report.clone());
        report
    }

    pub fn generate_compliance_check_report(&mut self, config: ReportConfig) -> ComplianceReport {
        let mut report = ComplianceReport::new(
            config.name,
            config.description,
            config.standard,
            config.format,
            ReportType::ComplianceCheck,
        );
        report.generated_by = config.generated_by;
        report.period_start = config.period_start;
        report.period_end = config.period_end;

        report.content = self.generate_compliance_check_content(&report);
        self.reports.insert(report.id, report.clone());
        report
    }

    pub fn generate_report(&mut self, config: ReportConfig) -> ComplianceReport {
        let mut report = ComplianceReport::new(
            config.name,
            config.description,
            config.standard,
            config.format,
            config.report_type.clone(),
        );
        report.generated_by = config.generated_by;
        report.period_start = config.period_start;
        report.period_end = config.period_end;

        if let Some(lineage) = config.data_lineage {
            report = report.with_data_lineage(lineage);
        }
        if let Some(events) = config.audit_events {
            report = report.with_audit_events(events);
        }

        if let Some(tid) = config.template_id {
            report.template_id = Some(tid);
            if let Some(template) = self.templates.get(&tid) {
                report.content = self.render_template(template, &report);
            }
        } else {
            report.content = match config.report_type {
                ReportType::SecurityAudit => self.generate_security_audit_content(&report),
                ReportType::DataGovernance => self.generate_data_governance_content(&report),
                ReportType::ComplianceCheck => self.generate_compliance_check_content(&report),
            };
        }

        self.reports.insert(report.id, report.clone());
        report
    }

    fn generate_security_audit_content(&self, report: &ComplianceReport) -> String {
        let events_summary = if let Some(events) = &report.audit_events {
            let security_violations = events
                .iter()
                .filter(|e| e.event_type == AuditEventType::SecurityViolation)
                .count();
            let events_by_type: HashMap<AuditEventType, usize> =
                events.iter().fold(HashMap::new(), |mut acc, e| {
                    *acc.entry(e.event_type.clone()).or_insert(0) += 1;
                    acc
                });
            let type_summary: String = events_by_type
                .iter()
                .map(|(t, c)| format!("- {:?}: {}", t, c))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "## Audit Events Summary\n\nTotal Events: {}\nSecurity Violations: {}\n\n### Events by Type\n\n{}",
                events.len(),
                security_violations,
                type_summary
            )
        } else {
            "## Audit Events Summary\n\nNo audit events provided.".to_string()
        };

        format!(
            r#"# Security Audit Report: {}

## Description
{}

## Report Details
- **Standard**: {:?}
- **Generated By**: {}
- **Generated At**: {}
- **Period**: {} to {}

{}

## Recommendations
1. Review all security violations
2. Implement access control measures
3. Conduct regular security audits
4. Update security policies as needed
            "#,
            report.name,
            report.description,
            report.standard,
            report.generated_by,
            report.generated_at.to_rfc3339(),
            report
                .period_start
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| "N/A".to_string()),
            report
                .period_end
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| "N/A".to_string()),
            events_summary
        )
    }

    fn generate_data_governance_content(&self, report: &ComplianceReport) -> String {
        let lineage_summary = if let Some(lineage) = &report.data_lineage {
            let node_count = lineage.nodes.len();
            let edge_count = lineage.edges.len();
            let node_types: HashMap<String, usize> =
                lineage.nodes.values().fold(HashMap::new(), |mut acc, n| {
                    let type_str = format!("{:?}", n.node_type);
                    *acc.entry(type_str).or_insert(0) += 1;
                    acc
                });
            let type_summary: String = node_types
                .iter()
                .map(|(t, c)| format!("- {}: {}", t, c))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "## Data Lineage Summary\n\nTotal Nodes: {}\nTotal Edges: {}\n\n### Node Types\n\n{}",
                node_count, edge_count, type_summary
            )
        } else {
            "## Data Lineage Summary\n\nNo data lineage provided.".to_string()
        };

        format!(
            r#"# Data Governance Report: {}

## Description
{}

## Report Details
- **Standard**: {:?}
- **Generated By**: {}
- **Generated At**: {}
- **Period**: {} to {}

{}

## Governance Recommendations
1. Maintain data lineage documentation
2. Implement data classification
3. Establish data quality metrics
4. Regularly review data access policies
            "#,
            report.name,
            report.description,
            report.standard,
            report.generated_by,
            report.generated_at.to_rfc3339(),
            report
                .period_start
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| "N/A".to_string()),
            report
                .period_end
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| "N/A".to_string()),
            lineage_summary
        )
    }

    fn generate_compliance_check_content(&self, report: &ComplianceReport) -> String {
        let checks_for_standard: Vec<&ComplianceCheck> = self
            .checks
            .values()
            .filter(|c| c.standard == report.standard)
            .collect();

        let checks_summary: String = checks_for_standard
            .iter()
            .map(|c| format!("- {}: {:?}\n  Details: {}", c.name, c.status, c.details))
            .collect::<Vec<_>>()
            .join("\n\n");

        format!(
            r#"# Compliance Check Report: {}

## Description
{}

## Report Details
- **Standard**: {:?}
- **Generated By**: {}
- **Generated At**: {}
- **Period**: {} to {}

## Compliance Checks
{}

## Summary
Total checks: {}
Passed: {}
Failed: {}
Warnings: {}
Not Applicable: {}
            "#,
            report.name,
            report.description,
            report.standard,
            report.generated_by,
            report.generated_at.to_rfc3339(),
            report
                .period_start
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| "N/A".to_string()),
            report
                .period_end
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| "N/A".to_string()),
            checks_summary,
            checks_for_standard.len(),
            checks_for_standard
                .iter()
                .filter(|c| c.status == CheckStatus::Passed)
                .count(),
            checks_for_standard
                .iter()
                .filter(|c| c.status == CheckStatus::Failed)
                .count(),
            checks_for_standard
                .iter()
                .filter(|c| c.status == CheckStatus::Warning)
                .count(),
            checks_for_standard
                .iter()
                .filter(|c| c.status == CheckStatus::NotApplicable)
                .count(),
        )
    }

    fn render_template(&self, template: &ReportTemplate, report: &ComplianceReport) -> String {
        let mut content = template.content.clone();

        content = content.replace("{{report_id}}", &report.id.to_string());
        content = content.replace("{{report_name}}", &report.name);
        content = content.replace("{{report_description}}", &report.description);
        content = content.replace("{{generated_by}}", &report.generated_by);
        content = content.replace("{{generated_at}}", &report.generated_at.to_rfc3339());

        if let Some(start) = report.period_start {
            content = content.replace("{{period_start}}", &start.to_rfc3339());
        }
        if let Some(end) = report.period_end {
            content = content.replace("{{period_end}}", &end.to_rfc3339());
        }

        content
    }

    pub fn get_report(&self, report_id: &Uuid) -> Option<&ComplianceReport> {
        self.reports.get(report_id)
    }

    pub fn list_reports(&self) -> Vec<ComplianceReport> {
        self.reports.values().cloned().collect()
    }

    pub fn sign_report(
        &mut self,
        report_id: Uuid,
        signer: String,
        signature: String,
    ) -> Option<ComplianceReport> {
        if let Some(report) = self.reports.get_mut(&report_id) {
            report.sign(signer, signature);
            Some(report.clone())
        } else {
            None
        }
    }

    pub fn export_report(&self, report_id: &Uuid) -> Option<(ReportFormat, String)> {
        self.reports.get(report_id).map(|report| {
            let content = match report.format {
                ReportFormat::JSON => self.export_to_json(report),
                ReportFormat::HTML => self.export_to_html(report),
                ReportFormat::PDF => self.export_to_pdf_placeholder(report),
            };
            (report.format.clone(), content)
        })
    }

    fn export_to_json(&self, report: &ComplianceReport) -> String {
        serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".to_string())
    }

    fn export_to_html(&self, report: &ComplianceReport) -> String {
        let content_escaped = report
            .content
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('\n', "<br>");

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>{}</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        h1 {{ color: #2c3e50; }}
        .report-header {{ background: #f8f9fa; padding: 20px; border-radius: 5px; margin-bottom: 20px; }}
        .report-content {{ line-height: 1.6; }}
        .signature {{ margin-top: 40px; border-top: 1px solid #eee; padding-top: 20px; }}
    </style>
</head>
<body>
    <div class="report-header">
        <h1>{}</h1>
        <p><strong>Generated By:</strong> {}</p>
        <p><strong>Generated At:</strong> {}</p>
    </div>
    <div class="report-content">
        {}
    </div>
    {}
</body>
</html>"#,
            report.name,
            report.name,
            report.generated_by,
            report.generated_at.to_rfc3339(),
            content_escaped,
            if let Some(signer) = &report.signed_by {
                format!(
                    r#"<div class="signature">
                        <p><strong>Signed By:</strong> {}</p>
                        <p><strong>Signed At:</strong> {}</p>
                    </div>"#,
                    signer,
                    report
                        .signed_at
                        .map(|d| d.to_rfc3339())
                        .unwrap_or_else(|| "N/A".to_string())
                )
            } else {
                "".to_string()
            }
        )
    }

    fn export_to_pdf_placeholder(&self, report: &ComplianceReport) -> String {
        format!(
            "PDF Export Placeholder for Report: {}\n\n{}",
            report.name, report.content
        )
    }

    pub fn add_scheduled_report(&mut self, scheduled: ScheduledReport) {
        self.scheduled_reports.insert(scheduled.id, scheduled);
    }

    pub fn get_scheduled_report(&self, scheduled_id: &Uuid) -> Option<&ScheduledReport> {
        self.scheduled_reports.get(scheduled_id)
    }

    pub fn list_scheduled_reports(&self) -> Vec<&ScheduledReport> {
        self.scheduled_reports.values().collect()
    }

    pub fn create_compliance_check(
        &mut self,
        name: String,
        description: String,
        standard: ComplianceStandard,
        check_type: String,
        status: CheckStatus,
        details: String,
    ) -> ComplianceCheck {
        let check = ComplianceCheck {
            id: Uuid::new_v4(),
            name,
            description,
            standard,
            check_type,
            status,
            details,
            checked_at: Utc::now(),
        };
        self.checks.insert(check.id, check.clone());
        check
    }
}

impl Default for ComplianceReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_governance::{LineageEdge, LineageEdgeType, LineageNode, LineageNodeType};
    use crate::security::{AuditEvent, AuditEventType};
    use serde_json::json;

    #[test]
    fn test_compliance_report_generator_new() {
        let generator = ComplianceReportGenerator::new();
        assert!(generator.list_templates().is_empty());
        assert!(generator.list_reports().is_empty());
        assert!(generator.list_checks().is_empty());
    }

    #[test]
    fn test_compliance_report_generator_default() {
        let generator = ComplianceReportGenerator::default();
        assert!(generator.list_templates().is_empty());
    }

    #[test]
    fn test_add_template() {
        let mut generator = ComplianceReportGenerator::new();
        let template = ReportTemplate::new(
            "Test Template".to_string(),
            "Test Description".to_string(),
            ComplianceStandard::GDPR,
            ReportFormat::HTML,
            ReportType::ComplianceCheck,
        );
        let template_id = template.id;
        generator.add_template(template);

        assert_eq!(generator.list_templates().len(), 1);
        assert!(generator.get_template(&template_id).is_some());
    }

    #[test]
    fn test_create_compliance_check() {
        let mut generator = ComplianceReportGenerator::new();
        let check = generator.create_compliance_check(
            "Test Check".to_string(),
            "Test Check Description".to_string(),
            ComplianceStandard::GDPR,
            "Data Protection".to_string(),
            CheckStatus::Passed,
            "All requirements met".to_string(),
        );

        assert_eq!(generator.list_checks().len(), 1);
        assert_eq!(check.status, CheckStatus::Passed);
    }

    #[test]
    fn test_generate_compliance_check_report() {
        let mut generator = ComplianceReportGenerator::new();

        generator.create_compliance_check(
            "Check 1".to_string(),
            "Description 1".to_string(),
            ComplianceStandard::GDPR,
            "Type 1".to_string(),
            CheckStatus::Passed,
            "Details 1".to_string(),
        );

        let report_config = ReportConfig {
            name: "Test Report".to_string(),
            description: "Test Description".to_string(),
            standard: ComplianceStandard::GDPR,
            format: ReportFormat::HTML,
            report_type: ReportType::ComplianceCheck,
            template_id: None,
            generated_by: "Test User".to_string(),
            period_start: None,
            period_end: None,
            data_lineage: None,
            audit_events: None,
        };
        let report = generator.generate_compliance_check_report(report_config);

        assert_eq!(generator.list_reports().len(), 1);
        assert!(report.content.contains("Compliance Check Report"));
    }

    #[test]
    fn test_generate_security_audit_report() {
        let mut generator = ComplianceReportGenerator::new();

        let event1 = AuditEvent::new(
            AuditEventType::SecurityViolation,
            Some("task1".to_string()),
            Some("agent1".to_string()),
            Some("user1".to_string()),
            false,
            json!({"reason": "Unauthorized access"}),
        );

        let event2 = AuditEvent::new(
            AuditEventType::TaskCompleted,
            Some("task1".to_string()),
            Some("agent1".to_string()),
            Some("user1".to_string()),
            true,
            json!({"result": "success"}),
        );

        let report_config = ReportConfig {
            name: "Security Audit".to_string(),
            description: "Security audit report".to_string(),
            standard: ComplianceStandard::ISO27001,
            format: ReportFormat::JSON,
            report_type: ReportType::SecurityAudit,
            template_id: None,
            generated_by: "Security Team".to_string(),
            period_start: None,
            period_end: None,
            data_lineage: None,
            audit_events: Some(vec![event1, event2]),
        };
        let report = generator.generate_security_audit_report(report_config);

        assert!(report.content.contains("Security Audit Report"));
        assert!(report.audit_events.is_some());
    }

    #[test]
    fn test_generate_data_governance_report() {
        let mut generator = ComplianceReportGenerator::new();

        let mut lineage = DataLineage::new();
        let node1 = LineageNode::new(LineageNodeType::Table, "table1".to_string());
        let node2 = LineageNode::new(LineageNodeType::Table, "table2".to_string());
        let edge = LineageEdge::new(
            LineageEdgeType::ReadsFrom,
            node1.id.clone(),
            node2.id.clone(),
        );

        lineage.add_node(node1);
        lineage.add_node(node2);
        lineage.add_edge(edge);

        let report_config = ReportConfig {
            name: "Data Governance".to_string(),
            description: "Data governance report".to_string(),
            standard: ComplianceStandard::SOC2,
            format: ReportFormat::HTML,
            report_type: ReportType::DataGovernance,
            template_id: None,
            generated_by: "Governance Team".to_string(),
            period_start: None,
            period_end: None,
            data_lineage: Some(lineage),
            audit_events: None,
        };
        let report = generator.generate_data_governance_report(report_config);

        assert!(report.content.contains("Data Governance Report"));
        assert!(report.data_lineage.is_some());
    }

    #[test]
    fn test_sign_report() {
        let mut generator = ComplianceReportGenerator::new();

        let report_config = ReportConfig {
            name: "Test Report".to_string(),
            description: "Description".to_string(),
            standard: ComplianceStandard::GDPR,
            format: ReportFormat::HTML,
            report_type: ReportType::ComplianceCheck,
            template_id: None,
            generated_by: "User".to_string(),
            period_start: None,
            period_end: None,
            data_lineage: None,
            audit_events: None,
        };
        let report = generator.generate_compliance_check_report(report_config);

        let signed_report =
            generator.sign_report(report.id, "Signer".to_string(), "signature123".to_string());

        assert!(signed_report.is_some());
        assert_eq!(signed_report.unwrap().signed_by, Some("Signer".to_string()));
    }

    #[test]
    fn test_export_report() {
        let mut generator = ComplianceReportGenerator::new();

        let report_config = ReportConfig {
            name: "Test Report".to_string(),
            description: "Description".to_string(),
            standard: ComplianceStandard::GDPR,
            format: ReportFormat::JSON,
            report_type: ReportType::ComplianceCheck,
            template_id: None,
            generated_by: "User".to_string(),
            period_start: None,
            period_end: None,
            data_lineage: None,
            audit_events: None,
        };
        let report = generator.generate_compliance_check_report(report_config);

        let export = generator.export_report(&report.id);
        assert!(export.is_some());
        let (format, content) = export.unwrap();
        assert_eq!(format, ReportFormat::JSON);
        assert!(!content.is_empty());
    }

    #[test]
    fn test_export_to_html() {
        let mut generator = ComplianceReportGenerator::new();

        let mut report = ComplianceReport::new(
            "Test".to_string(),
            "Desc".to_string(),
            ComplianceStandard::GDPR,
            ReportFormat::HTML,
            ReportType::ComplianceCheck,
        );
        report.content = "Test content".to_string();

        let html = generator.export_to_html(&report);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Test content"));
    }

    #[test]
    fn test_render_template() {
        let mut generator = ComplianceReportGenerator::new();

        let template = ReportTemplate::new(
            "Template".to_string(),
            "Desc".to_string(),
            ComplianceStandard::GDPR,
            ReportFormat::HTML,
            ReportType::ComplianceCheck,
        )
        .with_content("Report: {{report_name}} by {{generated_by}}".to_string());

        let report = ComplianceReport::new(
            "Test Report".to_string(),
            "Desc".to_string(),
            ComplianceStandard::GDPR,
            ReportFormat::HTML,
            ReportType::ComplianceCheck,
        );

        let rendered = generator.render_template(&template, &report);
        assert!(rendered.contains("Report: Test Report"));
    }

    #[test]
    fn test_report_with_data_lineage() {
        let mut lineage = DataLineage::new();
        let node = LineageNode::new(LineageNodeType::Table, "test".to_string());
        lineage.add_node(node);

        let report = ComplianceReport::new(
            "Test".to_string(),
            "Desc".to_string(),
            ComplianceStandard::GDPR,
            ReportFormat::JSON,
            ReportType::DataGovernance,
        )
        .with_data_lineage(lineage);

        assert!(report.data_lineage.is_some());
    }

    #[test]
    fn test_report_with_audit_events() {
        let event = AuditEvent::new(
            AuditEventType::TaskSubmitted,
            None,
            None,
            None,
            true,
            json!({}),
        );

        let report = ComplianceReport::new(
            "Test".to_string(),
            "Desc".to_string(),
            ComplianceStandard::GDPR,
            ReportFormat::JSON,
            ReportType::SecurityAudit,
        )
        .with_audit_events(vec![event]);

        assert!(report.audit_events.is_some());
    }
}
