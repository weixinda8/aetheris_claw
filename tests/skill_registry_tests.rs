use aetheris::skill::{
    BaseSkill, Skill, SkillMetadata, Version,
    registry::{
        SecurityScanResult, SkillRegistry, SkillVersionMetadata, Vulnerability,
        VulnerabilitySeverity,
    },
};
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;

fn create_test_skill(id: &str, name: &str, version: Version) -> Arc<dyn Skill> {
    let metadata = SkillMetadata::new(
        id.to_string(),
        name.to_string(),
        version,
        format!("Test skill: {}", name),
    );
    BaseSkill::new_arc(metadata)
}

#[test]
fn test_skill_version_metadata_initialization() {
    let now = Utc::now();
    let metadata = SkillVersionMetadata {
        commit_hash: Some("abc123".to_string()),
        published_at: Some(now),
        security_approved: true,
        changelog: Some("Initial release".to_string()),
    };

    assert_eq!(metadata.commit_hash, Some("abc123".to_string()));
    assert_eq!(metadata.published_at, Some(now));
    assert!(metadata.security_approved);
    assert_eq!(metadata.changelog, Some("Initial release".to_string()));
}

#[test]
fn test_skill_version_metadata_none_values() {
    let metadata = SkillVersionMetadata {
        commit_hash: None,
        published_at: None,
        security_approved: false,
        changelog: None,
    };

    assert!(metadata.commit_hash.is_none());
    assert!(metadata.published_at.is_none());
    assert!(!metadata.security_approved);
    assert!(metadata.changelog.is_none());
}

#[test]
fn test_skill_version_metadata_clone_and_partial_eq() {
    let now = Utc::now();
    let metadata1 = SkillVersionMetadata {
        commit_hash: Some("abc123".to_string()),
        published_at: Some(now),
        security_approved: true,
        changelog: Some("Initial release".to_string()),
    };
    let metadata2 = metadata1.clone();

    assert_eq!(metadata1, metadata2);
}

#[test]
fn test_vulnerability_severity_variants() {
    let severities = vec![
        VulnerabilitySeverity::Critical,
        VulnerabilitySeverity::High,
        VulnerabilitySeverity::Medium,
        VulnerabilitySeverity::Low,
        VulnerabilitySeverity::Info,
    ];

    assert_eq!(severities.len(), 5);
}

#[test]
fn test_vulnerability_initialization() {
    let now = Utc::now();
    let fixed_version = Version::new(1, 1, 0);
    let vulnerability = Vulnerability {
        id: "CVE-2024-0001".to_string(),
        severity: VulnerabilitySeverity::Critical,
        description: "SQL injection vulnerability".to_string(),
        discovered_at: now,
        fixed_in_version: Some(fixed_version.clone()),
    };

    assert_eq!(vulnerability.id, "CVE-2024-0001");
    assert_eq!(vulnerability.severity, VulnerabilitySeverity::Critical);
    assert_eq!(vulnerability.description, "SQL injection vulnerability");
    assert_eq!(vulnerability.discovered_at, now);
    assert_eq!(vulnerability.fixed_in_version, Some(fixed_version));
}

#[test]
fn test_vulnerability_no_fixed_version() {
    let now = Utc::now();
    let vulnerability = Vulnerability {
        id: "CVE-2024-0002".to_string(),
        severity: VulnerabilitySeverity::High,
        description: "Cross-site scripting vulnerability".to_string(),
        discovered_at: now,
        fixed_in_version: None,
    };

    assert!(vulnerability.fixed_in_version.is_none());
}

#[test]
fn test_vulnerability_clone_and_partial_eq() {
    let now = Utc::now();
    let vuln1 = Vulnerability {
        id: "CVE-2024-0003".to_string(),
        severity: VulnerabilitySeverity::Medium,
        description: "Information disclosure".to_string(),
        discovered_at: now,
        fixed_in_version: None,
    };
    let vuln2 = vuln1.clone();

    assert_eq!(vuln1, vuln2);
}

#[test]
fn test_security_scan_result_initialization() {
    let now = Utc::now();
    let vulnerabilities = vec![Vulnerability {
        id: "CVE-2024-0004".to_string(),
        severity: VulnerabilitySeverity::Low,
        description: "Minor issue".to_string(),
        discovered_at: now,
        fixed_in_version: None,
    }];
    let scan_result = SecurityScanResult {
        scan_id: "SCAN-001".to_string(),
        scanned_at: now,
        vulnerabilities: vulnerabilities.clone(),
        passed: false,
    };

    assert_eq!(scan_result.scan_id, "SCAN-001");
    assert_eq!(scan_result.scanned_at, now);
    assert_eq!(scan_result.vulnerabilities, vulnerabilities);
    assert!(!scan_result.passed);
}

#[test]
fn test_security_scan_result_passed() {
    let now = Utc::now();
    let scan_result = SecurityScanResult {
        scan_id: "SCAN-002".to_string(),
        scanned_at: now,
        vulnerabilities: vec![],
        passed: true,
    };

    assert!(scan_result.passed);
    assert!(scan_result.vulnerabilities.is_empty());
}

#[test]
fn test_security_scan_result_clone_and_partial_eq() {
    let now = Utc::now();
    let scan1 = SecurityScanResult {
        scan_id: "SCAN-003".to_string(),
        scanned_at: now,
        vulnerabilities: vec![],
        passed: true,
    };
    let scan2 = scan1.clone();

    assert_eq!(scan1, scan2);
}

#[test]
fn test_register_with_metadata_normal() {
    let registry = SkillRegistry::new();
    let version = Version::new(1, 0, 0);
    let skill = create_test_skill("test-skill", "Test Skill", version.clone());
    let now = Utc::now();
    let metadata = SkillVersionMetadata {
        commit_hash: Some("abc123".to_string()),
        published_at: Some(now),
        security_approved: true,
        changelog: Some("Initial release".to_string()),
    };

    let result = registry.register_with_metadata(skill, metadata.clone());
    assert!(result.is_ok());

    let retrieved_metadata = registry
        .get_version_metadata("test-skill", &version)
        .unwrap();
    assert_eq!(retrieved_metadata, Some(metadata));
}

#[test]
fn test_register_with_metadata_backward_compatibility() {
    let registry = SkillRegistry::new();
    let version = Version::new(1, 0, 0);
    let skill = create_test_skill("legacy-skill", "Legacy Skill", version.clone());

    registry.register(skill.clone());

    assert!(registry.exists("legacy-skill"));
    assert!(registry.version_exists("legacy-skill", &version));

    let retrieved_skill = registry.get("legacy-skill");
    assert!(retrieved_skill.is_some());
}

#[test]
fn test_get_version_metadata_exists() {
    let registry = SkillRegistry::new();
    let version = Version::new(2, 0, 0);
    let skill = create_test_skill("metadata-skill", "Metadata Skill", version.clone());
    let now = Utc::now();
    let metadata = SkillVersionMetadata {
        commit_hash: Some("def456".to_string()),
        published_at: Some(now),
        security_approved: false,
        changelog: Some("Second release".to_string()),
    };

    registry
        .register_with_metadata(skill, metadata.clone())
        .unwrap();

    let result = registry.get_version_metadata("metadata-skill", &version);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some(metadata));
}

#[test]
fn test_get_version_metadata_skill_not_exists() {
    let registry = SkillRegistry::new();
    let version = Version::new(1, 0, 0);

    let result = registry.get_version_metadata("nonexistent-skill", &version);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_get_version_metadata_version_not_exists() {
    let registry = SkillRegistry::new();
    let version1 = Version::new(1, 0, 0);
    let version2 = Version::new(2, 0, 0);
    let skill = create_test_skill("version-test", "Version Test", version1.clone());
    let metadata = SkillVersionMetadata {
        commit_hash: None,
        published_at: None,
        security_approved: true,
        changelog: None,
    };

    registry.register_with_metadata(skill, metadata).unwrap();

    let result = registry.get_version_metadata("version-test", &version2);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_record_security_scan_normal() {
    let registry = SkillRegistry::new();
    let version = Version::new(1, 0, 0);
    let skill = create_test_skill("scan-skill", "Scan Skill", version.clone());
    registry.register(skill);

    let now = Utc::now();
    let scan_result = SecurityScanResult {
        scan_id: "TEST-SCAN-001".to_string(),
        scanned_at: now,
        vulnerabilities: vec![],
        passed: true,
    };

    let result = registry.record_security_scan("scan-skill", scan_result.clone());
    assert!(result.is_ok());

    let history = registry.get_security_scan_history("scan-skill").unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0], scan_result);
}

#[test]
fn test_record_security_scan_skill_not_exists() {
    let registry = SkillRegistry::new();
    let now = Utc::now();
    let scan_result = SecurityScanResult {
        scan_id: "TEST-SCAN-002".to_string(),
        scanned_at: now,
        vulnerabilities: vec![],
        passed: true,
    };

    let result = registry.record_security_scan("nonexistent-skill", scan_result);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Skill not found"));
}

#[test]
fn test_list_versions_with_metadata() {
    let registry = SkillRegistry::new();
    let v1 = Version::new(1, 0, 0);
    let v2 = Version::new(2, 0, 0);

    let skill1 = create_test_skill("multi-version", "Multi Version Skill", v1.clone());
    let skill2 = create_test_skill("multi-version", "Multi Version Skill", v2.clone());

    let metadata1 = SkillVersionMetadata {
        commit_hash: Some("commit1".to_string()),
        published_at: Some(Utc::now() - Duration::days(30)),
        security_approved: true,
        changelog: Some("v1 release".to_string()),
    };
    let metadata2 = SkillVersionMetadata {
        commit_hash: Some("commit2".to_string()),
        published_at: Some(Utc::now()),
        security_approved: true,
        changelog: Some("v2 release".to_string()),
    };

    registry
        .register_with_metadata(skill1, metadata1.clone())
        .unwrap();
    registry
        .register_with_metadata(skill2, metadata2.clone())
        .unwrap();

    let result = registry
        .list_versions_with_metadata("multi-version")
        .unwrap();
    assert_eq!(result.len(), 2);

    let (ver1, _, meta1) = &result[0];
    let (ver2, _, meta2) = &result[1];

    assert_eq!(ver1, &v1);
    assert_eq!(ver2, &v2);
    assert_eq!(*meta1, Some(metadata1));
    assert_eq!(*meta2, Some(metadata2));
}

#[test]
fn test_list_versions_with_metadata_nonexistent_skill() {
    let registry = SkillRegistry::new();
    let result = registry.list_versions_with_metadata("nonexistent").unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_get_security_scan_history() {
    let registry = SkillRegistry::new();
    let version = Version::new(1, 0, 0);
    let skill = create_test_skill("history-skill", "History Skill", version.clone());
    registry.register(skill);

    let now = Utc::now();
    let scan1 = SecurityScanResult {
        scan_id: "HIST-001".to_string(),
        scanned_at: now - Duration::days(1),
        vulnerabilities: vec![],
        passed: true,
    };
    let scan2 = SecurityScanResult {
        scan_id: "HIST-002".to_string(),
        scanned_at: now,
        vulnerabilities: vec![],
        passed: true,
    };

    registry
        .record_security_scan("history-skill", scan1.clone())
        .unwrap();
    registry
        .record_security_scan("history-skill", scan2.clone())
        .unwrap();

    let history = registry.get_security_scan_history("history-skill").unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0], scan1);
    assert_eq!(history[1], scan2);
}

#[test]
fn test_get_security_scan_history_empty() {
    let registry = SkillRegistry::new();
    let version = Version::new(1, 0, 0);
    let skill = create_test_skill("empty-history", "Empty History", version.clone());
    registry.register(skill);

    let history = registry.get_security_scan_history("empty-history").unwrap();
    assert!(history.is_empty());
}

#[test]
fn test_get_security_scan_history_nonexistent_skill() {
    let registry = SkillRegistry::new();
    let history = registry.get_security_scan_history("nonexistent").unwrap();
    assert!(history.is_empty());
}

#[test]
fn test_unregister_cleanup() {
    let registry = SkillRegistry::new();
    let v1 = Version::new(1, 0, 0);
    let v2 = Version::new(2, 0, 0);

    let skill1 = create_test_skill("cleanup-skill", "Cleanup Skill", v1.clone());
    let skill2 = create_test_skill("cleanup-skill", "Cleanup Skill", v2.clone());

    let metadata = SkillVersionMetadata {
        commit_hash: None,
        published_at: None,
        security_approved: true,
        changelog: None,
    };

    registry
        .register_with_metadata(skill1, metadata.clone())
        .unwrap();
    registry.register_with_metadata(skill2, metadata).unwrap();

    let scan = SecurityScanResult {
        scan_id: "CLEAN-001".to_string(),
        scanned_at: Utc::now(),
        vulnerabilities: vec![],
        passed: true,
    };
    registry
        .record_security_scan("cleanup-skill", scan)
        .unwrap();

    assert!(registry.exists("cleanup-skill"));
    assert!(
        !registry
            .get_security_scan_history("cleanup-skill")
            .unwrap()
            .is_empty()
    );

    let result = registry.unregister("cleanup-skill");
    assert!(result.is_ok());

    assert!(!registry.exists("cleanup-skill"));
    assert!(
        registry
            .get_security_scan_history("cleanup-skill")
            .unwrap()
            .is_empty()
    );
    assert!(
        registry
            .list_versions_with_metadata("cleanup-skill")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn test_unregister_version_cleanup() {
    let registry = SkillRegistry::new();
    let v1 = Version::new(1, 0, 0);
    let v2 = Version::new(2, 0, 0);

    let skill1 = create_test_skill("version-cleanup", "Version Cleanup", v1.clone());
    let skill2 = create_test_skill("version-cleanup", "Version Cleanup", v2.clone());

    let metadata1 = SkillVersionMetadata {
        commit_hash: Some("v1-commit".to_string()),
        published_at: None,
        security_approved: true,
        changelog: None,
    };
    let metadata2 = SkillVersionMetadata {
        commit_hash: Some("v2-commit".to_string()),
        published_at: None,
        security_approved: true,
        changelog: None,
    };

    registry.register_with_metadata(skill1, metadata1).unwrap();
    registry
        .register_with_metadata(skill2, metadata2.clone())
        .unwrap();

    let result = registry.unregister_version("version-cleanup", &v1);
    assert!(result.is_ok());

    assert!(!registry.version_exists("version-cleanup", &v1));
    assert!(registry.version_exists("version-cleanup", &v2));

    let v1_meta = registry
        .get_version_metadata("version-cleanup", &v1)
        .unwrap();
    let v2_meta = registry
        .get_version_metadata("version-cleanup", &v2)
        .unwrap();
    assert!(v1_meta.is_none());
    assert_eq!(v2_meta, Some(metadata2));
}

#[test]
fn test_boundary_conditions_none_metadata() {
    let registry = SkillRegistry::new();
    let version = Version::new(1, 0, 0);
    let skill = create_test_skill("boundary", "Boundary Test", version.clone());
    let metadata = SkillVersionMetadata {
        commit_hash: None,
        published_at: None,
        security_approved: false,
        changelog: None,
    };

    registry
        .register_with_metadata(skill, metadata.clone())
        .unwrap();

    let retrieved = registry
        .get_version_metadata("boundary", &version)
        .unwrap()
        .unwrap();
    assert!(retrieved.commit_hash.is_none());
    assert!(retrieved.published_at.is_none());
    assert!(!retrieved.security_approved);
    assert!(retrieved.changelog.is_none());
}

#[test]
fn test_boundary_conditions_empty_vulnerabilities() {
    let registry = SkillRegistry::new();
    let version = Version::new(1, 0, 0);
    let skill = create_test_skill("empty-vulns", "Empty Vulns", version.clone());
    registry.register(skill);

    let scan = SecurityScanResult {
        scan_id: "EMPTY-001".to_string(),
        scanned_at: Utc::now(),
        vulnerabilities: vec![],
        passed: true,
    };

    registry.record_security_scan("empty-vulns", scan).unwrap();

    let history = registry.get_security_scan_history("empty-vulns").unwrap();
    assert_eq!(history.len(), 1);
    assert!(history[0].vulnerabilities.is_empty());
    assert!(history[0].passed);
}

#[test]
fn test_boundary_conditions_multiple_vulnerabilities() {
    let registry = SkillRegistry::new();
    let version = Version::new(1, 0, 0);
    let skill = create_test_skill("multi-vulns", "Multi Vulns", version.clone());
    registry.register(skill);

    let now = Utc::now();
    let vuln1 = Vulnerability {
        id: "CVE-2024-1001".to_string(),
        severity: VulnerabilitySeverity::Critical,
        description: "Critical issue".to_string(),
        discovered_at: now,
        fixed_in_version: None,
    };
    let vuln2 = Vulnerability {
        id: "CVE-2024-1002".to_string(),
        severity: VulnerabilitySeverity::High,
        description: "High issue".to_string(),
        discovered_at: now,
        fixed_in_version: None,
    };
    let vuln3 = Vulnerability {
        id: "CVE-2024-1003".to_string(),
        severity: VulnerabilitySeverity::Low,
        description: "Low issue".to_string(),
        discovered_at: now,
        fixed_in_version: None,
    };

    let scan = SecurityScanResult {
        scan_id: "MULTI-001".to_string(),
        scanned_at: now,
        vulnerabilities: vec![vuln1.clone(), vuln2.clone(), vuln3.clone()],
        passed: false,
    };

    registry.record_security_scan("multi-vulns", scan).unwrap();

    let history = registry.get_security_scan_history("multi-vulns").unwrap();
    assert_eq!(history[0].vulnerabilities.len(), 3);
    assert_eq!(history[0].vulnerabilities[0], vuln1);
    assert_eq!(history[0].vulnerabilities[1], vuln2);
    assert_eq!(history[0].vulnerabilities[2], vuln3);
}

#[test]
fn test_skill_registry_clone() {
    let original = SkillRegistry::new();
    let version = Version::new(1, 0, 0);
    let skill = create_test_skill("clone-test", "Clone Test", version.clone());
    let metadata = SkillVersionMetadata {
        commit_hash: Some("clone-commit".to_string()),
        published_at: None,
        security_approved: true,
        changelog: None,
    };

    original
        .register_with_metadata(skill, metadata.clone())
        .unwrap();

    let cloned = original.clone();

    assert!(cloned.exists("clone-test"));
    let cloned_metadata = cloned.get_version_metadata("clone-test", &version).unwrap();
    assert_eq!(cloned_metadata, Some(metadata));
}

#[test]
fn test_skill_registry_default() {
    let registry = SkillRegistry::default();
    assert_eq!(registry.skill_count(), 0);
}

#[test]
fn test_unregister_nonexistent_skill() {
    let registry = SkillRegistry::new();
    let result = registry.unregister("nonexistent");
    assert!(result.is_ok());
}

#[test]
fn test_unregister_version_nonexistent_skill() {
    let registry = SkillRegistry::new();
    let version = Version::new(1, 0, 0);
    let result = registry.unregister_version("nonexistent", &version);
    assert!(result.is_ok());
}

#[test]
fn test_unregister_version_nonexistent_version() {
    let registry = SkillRegistry::new();
    let v1 = Version::new(1, 0, 0);
    let v2 = Version::new(2, 0, 0);
    let skill = create_test_skill("version-nonexist", "Version Nonexist", v1.clone());
    registry.register(skill);

    let result = registry.unregister_version("version-nonexist", &v2);
    assert!(result.is_ok());
    assert!(registry.version_exists("version-nonexist", &v1));
}
