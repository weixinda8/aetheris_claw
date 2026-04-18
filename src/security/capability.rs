use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CapabilityType {
    FileSystem,
    Network,
    Process,
    Memory,
    SkillExecution,
    AgentManagement,
    Configuration,
    Security,
    Custom(String),
}

impl CapabilityType {
    pub fn as_str(&self) -> &str {
        match self {
            CapabilityType::FileSystem => "filesystem",
            CapabilityType::Network => "network",
            CapabilityType::Process => "process",
            CapabilityType::Memory => "memory",
            CapabilityType::SkillExecution => "skill_execution",
            CapabilityType::AgentManagement => "agent_management",
            CapabilityType::Configuration => "configuration",
            CapabilityType::Security => "security",
            CapabilityType::Custom(s) => s,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "filesystem" => CapabilityType::FileSystem,
            "network" => CapabilityType::Network,
            "process" => CapabilityType::Process,
            "memory" => CapabilityType::Memory,
            "skill_execution" => CapabilityType::SkillExecution,
            "agent_management" => CapabilityType::AgentManagement,
            "configuration" => CapabilityType::Configuration,
            "security" => CapabilityType::Security,
            _ => CapabilityType::Custom(s.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CapabilityScope {
    Global,
    Plugin(String),
    Agent(String),
    Task(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub id: String,
    pub capability_type: CapabilityType,
    pub scope: CapabilityScope,
    pub resources: Vec<String>,
    pub actions: Vec<String>,
    pub granted_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub granted_by: String,
    pub active: bool,
}

impl Capability {
    pub fn new(
        id: String,
        capability_type: CapabilityType,
        scope: CapabilityScope,
        resources: Vec<String>,
        actions: Vec<String>,
        granted_by: String,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            capability_type,
            scope,
            resources,
            actions,
            granted_at: now,
            expires_at: None,
            granted_by,
            active: true,
        }
    }

    pub fn with_expiry(mut self, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|e| chrono::Utc::now() >= e).unwrap_or(false)
    }

    pub fn is_active(&self) -> bool {
        self.active && !self.is_expired()
    }

    pub fn can_access(&self, resource: &str, action: &str) -> bool {
        if !self.is_active() {
            return false;
        }

        if !self.resources.is_empty() && !self.resources.contains(&resource.to_string()) {
            return false;
        }

        if !self.actions.is_empty() && !self.actions.contains(&action.to_string()) {
            return false;
        }

        true
    }

    pub fn matches_scope(&self, scope: &CapabilityScope) -> bool {
        match (&self.scope, scope) {
            (CapabilityScope::Global, _) => true,
            (CapabilityScope::Plugin(p1), CapabilityScope::Plugin(p2)) => p1 == p2,
            (CapabilityScope::Agent(a1), CapabilityScope::Agent(a2)) => a1 == a2,
            (CapabilityScope::Task(t1), CapabilityScope::Task(t2)) => t1 == t2,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRequest {
    pub request_id: String,
    pub requester: String,
    pub capability_type: CapabilityType,
    pub scope: CapabilityScope,
    pub resources: Vec<String>,
    pub actions: Vec<String>,
    pub reason: String,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub approved: Option<bool>,
    pub approved_by: Option<String>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl CapabilityRequest {
    pub fn new(
        requester: String,
        capability_type: CapabilityType,
        scope: CapabilityScope,
        resources: Vec<String>,
        actions: Vec<String>,
        reason: String,
    ) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            requester,
            capability_type,
            scope,
            resources,
            actions,
            reason,
            requested_at: chrono::Utc::now(),
            approved: None,
            approved_by: None,
            approved_at: None,
        }
    }

    pub fn approve(&mut self, approved_by: String) {
        self.approved = Some(true);
        self.approved_by = Some(approved_by);
        self.approved_at = Some(chrono::Utc::now());
    }

    pub fn deny(&mut self, approved_by: String) {
        self.approved = Some(false);
        self.approved_by = Some(approved_by);
        self.approved_at = Some(chrono::Utc::now());
    }

    pub fn is_pending(&self) -> bool {
        self.approved.is_none()
    }

    pub fn is_approved(&self) -> bool {
        self.approved == Some(true)
    }
}

pub struct CapabilityManager {
    capabilities: Arc<DashMap<String, Capability>>,
    requests: Arc<DashMap<String, CapabilityRequest>>,
    capability_index: Arc<DashMap<CapabilityType, Vec<String>>>,
    scope_index: Arc<DashMap<CapabilityScope, Vec<String>>>,
    requester_index: Arc<DashMap<String, Vec<String>>>,
}

impl CapabilityManager {
    pub fn new() -> Self {
        Self {
            capabilities: Arc::new(DashMap::new()),
            requests: Arc::new(DashMap::new()),
            capability_index: Arc::new(DashMap::new()),
            scope_index: Arc::new(DashMap::new()),
            requester_index: Arc::new(DashMap::new()),
        }
    }

    pub fn grant_capability(&self, capability: Capability) -> Result<()> {
        info!(
            "Granting capability: {} - Type: {:?}",
            capability.id, capability.capability_type
        );

        let capability_id = capability.id.clone();
        let capability_type = capability.capability_type.clone();
        let scope = capability.scope.clone();
        let granted_by = capability.granted_by.clone();

        self.capabilities.insert(capability_id.clone(), capability);

        self.update_indices(&capability_id, &capability_type, &scope, &granted_by);

        debug!("Capability granted successfully: {}", capability_id);
        Ok(())
    }

    fn update_indices(
        &self,
        capability_id: &str,
        capability_type: &CapabilityType,
        scope: &CapabilityScope,
        granted_by: &str,
    ) {
        self.capability_index
            .entry(capability_type.clone())
            .or_default()
            .push(capability_id.to_string());

        self.scope_index
            .entry(scope.clone())
            .or_default()
            .push(capability_id.to_string());

        self.requester_index
            .entry(granted_by.to_string())
            .or_default()
            .push(capability_id.to_string());
    }

    pub fn revoke_capability(&self, capability_id: &str) -> Result<()> {
        info!("Revoking capability: {}", capability_id);

        if let Some((_, mut capability)) = self.capabilities.remove(capability_id) {
            capability.active = false;
            self.capabilities.insert(capability_id.to_string(), capability);
        }

        Ok(())
    }

    pub fn get_capability(&self, capability_id: &str) -> Option<Capability> {
        self.capabilities.get(capability_id).map(|c| c.value().clone())
    }

    pub fn has_capability(
        &self,
        scope: &CapabilityScope,
        capability_type: &CapabilityType,
        resource: &str,
        action: &str,
    ) -> bool {
        for entry in self.capabilities.iter() {
            let capability = entry.value();
            if capability.is_active()
                && capability.capability_type == *capability_type
                && capability.matches_scope(scope)
                && capability.can_access(resource, action)
            {
                return true;
            }
        }
        false
    }

    pub fn check_access(
        &self,
        scope: &CapabilityScope,
        capability_type: &CapabilityType,
        resource: &str,
        action: &str,
    ) -> Result<()> {
        if self.has_capability(scope, capability_type, resource, action) {
            Ok(())
        } else {
            Err(AetherisError::Security(format!(
                "Access denied: {:?} {} {} for scope {:?}",
                capability_type, resource, action, scope
            )))
        }
    }

    pub fn get_capabilities_by_type(&self, capability_type: &CapabilityType) -> Vec<Capability> {
        self.capability_index
            .get(capability_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.capabilities.get(id).map(|c| c.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_capabilities_by_scope(&self, scope: &CapabilityScope) -> Vec<Capability> {
        self.scope_index
            .get(scope)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.capabilities.get(id).map(|c| c.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_capabilities_by_requester(&self, requester: &str) -> Vec<Capability> {
        self.requester_index
            .get(requester)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.capabilities.get(id).map(|c| c.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn list_all_capabilities(&self) -> Vec<Capability> {
        self.capabilities
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn list_active_capabilities(&self) -> Vec<Capability> {
        self.capabilities
            .iter()
            .filter(|entry| entry.value().is_active())
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn request_capability(&self, request: CapabilityRequest) -> Result<()> {
        info!(
            "Capability request: {} - Requester: {} - Type: {:?}",
            request.request_id, request.requester, request.capability_type
        );

        self.requests.insert(request.request_id.clone(), request);

        Ok(())
    }

    pub fn approve_request(&self, request_id: &str, approved_by: String) -> Result<Capability> {
        info!("Approving capability request: {}", request_id);

        if let Some(mut request) = self.requests.get_mut(request_id) {
            request.approve(approved_by.clone());

            let capability = Capability::new(
                uuid::Uuid::new_v4().to_string(),
                request.capability_type.clone(),
                request.scope.clone(),
                request.resources.clone(),
                request.actions.clone(),
                approved_by,
            );

            self.grant_capability(capability.clone())?;

            Ok(capability)
        } else {
            Err(AetherisError::NotFound(format!(
                "Capability request not found: {}",
                request_id
            )))
        }
    }

    pub fn deny_request(&self, request_id: &str, approved_by: String) -> Result<()> {
        info!("Denying capability request: {}", request_id);

        if let Some(mut request) = self.requests.get_mut(request_id) {
            request.deny(approved_by);
            Ok(())
        } else {
            Err(AetherisError::NotFound(format!(
                "Capability request not found: {}",
                request_id
            )))
        }
    }

    pub fn get_request(&self, request_id: &str) -> Option<CapabilityRequest> {
        self.requests.get(request_id).map(|r| r.value().clone())
    }

    pub fn list_pending_requests(&self) -> Vec<CapabilityRequest> {
        self.requests
            .iter()
            .filter(|entry| entry.value().is_pending())
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn capability_count(&self) -> usize {
        self.capabilities.len()
    }

    pub fn active_capability_count(&self) -> usize {
        self.list_active_capabilities().len()
    }
}

impl Default for CapabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_creation() {
        let capability = Capability::new(
            "test-cap".to_string(),
            CapabilityType::FileSystem,
            CapabilityScope::Global,
            vec!["/tmp/*".to_string()],
            vec!["read".to_string(), "write".to_string()],
            "system".to_string(),
        );

        assert_eq!(capability.id, "test-cap");
        assert_eq!(capability.capability_type, CapabilityType::FileSystem);
        assert!(capability.is_active());
        assert!(capability.can_access("/tmp/test.txt", "read"));
        assert!(!capability.can_access("/etc/passwd", "read"));
    }

    #[test]
    fn test_capability_manager() {
        let manager = CapabilityManager::new();

        let capability = Capability::new(
            "test-cap".to_string(),
            CapabilityType::FileSystem,
            CapabilityScope::Global,
            vec!["/tmp/*".to_string()],
            vec!["read".to_string()],
            "system".to_string(),
        );

        manager.grant_capability(capability).unwrap();

        assert_eq!(manager.capability_count(), 1);
        assert!(manager.has_capability(
            &CapabilityScope::Global,
            &CapabilityType::FileSystem,
            "/tmp/test.txt",
            "read"
        ));
    }

    #[test]
    fn test_capability_request() {
        let manager = CapabilityManager::new();

        let request = CapabilityRequest::new(
            "test-plugin".to_string(),
            CapabilityType::Network,
            CapabilityScope::Plugin("test-plugin".to_string()),
            vec!["api.example.com".to_string()],
            vec!["GET".to_string()],
            "Need to access API".to_string(),
        );

        manager.request_capability(request.clone()).unwrap();

        assert!(manager.get_request(&request.request_id).is_some());
        assert_eq!(manager.list_pending_requests().len(), 1);
    }
}
