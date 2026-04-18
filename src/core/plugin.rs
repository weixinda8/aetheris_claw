use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PluginType {
    Skill,
    Agent,
    Memory,
    Security,
    Observability,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub plugin_type: PluginType,
    pub author: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub dependencies: Vec<String>,
    pub capabilities: Vec<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub enabled: bool,
    pub deprecated: bool,
    pub deprecation_message: Option<String>,
}

impl PluginMetadata {
    pub fn new(
        id: String,
        name: String,
        version: String,
        description: String,
        plugin_type: PluginType,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name,
            version,
            description,
            plugin_type,
            author: None,
            license: None,
            tags: Vec::new(),
            categories: Vec::new(),
            dependencies: Vec::new(),
            capabilities: Vec::new(),
            homepage: None,
            repository: None,
            created_at: now,
            updated_at: now,
            enabled: true,
            deprecated: false,
            deprecation_message: None,
        }
    }

    pub fn with_author(mut self, author: String) -> Self {
        self.author = Some(author);
        self
    }

    pub fn with_license(mut self, license: String) -> Self {
        self.license = Some(license);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_categories(mut self, categories: Vec<String>) -> Self {
        self.categories = categories;
        self
    }

    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_homepage(mut self, homepage: String) -> Self {
        self.homepage = Some(homepage);
        self
    }

    pub fn with_repository(mut self, repository: String) -> Self {
        self.repository = Some(repository);
        self
    }

    pub fn is_compatible(&self) -> bool {
        self.enabled && !self.deprecated
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PluginLifecycleState {
    Unloaded,
    Loading,
    Loaded,
    Initializing,
    Active,
    Deactivating,
    Unloading,
    Error,
    Disabled,
    Deprecated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub plugin_id: String,
    pub version_constraint: String,
    pub required: bool,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapability {
    pub capability_id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub interfaces: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstance {
    pub metadata: PluginMetadata,
    pub state: PluginLifecycleState,
    pub instance_id: String,
    pub loaded_at: Option<chrono::DateTime<chrono::Utc>>,
    pub initialized_at: Option<chrono::DateTime<chrono::Utc>>,
    pub activated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub access_count: u64,
    pub dependencies: Vec<PluginDependency>,
    pub capabilities: Vec<PluginCapability>,
    pub config: Option<serde_json::Value>,
    pub error: Option<String>,
    pub error_count: u32,
    pub warnings: Vec<String>,
    pub performance_metrics: PluginPerformanceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPerformanceMetrics {
    pub load_time_ms: u64,
    pub init_time_ms: u64,
    pub avg_execution_time_ms: f64,
    pub total_executions: u64,
    pub total_execution_time_ms: u64,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEvent {
    pub event_id: String,
    pub plugin_id: String,
    pub event_type: PluginEventType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginEventType {
    Loading,
    Loaded,
    Initialized,
    Activated,
    Deactivated,
    Unloading,
    Unloaded,
    Error,
    Warning,
    DependencyResolved,
    DependencyFailed,
    CapabilityRegistered,
    CapabilityUnregistered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealthCheck {
    pub plugin_id: String,
    pub check_id: String,
    pub check_name: String,
    pub status: HealthCheckStatus,
    pub message: Option<String>,
    pub checked_at: chrono::DateTime<chrono::Utc>,
    pub response_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthCheckStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

pub struct EnhancedPluginRegistry {
    plugins: Arc<DashMap<String, PluginInstance>>,
    events: Arc<DashMap<String, Vec<PluginEvent>>>,
    health_checks: Arc<DashMap<String, Vec<PluginHealthCheck>>>,
    capability_index: Arc<DashMap<String, Vec<String>>>,
    type_index: Arc<DashMap<PluginType, Vec<String>>>,
    tag_index: Arc<DashMap<String, Vec<String>>>,
    dependency_graph: Arc<DashMap<String, Vec<String>>>,
    reverse_dependency_graph: Arc<DashMap<String, Vec<String>>>,
    storage_path: PathBuf,
}

impl EnhancedPluginRegistry {
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;

        let instance = Self {
            plugins: Arc::new(DashMap::new()),
            events: Arc::new(DashMap::new()),
            health_checks: Arc::new(DashMap::new()),
            capability_index: Arc::new(DashMap::new()),
            type_index: Arc::new(DashMap::new()),
            tag_index: Arc::new(DashMap::new()),
            dependency_graph: Arc::new(DashMap::new()),
            reverse_dependency_graph: Arc::new(DashMap::new()),
            storage_path,
        };

        instance.load()?;

        Ok(instance)
    }

    pub fn save(&self) -> Result<()> {
        let plugins_path = self.storage_path.join("plugins.json");
        let events_path = self.storage_path.join("events.json");
        let health_checks_path = self.storage_path.join("health_checks.json");
        let capability_index_path = self.storage_path.join("capability_index.json");
        let type_index_path = self.storage_path.join("type_index.json");
        let tag_index_path = self.storage_path.join("tag_index.json");
        let dependency_graph_path = self.storage_path.join("dependency_graph.json");
        let reverse_dependency_graph_path = self.storage_path.join("reverse_dependency_graph.json");

        let plugins: Vec<PluginInstance> = self.plugins.iter().map(|p| p.value().clone()).collect();
        let events: Vec<(String, Vec<PluginEvent>)> = self
            .events
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        let health_checks: Vec<(String, Vec<PluginHealthCheck>)> = self
            .health_checks
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        let capability_index: Vec<(String, Vec<String>)> = self
            .capability_index
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        let type_index: Vec<(PluginType, Vec<String>)> = self
            .type_index
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        let tag_index: Vec<(String, Vec<String>)> = self
            .tag_index
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        let dependency_graph: Vec<(String, Vec<String>)> = self
            .dependency_graph
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        let reverse_dependency_graph: Vec<(String, Vec<String>)> = self
            .reverse_dependency_graph
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        std::fs::write(plugins_path, serde_json::to_string_pretty(&plugins)?)?;
        std::fs::write(events_path, serde_json::to_string_pretty(&events)?)?;
        std::fs::write(
            health_checks_path,
            serde_json::to_string_pretty(&health_checks)?,
        )?;
        std::fs::write(
            capability_index_path,
            serde_json::to_string_pretty(&capability_index)?,
        )?;
        std::fs::write(type_index_path, serde_json::to_string_pretty(&type_index)?)?;
        std::fs::write(tag_index_path, serde_json::to_string_pretty(&tag_index)?)?;
        std::fs::write(
            dependency_graph_path,
            serde_json::to_string_pretty(&dependency_graph)?,
        )?;
        std::fs::write(
            reverse_dependency_graph_path,
            serde_json::to_string_pretty(&reverse_dependency_graph)?,
        )?;

        info!("EnhancedPluginRegistry saved to: {:?}", self.storage_path);

        Ok(())
    }

    fn load(&self) -> Result<()> {
        let plugins_path = self.storage_path.join("plugins.json");
        let events_path = self.storage_path.join("events.json");
        let health_checks_path = self.storage_path.join("health_checks.json");
        let capability_index_path = self.storage_path.join("capability_index.json");
        let type_index_path = self.storage_path.join("type_index.json");
        let tag_index_path = self.storage_path.join("tag_index.json");
        let dependency_graph_path = self.storage_path.join("dependency_graph.json");
        let reverse_dependency_graph_path = self.storage_path.join("reverse_dependency_graph.json");

        if plugins_path.exists() {
            let content = std::fs::read_to_string(plugins_path)?;
            let plugins: Vec<PluginInstance> = serde_json::from_str(&content)?;
            for plugin in plugins {
                self.plugins.insert(plugin.metadata.id.clone(), plugin);
            }
        }

        if events_path.exists() {
            let content = std::fs::read_to_string(events_path)?;
            let events: Vec<(String, Vec<PluginEvent>)> = serde_json::from_str(&content)?;
            for (plugin_id, plugin_events) in events {
                self.events.insert(plugin_id, plugin_events);
            }
        }

        if health_checks_path.exists() {
            let content = std::fs::read_to_string(health_checks_path)?;
            let health_checks: Vec<(String, Vec<PluginHealthCheck>)> =
                serde_json::from_str(&content)?;
            for (plugin_id, checks) in health_checks {
                self.health_checks.insert(plugin_id, checks);
            }
        }

        if capability_index_path.exists() {
            let content = std::fs::read_to_string(capability_index_path)?;
            let capability_index: Vec<(String, Vec<String>)> = serde_json::from_str(&content)?;
            for (capability_id, plugin_ids) in capability_index {
                self.capability_index.insert(capability_id, plugin_ids);
            }
        }

        if type_index_path.exists() {
            let content = std::fs::read_to_string(type_index_path)?;
            let type_index: Vec<(PluginType, Vec<String>)> = serde_json::from_str(&content)?;
            for (plugin_type, plugin_ids) in type_index {
                self.type_index.insert(plugin_type, plugin_ids);
            }
        }

        if tag_index_path.exists() {
            let content = std::fs::read_to_string(tag_index_path)?;
            let tag_index: Vec<(String, Vec<String>)> = serde_json::from_str(&content)?;
            for (tag, plugin_ids) in tag_index {
                self.tag_index.insert(tag, plugin_ids);
            }
        }

        if dependency_graph_path.exists() {
            let content = std::fs::read_to_string(dependency_graph_path)?;
            let dependency_graph: Vec<(String, Vec<String>)> = serde_json::from_str(&content)?;
            for (plugin_id, dependencies) in dependency_graph {
                self.dependency_graph.insert(plugin_id, dependencies);
            }
        }

        if reverse_dependency_graph_path.exists() {
            let content = std::fs::read_to_string(reverse_dependency_graph_path)?;
            let reverse_dependency_graph: Vec<(String, Vec<String>)> =
                serde_json::from_str(&content)?;
            for (plugin_id, dependents) in reverse_dependency_graph {
                self.reverse_dependency_graph.insert(plugin_id, dependents);
            }
        }

        info!(
            "EnhancedPluginRegistry loaded from: {:?}",
            self.storage_path
        );

        Ok(())
    }

    pub fn register_plugin(
        &self,
        metadata: PluginMetadata,
        config: Option<serde_json::Value>,
    ) -> Result<PluginInstance> {
        info!("Registering plugin: {} ({})", metadata.name, metadata.id);

        if self.plugins.contains_key(&metadata.id) {
            return Err(AetherisError::Validation(format!(
                "Plugin with ID '{}' already exists",
                metadata.id
            )));
        }

        let instance_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let instance = PluginInstance {
            metadata,
            state: PluginLifecycleState::Unloaded,
            instance_id,
            loaded_at: None,
            initialized_at: None,
            activated_at: None,
            last_accessed: now,
            access_count: 0,
            dependencies: Vec::new(),
            capabilities: Vec::new(),
            config,
            error: None,
            error_count: 0,
            warnings: Vec::new(),
            performance_metrics: PluginPerformanceMetrics {
                load_time_ms: 0,
                init_time_ms: 0,
                avg_execution_time_ms: 0.0,
                total_executions: 0,
                total_execution_time_ms: 0,
                memory_usage_bytes: 0,
                cpu_usage_percent: 0.0,
            },
        };

        self.plugins
            .insert(instance.metadata.id.clone(), instance.clone());
        self.update_indices(&instance);
        self.emit_event(&instance.metadata.id, PluginEventType::Loaded, None);

        self.save()?;

        Ok(instance)
    }

    fn update_indices(&self, instance: &PluginInstance) {
        self.type_index
            .entry(instance.metadata.plugin_type.clone())
            .or_default()
            .push(instance.metadata.id.clone());

        for tag in &instance.metadata.tags {
            self.tag_index
                .entry(tag.clone())
                .or_default()
                .push(instance.metadata.id.clone());
        }

        for capability in &instance.capabilities {
            self.capability_index
                .entry(capability.capability_id.clone())
                .or_default()
                .push(instance.metadata.id.clone());
        }

        for dep in &instance.dependencies {
            self.dependency_graph
                .entry(instance.metadata.id.clone())
                .or_default()
                .push(dep.plugin_id.clone());

            self.reverse_dependency_graph
                .entry(dep.plugin_id.clone())
                .or_default()
                .push(instance.metadata.id.clone());
        }
    }

    pub fn get_plugin(&self, plugin_id: &str) -> Option<PluginInstance> {
        self.plugins.get(plugin_id).map(|p| p.value().clone())
    }

    pub async fn load_plugin(&self, plugin_id: &str) -> Result<PluginInstance> {
        let mut instance = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Plugin not found: {}", plugin_id)))?;

        if instance.state == PluginLifecycleState::Loaded
            || instance.state == PluginLifecycleState::Active
        {
            return Ok(instance.value().clone());
        }

        info!("Loading plugin: {}", plugin_id);
        let start = chrono::Utc::now();

        instance.state = PluginLifecycleState::Loading;
        self.emit_event(plugin_id, PluginEventType::Loading, None);

        self.resolve_dependencies(plugin_id)?;

        let load_time = (chrono::Utc::now() - start).num_milliseconds() as u64;
        instance.performance_metrics.load_time_ms = load_time;
        instance.loaded_at = Some(chrono::Utc::now());
        instance.state = PluginLifecycleState::Loaded;

        self.emit_event(plugin_id, PluginEventType::Loaded, None);

        self.save()?;

        Ok(instance.value().clone())
    }

    fn resolve_dependencies(&self, plugin_id: &str) -> Result<()> {
        let instance = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Plugin not found: {}", plugin_id)))?;

        for dep in &instance.dependencies {
            if dep.required {
                if let Some(dep_instance) = self.plugins.get(&dep.plugin_id) {
                    if !dep_instance.metadata.enabled {
                        return Err(AetherisError::Validation(format!(
                            "Required dependency '{}' is disabled",
                            dep.plugin_id
                        )));
                    }
                } else {
                    return Err(AetherisError::Validation(format!(
                        "Required dependency '{}' not found",
                        dep.plugin_id
                    )));
                }
            }
        }

        Ok(())
    }

    pub async fn initialize_plugin(&self, plugin_id: &str) -> Result<PluginInstance> {
        let mut instance = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Plugin not found: {}", plugin_id)))?;

        if instance.state != PluginLifecycleState::Loaded {
            return Err(AetherisError::Validation(format!(
                "Plugin '{}' must be loaded before initialization",
                plugin_id
            )));
        }

        info!("Initializing plugin: {}", plugin_id);
        let start = chrono::Utc::now();

        instance.state = PluginLifecycleState::Initializing;

        let init_time = (chrono::Utc::now() - start).num_milliseconds() as u64;
        instance.performance_metrics.init_time_ms = init_time;
        instance.initialized_at = Some(chrono::Utc::now());
        instance.state = PluginLifecycleState::Loaded;

        self.emit_event(plugin_id, PluginEventType::Initialized, None);

        self.save()?;

        Ok(instance.value().clone())
    }

    pub async fn activate_plugin(&self, plugin_id: &str) -> Result<PluginInstance> {
        let mut instance = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Plugin not found: {}", plugin_id)))?;

        if instance.state == PluginLifecycleState::Active {
            return Ok(instance.value().clone());
        }

        if instance.state != PluginLifecycleState::Loaded {
            return Err(AetherisError::Validation(format!(
                "Plugin '{}' must be loaded before activation",
                plugin_id
            )));
        }

        info!("Activating plugin: {}", plugin_id);

        instance.state = PluginLifecycleState::Active;
        instance.initialized_at = Some(chrono::Utc::now());

        self.emit_event(plugin_id, PluginEventType::Activated, None);

        self.save()?;

        Ok(instance.value().clone())
    }

    pub async fn deactivate_plugin(&self, plugin_id: &str) -> Result<PluginInstance> {
        let mut instance = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Plugin not found: {}", plugin_id)))?;

        if instance.state != PluginLifecycleState::Active {
            return Ok(instance.value().clone());
        }

        info!("Deactivating plugin: {}", plugin_id);

        instance.state = PluginLifecycleState::Deactivating;
        self.emit_event(plugin_id, PluginEventType::Deactivated, None);

        instance.state = PluginLifecycleState::Loaded;

        self.save()?;

        Ok(instance.value().clone())
    }

    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut instance = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Plugin not found: {}", plugin_id)))?;

        if instance.state == PluginLifecycleState::Unloaded {
            return Ok(());
        }

        let dependents = self.get_dependent_plugins(plugin_id);
        if !dependents.is_empty() {
            return Err(AetherisError::Validation(format!(
                "Cannot unload plugin '{}', it has {} dependent plugins",
                plugin_id,
                dependents.len()
            )));
        }

        info!("Unloading plugin: {}", plugin_id);

        instance.state = PluginLifecycleState::Unloading;
        self.emit_event(plugin_id, PluginEventType::Unloading, None);

        instance.state = PluginLifecycleState::Unloaded;
        instance.loaded_at = None;
        instance.initialized_at = None;
        instance.activated_at = None;

        self.emit_event(plugin_id, PluginEventType::Unloaded, None);

        self.save()?;

        Ok(())
    }

    pub fn get_dependent_plugins(&self, plugin_id: &str) -> Vec<String> {
        self.reverse_dependency_graph
            .get(plugin_id)
            .map(|deps| deps.value().clone())
            .unwrap_or_default()
    }

    pub fn list_plugins(
        &self,
        filter: Option<PluginType>,
        include_disabled: bool,
    ) -> Vec<PluginInstance> {
        self.plugins
            .iter()
            .filter(|entry| {
                let instance = entry.value();
                if !include_disabled && !instance.metadata.enabled {
                    return false;
                }
                if let Some(filter_type) = &filter {
                    if instance.metadata.plugin_type != *filter_type {
                        return false;
                    }
                }
                true
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_plugins_by_type(&self, plugin_type: &PluginType) -> Vec<PluginInstance> {
        self.type_index
            .get(plugin_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_plugin(id))
                    .filter(|p| p.metadata.enabled)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_plugins_by_tag(&self, tag: &str) -> Vec<PluginInstance> {
        self.tag_index
            .get(tag)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_plugin(id))
                    .filter(|p| p.metadata.enabled)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_plugins_by_capability(&self, capability_id: &str) -> Vec<PluginInstance> {
        self.capability_index
            .get(capability_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_plugin(id))
                    .filter(|p| p.metadata.enabled && p.state == PluginLifecycleState::Active)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn perform_health_check(&self, plugin_id: &str) -> Result<PluginHealthCheck> {
        let instance = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Plugin not found: {}", plugin_id)))?;

        let start = chrono::Utc::now();
        let mut status = HealthCheckStatus::Healthy;
        let mut message = None;

        if instance.state == PluginLifecycleState::Error {
            status = HealthCheckStatus::Unhealthy;
            message = instance.error.clone();
        } else if instance.state != PluginLifecycleState::Active {
            status = HealthCheckStatus::Degraded;
            message = Some(format!("Plugin is in state: {:?}", instance.state));
        }

        let health_check = PluginHealthCheck {
            plugin_id: plugin_id.to_string(),
            check_id: uuid::Uuid::new_v4().to_string(),
            check_name: "basic_health_check".to_string(),
            status,
            message,
            checked_at: chrono::Utc::now(),
            response_time_ms: (chrono::Utc::now() - start).num_milliseconds() as u64,
        };

        self.health_checks
            .entry(plugin_id.to_string())
            .or_default()
            .push(health_check.clone());

        self.save()?;

        Ok(health_check)
    }

    pub fn get_plugin_events(&self, plugin_id: &str, limit: Option<usize>) -> Vec<PluginEvent> {
        let mut events = self
            .events
            .get(plugin_id)
            .map(|e| e.value().clone())
            .unwrap_or_default();

        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        if let Some(limit) = limit {
            events.truncate(limit);
        }

        events
    }

    fn emit_event(
        &self,
        plugin_id: &str,
        event_type: PluginEventType,
        details: Option<serde_json::Value>,
    ) {
        let event = PluginEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            plugin_id: plugin_id.to_string(),
            event_type,
            timestamp: chrono::Utc::now(),
            details,
        };

        self.events
            .entry(plugin_id.to_string())
            .or_default()
            .push(event);
    }

    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    pub fn active_plugin_count(&self) -> usize {
        self.plugins
            .iter()
            .filter(|entry| entry.value().state == PluginLifecycleState::Active)
            .count()
    }
}

impl Default for EnhancedPluginRegistry {
    fn default() -> Self {
        let storage_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".aetheris")
            .join("plugins");

        Self::new(storage_path).unwrap_or_else(|_| {
            let temp_dir = tempfile::tempdir().unwrap();
            Self::new(temp_dir.path().to_path_buf()).unwrap()
        })
    }
}

impl Default for PluginPerformanceMetrics {
    fn default() -> Self {
        Self {
            load_time_ms: 0,
            init_time_ms: 0,
            avg_execution_time_ms: 0.0,
            total_executions: 0,
            total_execution_time_ms: 0,
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_registry_new() {
        let temp_dir = tempfile::tempdir().unwrap();
        let registry = EnhancedPluginRegistry::new(temp_dir.path().to_path_buf());
        assert!(registry.is_ok());
    }

    #[test]
    fn test_register_plugin() {
        let temp_dir = tempfile::tempdir().unwrap();
        let registry = EnhancedPluginRegistry::new(temp_dir.path().to_path_buf()).unwrap();

        let metadata = PluginMetadata::new(
            "test-plugin".to_string(),
            "Test Plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
            PluginType::Skill,
        );

        let result = registry.register_plugin(metadata, None);
        assert!(result.is_ok());
        assert_eq!(registry.plugin_count(), 1);
    }
}
