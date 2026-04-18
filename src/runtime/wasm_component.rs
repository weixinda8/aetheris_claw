use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info};
use wasmtime::{Engine, Instance, Module, Store, Val};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ComponentLifecycleState {
    Unloaded,
    Loading,
    Loaded,
    Active,
    Unloading,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub license: Option<String>,
    pub component_type: ComponentType,
    pub entry_point: String,
    pub dependencies: Vec<String>,
    pub capabilities: Vec<String>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ComponentType {
    Skill,
    Agent,
    Memory,
    Security,
    Utility,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitInterface {
    pub name: String,
    pub functions: Vec<WitFunction>,
    pub types: Vec<WitType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitFunction {
    pub name: String,
    pub parameters: Vec<WitParameter>,
    pub return_type: Option<WitType>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitParameter {
    pub name: String,
    pub r#type: WitType,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WitType {
    String,
    Int32,
    Int64,
    Float32,
    Float64,
    Boolean,
    List(Box<WitType>),
    Option(Box<WitType>),
    Record(String),
    Enum(String, Vec<String>),
}

#[derive(Debug, Clone)]
pub struct ComponentInstance {
    pub metadata: ComponentMetadata,
    pub state: ComponentLifecycleState,
    pub bytes: Option<Vec<u8>>,
    pub path: Option<PathBuf>,
    pub loaded_at: Option<Instant>,
    pub last_accessed: Instant,
    pub access_count: u64,
    pub wit_interfaces: Vec<WitInterface>,
}

impl ComponentInstance {
    pub fn new(metadata: ComponentMetadata) -> Self {
        Self {
            metadata,
            state: ComponentLifecycleState::Unloaded,
            bytes: None,
            path: None,
            loaded_at: None,
            last_accessed: Instant::now(),
            access_count: 0,
            wit_interfaces: Vec::new(),
        }
    }

    pub fn with_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.bytes = Some(bytes);
        self
    }

    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    pub fn with_wit_interfaces(mut self, interfaces: Vec<WitInterface>) -> Self {
        self.wit_interfaces = interfaces;
        self
    }

    pub fn is_loaded(&self) -> bool {
        self.state == ComponentLifecycleState::Loaded
            || self.state == ComponentLifecycleState::Active
    }

    pub fn should_unload(&self, idle_timeout: Duration) -> bool {
        self.last_accessed.elapsed() > idle_timeout && self.state == ComponentLifecycleState::Loaded
    }

    pub fn access(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ComponentExecutionMode {
    Lazy,
    Eager,
    Pooled,
    Jit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentPoolConfig {
    pub min_instances: u32,
    pub max_instances: u32,
    pub idle_timeout: Duration,
    pub max_idle_instances: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentResourceLimits {
    pub max_memory_bytes: u64,
    pub max_execution_time_ms: u64,
    pub max_stack_bytes: u64,
    pub max_fuel: u64,
    pub max_instances_per_component: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInvocationResult {
    pub result: Option<serde_json::Value>,
    pub execution_time_ms: u64,
    pub memory_used_bytes: u64,
    pub fuel_consumed: u64,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetrics {
    pub component_id: String,
    pub total_invocations: u64,
    pub successful_invocations: u64,
    pub failed_invocations: u64,
    pub total_execution_time_ms: u64,
    pub avg_execution_time_ms: f64,
    pub min_execution_time_ms: u64,
    pub max_execution_time_ms: u64,
    pub peak_memory_bytes: u64,
    pub last_invoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct PooledComponentInstance {
    pub instance: ComponentInstance,
    pub acquired_at: Option<Instant>,
    pub last_used_at: Instant,
    pub is_available: bool,
}

pub struct EnhancedWasmComponentRuntime {
    components: Arc<DashMap<String, ComponentMetadata>>,
    instances: Arc<DashMap<String, ComponentInstance>>,
    pools: Arc<DashMap<String, Vec<PooledComponentInstance>>>,
    metrics: Arc<DashMap<String, ComponentMetrics>>,
    events: Arc<DashMap<String, Vec<ComponentEvent>>>,
    resource_limits: ComponentResourceLimits,
    pool_config: ComponentPoolConfig,
    execution_mode: ComponentExecutionMode,
    storage_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentEvent {
    pub event_id: String,
    pub component_id: String,
    pub event_type: ComponentEventType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComponentEventType {
    Loaded,
    Instantiated,
    Invoked,
    InvocationFailed,
    Unloaded,
    Error,
    Warning,
    ResourceLimitExceeded,
}

impl Default for ComponentPoolConfig {
    fn default() -> Self {
        Self {
            min_instances: 2,
            max_instances: 10,
            idle_timeout: Duration::from_secs(300),
            max_idle_instances: 5,
        }
    }
}

impl Default for ComponentResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 1024 * 1024 * 1024,
            max_execution_time_ms: 30000,
            max_stack_bytes: 1024 * 1024 * 8,
            max_fuel: 1_000_000_000,
            max_instances_per_component: 10,
        }
    }
}

impl EnhancedWasmComponentRuntime {
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;

        let runtime = Self {
            components: Arc::new(DashMap::new()),
            instances: Arc::new(DashMap::new()),
            pools: Arc::new(DashMap::new()),
            metrics: Arc::new(DashMap::new()),
            events: Arc::new(DashMap::new()),
            resource_limits: ComponentResourceLimits::default(),
            pool_config: ComponentPoolConfig::default(),
            execution_mode: ComponentExecutionMode::Lazy,
            storage_path,
        };

        runtime.load()?;
        Ok(runtime)
    }

    fn save(&self) -> Result<()> {
        let components_path = self.storage_path.join("components.json");
        let components: Vec<_> = self.components.iter().map(|e| e.value().clone()).collect();
        std::fs::write(&components_path, serde_json::to_string_pretty(&components)?)?;

        let metrics_path = self.storage_path.join("metrics.json");
        let metrics: Vec<_> = self.metrics.iter().map(|e| e.value().clone()).collect();
        std::fs::write(&metrics_path, serde_json::to_string_pretty(&metrics)?)?;

        let events_path = self.storage_path.join("events.json");
        let events_map: Vec<(String, Vec<ComponentEvent>)> = self
            .events
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect();
        std::fs::write(&events_path, serde_json::to_string_pretty(&events_map)?)?;

        Ok(())
    }

    fn load(&self) -> Result<()> {
        let components_path = self.storage_path.join("components.json");
        if components_path.exists() {
            let content = std::fs::read_to_string(&components_path)?;
            let components: Vec<ComponentMetadata> = serde_json::from_str(&content)?;
            for component in components {
                self.components.insert(component.id.clone(), component);
            }
        }

        let metrics_path = self.storage_path.join("metrics.json");
        if metrics_path.exists() {
            let content = std::fs::read_to_string(&metrics_path)?;
            let metrics: Vec<ComponentMetrics> = serde_json::from_str(&content)?;
            for metric in metrics {
                self.metrics.insert(metric.component_id.clone(), metric);
            }
        }

        let events_path = self.storage_path.join("events.json");
        if events_path.exists() {
            let content = std::fs::read_to_string(&events_path)?;
            let events_map: Vec<(String, Vec<ComponentEvent>)> = serde_json::from_str(&content)?;
            for (component_id, events) in events_map {
                self.events.insert(component_id, events);
            }
        }

        Ok(())
    }

    pub fn register_component(&self, metadata: ComponentMetadata) -> Result<()> {
        if self.components.contains_key(&metadata.id) {
            return Err(AetherisError::Validation(format!(
                "Component with ID '{}' already exists",
                metadata.id
            )));
        }

        info!(
            "Registering WASM component: {} ({})",
            metadata.name, metadata.id
        );

        self.components
            .insert(metadata.id.clone(), metadata.clone());

        self.metrics.insert(
            metadata.id.clone(),
            ComponentMetrics {
                component_id: metadata.id.clone(),
                total_invocations: 0,
                successful_invocations: 0,
                failed_invocations: 0,
                total_execution_time_ms: 0,
                avg_execution_time_ms: 0.0,
                min_execution_time_ms: u64::MAX,
                max_execution_time_ms: 0,
                peak_memory_bytes: 0,
                last_invoked_at: None,
            },
        );

        self.emit_event(&metadata.id, ComponentEventType::Loaded, None);
        self.save()?;

        Ok(())
    }

    pub fn get_component(&self, component_id: &str) -> Option<ComponentMetadata> {
        self.components.get(component_id).map(|c| c.value().clone())
    }

    pub fn list_components(&self, filter: Option<ComponentType>) -> Vec<ComponentMetadata> {
        self.components
            .iter()
            .filter(|entry| {
                if let Some(filter_type) = &filter {
                    entry.value().component_type == *filter_type
                } else {
                    true
                }
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub async fn instantiate_component(&self, component_id: &str) -> Result<ComponentInstance> {
        let metadata = self.components.get(component_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Component not found: {}", component_id))
        })?;

        info!("Instantiating component: {}", component_id);

        let instance_id = uuid::Uuid::new_v4().to_string();
        let now = Instant::now();

        let instance = ComponentInstance {
            metadata: metadata.value().clone(),
            state: ComponentLifecycleState::Loading,
            bytes: None,
            path: None,
            loaded_at: Some(now),
            last_accessed: now,
            access_count: 0,
            wit_interfaces: Vec::new(),
        };

        self.instances.insert(instance_id.clone(), instance.clone());
        self.emit_event(component_id, ComponentEventType::Instantiated, None);

        Ok(instance)
    }

    pub async fn invoke_function(
        &self,
        component_id: &str,
        function_name: &str,
        parameters: Vec<serde_json::Value>,
    ) -> Result<ComponentInvocationResult> {
        let start = Instant::now();

        info!(
            "Invoking function: {} on component: {}",
            function_name, component_id
        );

        let mut result_val = ComponentInvocationResult {
            result: None,
            execution_time_ms: 0,
            memory_used_bytes: 0,
            fuel_consumed: 0,
            success: false,
            error: None,
        };

        let mut failed = false;

        let invoke_result = self
            .invoke_function_internal(component_id, function_name, parameters)
            .await;

        let execution_time = start.elapsed().as_millis() as u64;
        result_val.execution_time_ms = execution_time;

        if let Some(mut metrics) = self.metrics.get_mut(component_id) {
            metrics.total_invocations += 1;
            metrics.last_invoked_at = Some(chrono::Utc::now());
            metrics.total_execution_time_ms += execution_time;
            metrics.avg_execution_time_ms =
                metrics.total_execution_time_ms as f64 / metrics.total_invocations as f64;
            metrics.min_execution_time_ms = metrics.min_execution_time_ms.min(execution_time);
            metrics.max_execution_time_ms = metrics.max_execution_time_ms.max(execution_time);
        }

        match invoke_result {
            Ok(value) => {
                result_val.success = true;
                result_val.result = Some(value);
                if let Some(mut metrics) = self.metrics.get_mut(component_id) {
                    metrics.successful_invocations += 1;
                }
                self.emit_event(component_id, ComponentEventType::Invoked, None);
            }
            Err(e) => {
                failed = true;
                result_val.error = Some(e.to_string());
                if let Some(mut metrics) = self.metrics.get_mut(component_id) {
                    metrics.failed_invocations += 1;
                }
                self.emit_event(component_id, ComponentEventType::InvocationFailed, None);
                error!("Component invocation failed: {}", e);
            }
        }

        self.save()?;

        Ok(result_val)
    }

    async fn invoke_function_internal(
        &self,
        component_id: &str,
        function_name: &str,
        parameters: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let instance = self.instances.get(component_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Component instance not found: {}", component_id))
        })?;

        let bytes = instance
            .bytes
            .as_ref()
            .ok_or_else(|| AetherisError::Runtime("Component not loaded with bytes".to_string()))?;

        let max_memory = self.resource_limits.max_memory_bytes as usize;
        let timeout = Duration::from_millis(self.resource_limits.max_execution_time_ms);

        info!(
            "Invoking function with max memory: {} bytes, component: {}",
            max_memory, component_id
        );

        let mut config = wasmtime::Config::new();
        config.async_support(true);
        let engine = Engine::new(&config)
            .map_err(|e| AetherisError::Runtime(format!("Failed to create Wasm engine: {}", e)))?;

        let module = Module::from_binary(&engine, bytes)
            .map_err(|e| AetherisError::Runtime(format!("Failed to load module: {}", e)))?;

        let result = tokio::time::timeout(timeout, async move {
            let mut store = Store::new(&engine, ());

            let instance = Instance::new(&mut store, &module, &[]).map_err(|e| {
                if e.to_string().to_lowercase().contains("out of memory")
                    || e.to_string().to_lowercase().contains("memory")
                {
                    AetherisError::Runtime(format!(
                        "Component exceeded memory limit of {} bytes",
                        max_memory
                    ))
                } else {
                    AetherisError::Runtime(format!("Failed to instantiate module: {}", e))
                }
            })?;

            let func = instance
                .get_func(&mut store, function_name)
                .ok_or_else(|| {
                    AetherisError::Runtime(format!("Function not found: {}", function_name))
                })?;

            let params: Vec<Val> = parameters
                .iter()
                .map(json_to_val)
                .collect::<Result<_>>()?;

            let mut results = vec![Val::I32(0); func.ty(&store).results().len()];

            func.call(&mut store, &params, &mut results).map_err(|e| {
                if e.to_string().to_lowercase().contains("out of memory")
                    || e.to_string().to_lowercase().contains("memory")
                {
                    AetherisError::Runtime(format!(
                        "Component execution exceeded memory limit of {} bytes",
                        max_memory
                    ))
                } else {
                    AetherisError::Runtime(format!("Function call failed: {}", e))
                }
            })?;

            let result_json = if results.len() == 1 {
                val_to_json(&results[0])
            } else {
                serde_json::Value::Array(results.iter().map(val_to_json).collect())
            };

            Ok(result_json)
        })
        .await;

        result.map_err(|_| AetherisError::Runtime("Component execution timed out".to_string()))?
    }

    pub async fn acquire_from_pool(&self, component_id: &str) -> Result<ComponentInstance> {
        let _metadata = self.components.get(component_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Component not found: {}", component_id))
        })?;

        debug!("Acquiring component from pool: {}", component_id);

        if let Some(mut pool) = self.pools.get_mut(component_id) {
            for pooled in pool.iter_mut() {
                if pooled.is_available {
                    pooled.is_available = false;
                    pooled.acquired_at = Some(Instant::now());
                    pooled.last_used_at = Instant::now();
                    return Ok(pooled.instance.clone());
                }
            }
        }

        let instance = self.instantiate_component(component_id).await?;

        let pooled = PooledComponentInstance {
            instance: instance.clone(),
            acquired_at: Some(Instant::now()),
            last_used_at: Instant::now(),
            is_available: false,
        };

        self.pools
            .entry(component_id.to_string())
            .or_default()
            .push(pooled);

        Ok(instance)
    }

    pub async fn release_to_pool(&self, component_id: &str, _instance_id: &str) -> Result<()> {
        if let Some(mut pool) = self.pools.get_mut(component_id) {
            for pooled in pool.iter_mut() {
                if pooled.instance.metadata.id == component_id {
                    pooled.is_available = true;
                    pooled.acquired_at = None;
                    pooled.last_used_at = Instant::now();
                    debug!("Released component to pool: {}", component_id);
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    pub async fn warmup_pool(&self, component_id: &str) -> Result<()> {
        let _metadata = self.components.get(component_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Component not found: {}", component_id))
        })?;

        info!("Warming up pool for component: {}", component_id);

        let current_count = self
            .pools
            .get(component_id)
            .map(|p| p.value().len())
            .unwrap_or(0);

        for _ in current_count..self.pool_config.min_instances as usize {
            let instance = self.instantiate_component(component_id).await?;
            let pooled = PooledComponentInstance {
                instance,
                acquired_at: None,
                last_used_at: Instant::now(),
                is_available: true,
            };

            self.pools
                .entry(component_id.to_string())
                .or_default()
                .push(pooled);
        }

        Ok(())
    }

    pub async fn cleanup_pool(&self, component_id: &str) -> Result<usize> {
        let mut cleaned = 0;
        let now = Instant::now();

        if let Some(mut pool) = self.pools.get_mut(component_id) {
            let original_len = pool.len();
            pool.retain(|pooled| {
                if pooled.is_available
                    && now.duration_since(pooled.last_used_at) > self.pool_config.idle_timeout
                {
                    cleaned += 1;
                    false
                } else {
                    true
                }
            });

            if pool.len() > self.pool_config.max_instances as usize {
                pool.truncate(self.pool_config.max_instances as usize);
                cleaned += original_len - pool.len();
            }
        }

        if cleaned > 0 {
            info!(
                "Cleaned up {} instances from pool: {}",
                cleaned, component_id
            );
        }

        Ok(cleaned)
    }

    pub fn get_component_metrics(&self, component_id: &str) -> Option<ComponentMetrics> {
        self.metrics.get(component_id).map(|m| m.value().clone())
    }

    pub fn get_all_metrics(&self) -> Vec<ComponentMetrics> {
        self.metrics.iter().map(|e| e.value().clone()).collect()
    }

    pub fn get_component_events(
        &self,
        component_id: &str,
        limit: Option<usize>,
    ) -> Vec<ComponentEvent> {
        let mut events = self
            .events
            .get(component_id)
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
        component_id: &str,
        event_type: ComponentEventType,
        details: Option<serde_json::Value>,
    ) {
        let event = ComponentEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            component_id: component_id.to_string(),
            event_type,
            timestamp: chrono::Utc::now(),
            details,
        };

        self.events
            .entry(component_id.to_string())
            .or_default()
            .push(event);
    }

    pub fn set_resource_limits(&mut self, limits: ComponentResourceLimits) {
        self.resource_limits = limits;
    }

    pub fn set_pool_config(&mut self, config: ComponentPoolConfig) {
        self.pool_config = config;
    }

    pub fn set_execution_mode(&mut self, mode: ComponentExecutionMode) {
        self.execution_mode = mode;
    }

    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    pub fn pool_size(&self, component_id: &str) -> usize {
        self.pools
            .get(component_id)
            .map(|p| p.value().len())
            .unwrap_or(0)
    }
}

impl Default for EnhancedWasmComponentRuntime {
    fn default() -> Self {
        let storage_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".aetheris")
            .join("wasm-components");

        Self::new(storage_path).unwrap_or_else(|_| {
            let temp_dir = tempfile::tempdir().unwrap();
            Self::new(temp_dir.path().to_path_buf()).unwrap()
        })
    }
}

fn json_to_val(value: &serde_json::Value) -> Result<Val> {
    match value {
        serde_json::Value::Null => Ok(Val::I32(0)),
        serde_json::Value::Bool(b) => Ok(Val::I32(if *b { 1 } else { 0 })),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Val::I64(i))
            } else if let Some(f) = n.as_f64() {
                Ok(Val::F64(f.to_bits()))
            } else {
                Err(AetherisError::Validation(
                    "Unsupported number type".to_string(),
                ))
            }
        }
        serde_json::Value::String(_s) => Err(AetherisError::Validation(
            "String parameters not directly supported in Core Wasm".to_string(),
        )),
        serde_json::Value::Array(_) => Err(AetherisError::Validation(
            "Array parameters not supported".to_string(),
        )),
        serde_json::Value::Object(_) => Err(AetherisError::Validation(
            "Object parameters not supported".to_string(),
        )),
    }
}

fn val_to_json(val: &Val) -> serde_json::Value {
    match val {
        Val::I32(i) => serde_json::json!(i),
        Val::I64(i) => serde_json::json!(i),
        Val::F32(f) => serde_json::json!(f32::from_bits(*f)),
        Val::F64(f) => serde_json::json!(f64::from_bits(*f)),
        Val::V128(_) => serde_json::json!("V128"),
        Val::FuncRef(_) => serde_json::Value::Null,
        Val::ExternRef(_) => serde_json::Value::Null,
        Val::AnyRef(_) => serde_json::Value::Null,
        Val::ExnRef(_) => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_runtime_new() {
        let temp_dir = tempfile::tempdir().unwrap();
        let runtime = EnhancedWasmComponentRuntime::new(temp_dir.path().to_path_buf());
        assert!(runtime.is_ok());
    }
}
