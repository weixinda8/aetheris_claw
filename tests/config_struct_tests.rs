use aetheris::data_governance::compliance_reporting::{
    ReportConfig, ReportType, ReportFormat, ComplianceStandard,
};
use aetheris::api::AppStateBuilder;
use uuid::Uuid;

#[test]
fn test_report_config_minimal() {
    let config = ReportConfig::minimal(
        "Test Report".to_string(),
        "Test Description".to_string(),
        ComplianceStandard::ISO27001,
        ReportFormat::PDF,
        ReportType::SecurityAudit,
        "Test Team".to_string(),
    );
    
    assert_eq!(config.name, "Test Report");
    assert_eq!(config.description, "Test Description");
    assert_eq!(config.standard, ComplianceStandard::ISO27001);
    assert_eq!(config.format, ReportFormat::PDF);
    assert_eq!(config.report_type, ReportType::SecurityAudit);
    assert_eq!(config.generated_by, "Test Team");
    assert!(config.template_id.is_none());
    assert!(config.period_start.is_none());
    assert!(config.period_end.is_none());
    assert!(config.data_lineage.is_none());
    assert!(config.audit_events.is_none());
}

#[test]
fn test_report_config_full() {
    let template_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    
    let config = ReportConfig {
        name: "Full Report".to_string(),
        description: "Full Description".to_string(),
        standard: ComplianceStandard::GDPR,
        format: ReportFormat::JSON,
        report_type: ReportType::DataGovernance,
        template_id: Some(template_id),
        generated_by: "Full Team".to_string(),
        period_start: Some(now),
        period_end: Some(now + chrono::Duration::days(7)),
        data_lineage: None,
        audit_events: None,
    };
    
    assert_eq!(config.name, "Full Report");
    assert_eq!(config.description, "Full Description");
    assert_eq!(config.standard, ComplianceStandard::GDPR);
    assert_eq!(config.format, ReportFormat::JSON);
    assert_eq!(config.report_type, ReportType::DataGovernance);
    assert_eq!(config.template_id, Some(template_id));
    assert_eq!(config.generated_by, "Full Team");
    assert_eq!(config.period_start, Some(now));
    assert_eq!(config.period_end, Some(now + chrono::Duration::days(7)));
}

#[test]
fn test_report_config_clone() {
    let config1 = ReportConfig::minimal(
        "Original".to_string(),
        "Desc".to_string(),
        ComplianceStandard::HIPAA,
        ReportFormat::HTML,
        ReportType::ComplianceCheck,
        "Tester".to_string(),
    );
    
    let config2 = config1.clone();
    
    assert_eq!(config1.name, config2.name);
    assert_eq!(config1.description, config2.description);
    assert_eq!(config1.standard, config2.standard);
    assert_eq!(config1.format, config2.format);
    assert_eq!(config1.report_type, config2.report_type);
    assert_eq!(config1.generated_by, config2.generated_by);
}

#[test]
fn test_app_state_builder_new() {
    let builder = AppStateBuilder::new();
    assert!(builder.commander.is_none());
    assert!(builder.security.is_none());
    assert!(builder.agents.is_none());
    assert!(builder.memory.is_none());
    assert!(builder.telemetry.is_none());
    assert!(builder.auth.is_none());
    assert!(builder.ws_manager.is_none());
    assert!(builder.rate_limiter.is_none());
    assert!(builder.opentelemetry.is_none());
    assert!(builder.skill_registry.is_none());
    assert!(builder.agent_skills_registry.is_none());
    assert!(builder.skill_marketplace.is_none());
    assert!(builder.industrial_protocol_manager.is_none());
    assert!(builder.timeseries_manager.is_none());
    assert!(builder.streaming_manager.is_none());
}

#[test]
fn test_app_state_builder_default() {
    let builder1 = AppStateBuilder::new();
    let builder2 = AppStateBuilder::default();
    
    assert!(builder1.commander.is_none());
    assert!(builder2.commander.is_none());
}

#[test]
fn test_app_state_builder_chaining() {
    let builder = AppStateBuilder::new();
    
    let builder = builder
        .commander(Default::default())
        .security(Default::default())
        .agents(Default::default())
        .memory(Default::default())
        .telemetry(Default::default())
        .auth(Default::default())
        .ws_manager(Default::default())
        .rate_limiter(Default::default())
        .skill_registry(Default::default())
        .agent_skills_registry(Default::default());
    
    assert!(builder.commander.is_some());
    assert!(builder.security.is_some());
    assert!(builder.agents.is_some());
    assert!(builder.memory.is_some());
    assert!(builder.telemetry.is_some());
    assert!(builder.auth.is_some());
    assert!(builder.ws_manager.is_some());
    assert!(builder.rate_limiter.is_some());
    assert!(builder.skill_registry.is_some());
    assert!(builder.agent_skills_registry.is_some());
}

#[test]
fn test_app_state_builder_set_methods() {
    let mut builder = AppStateBuilder::new();
    
    builder
        .set_commander(Default::default())
        .set_security(Default::default())
        .set_agents(Default::default())
        .set_memory(Default::default())
        .set_telemetry(Default::default())
        .set_auth(Default::default())
        .set_ws_manager(Default::default())
        .set_rate_limiter(Default::default())
        .set_skill_registry(Default::default())
        .set_agent_skills_registry(Default::default());
    
    assert!(builder.commander.is_some());
    assert!(builder.security.is_some());
    assert!(builder.agents.is_some());
    assert!(builder.memory.is_some());
    assert!(builder.telemetry.is_some());
    assert!(builder.auth.is_some());
    assert!(builder.ws_manager.is_some());
    assert!(builder.rate_limiter.is_some());
    assert!(builder.skill_registry.is_some());
    assert!(builder.agent_skills_registry.is_some());
}
