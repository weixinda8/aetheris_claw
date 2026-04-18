use crate::utils::{AetherisError, Result};
use bollard::Docker;
use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};
use bollard::models::ContainerCreateBody;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};
use wasmtime::{Engine, Instance, Module, Store, Val};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPermission {
    pub allow_network: bool,
    pub allow_file_system: bool,
    pub allowed_paths: Vec<String>,
    pub allow_wasm_modules: Vec<String>,
    pub max_execution_time_ms: u64,
    pub max_memory_bytes: usize,
}

impl Default for SandboxPermission {
    fn default() -> Self {
        Self {
            allow_network: false,
            allow_file_system: false,
            allowed_paths: Vec::new(),
            allow_wasm_modules: Vec::new(),
            max_execution_time_ms: 30000,
            max_memory_bytes: 1024 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub max_memory: usize,
    pub timeout_seconds: u64,
    pub allow_network: bool,
    pub allow_file_system: bool,
    pub permissions: SandboxPermission,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_memory: 1024 * 1024 * 1024,
            timeout_seconds: 300,
            allow_network: false,
            allow_file_system: false,
            permissions: SandboxPermission::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmModule {
    pub id: String,
    pub name: String,
    pub version: String,
    pub bytes: Vec<u8>,
    pub entry_point: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl WasmModule {
    pub fn new(id: String, name: String, version: String, bytes: Vec<u8>) -> Self {
        Self {
            id,
            name,
            version,
            bytes,
            entry_point: "main".to_string(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn with_entry_point(mut self, entry_point: String) -> Self {
        self.entry_point = entry_point;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxExecutionResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub memory_used_bytes: usize,
    pub logs: Vec<String>,
}

pub struct WasmSandbox {
    config: SandboxConfig,
    modules: Arc<Mutex<HashMap<String, WasmModule>>>,
}

impl WasmSandbox {
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            modules: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(SandboxConfig::default())
    }

    pub async fn register_module(&self, module: WasmModule) -> Result<()> {
        info!(
            "Registering Wasm module: {} v{}",
            module.name, module.version
        );
        let mut modules = self.modules.lock().await;
        modules.insert(module.id.clone(), module);
        debug!("Wasm module registered successfully");
        Ok(())
    }

    pub async fn get_module(&self, id: &str) -> Result<Option<WasmModule>> {
        let modules = self.modules.lock().await;
        Ok(modules.get(id).cloned())
    }

    pub async fn list_modules(&self) -> Result<Vec<WasmModule>> {
        let modules = self.modules.lock().await;
        Ok(modules.values().cloned().collect())
    }

    pub async fn unregister_module(&self, id: &str) -> Result<()> {
        info!("Unregistering Wasm module: {}", id);
        let mut modules = self.modules.lock().await;
        modules.remove(id);
        Ok(())
    }

    pub async fn execute(&self, module_id: &str, input: &str) -> Result<SandboxExecutionResult> {
        let start = std::time::Instant::now();
        info!("Executing Wasm module: {}", module_id);

        let modules = self.modules.lock().await;
        let module = modules.get(module_id).ok_or_else(|| {
            AetherisError::Runtime(format!("Wasm module not found: {}", module_id))
        })?;

        let result = self.execute_module_internal(module, input).await;
        let duration = start.elapsed();

        let execution_result = match result {
            Ok(output) => SandboxExecutionResult {
                success: true,
                output: Some(output),
                error: None,
                execution_time_ms: duration.as_millis() as u64,
                memory_used_bytes: 0,
                logs: Vec::new(),
            },
            Err(e) => SandboxExecutionResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
                execution_time_ms: duration.as_millis() as u64,
                memory_used_bytes: 0,
                logs: Vec::new(),
            },
        };

        Ok(execution_result)
    }

    async fn execute_wasm_with_engine(
        &self,
        module_bytes: &[u8],
        entry_point: &str,
        _input: &str,
    ) -> Result<String> {
        let max_memory = self.config.permissions.max_memory_bytes;
        let timeout =
            std::time::Duration::from_millis(self.config.permissions.max_execution_time_ms);

        info!(
            "Starting Wasm execution with max memory: {} bytes",
            max_memory
        );

        let mut config = wasmtime::Config::new();
        config.async_support(true);
        let engine = Engine::new(&config)
            .map_err(|e| AetherisError::Runtime(format!("Failed to create Wasm engine: {}", e)))?;

        let module = Module::from_binary(&engine, module_bytes)
            .map_err(|e| AetherisError::Runtime(format!("Failed to load Wasm module: {}", e)))?;

        let result = tokio::time::timeout(timeout, async move {
            let mut store = Store::new(&engine, ());

            let instance = Instance::new(&mut store, &module, &[]).map_err(|e| {
                if e.to_string().to_lowercase().contains("out of memory")
                    || e.to_string().to_lowercase().contains("memory")
                {
                    AetherisError::Runtime(format!(
                        "Wasm module exceeded memory limit of {} bytes",
                        max_memory
                    ))
                } else {
                    AetherisError::Runtime(format!("Failed to instantiate Wasm module: {}", e))
                }
            })?;

            let func = instance
                .get_func(&mut store, entry_point)
                .or_else(|| instance.get_func(&mut store, "main"))
                .or_else(|| instance.get_func(&mut store, "run"))
                .ok_or_else(|| {
                    AetherisError::Runtime(format!(
                        "Failed to find entry function: {}",
                        entry_point
                    ))
                })?;

            let mut results = vec![Val::I32(0); func.ty(&store).results().len()];

            func.call(&mut store, &[], &mut results).map_err(|e| {
                if e.to_string().to_lowercase().contains("out of memory")
                    || e.to_string().to_lowercase().contains("memory")
                {
                    AetherisError::Runtime(format!(
                        "Wasm execution exceeded memory limit of {} bytes",
                        max_memory
                    ))
                } else {
                    AetherisError::Runtime(format!("Failed to execute Wasm function: {}", e))
                }
            })?;

            let output = if results.is_empty() {
                "Execution completed successfully".to_string()
            } else {
                format!("{:?}", results)
            };

            Ok(output)
        })
        .await;

        result.map_err(|_| AetherisError::Runtime("Wasm execution timed out".to_string()))?
    }

    async fn execute_module_internal(&self, module: &WasmModule, input: &str) -> Result<String> {
        info!(
            "Executing Wasm module: {} with entry point: {}",
            module.name, module.entry_point
        );
        self.execute_wasm_with_engine(&module.bytes, &module.entry_point, input)
            .await
    }

    pub async fn execute_bytes(
        &self,
        module_bytes: &[u8],
        input: &str,
    ) -> Result<SandboxExecutionResult> {
        let start = std::time::Instant::now();
        info!("Executing Wasm module from bytes");

        let result = self.execute_bytes_internal(module_bytes, input).await;
        let duration = start.elapsed();

        let execution_result = match result {
            Ok(output) => SandboxExecutionResult {
                success: true,
                output: Some(output),
                error: None,
                execution_time_ms: duration.as_millis() as u64,
                memory_used_bytes: 0,
                logs: Vec::new(),
            },
            Err(e) => SandboxExecutionResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
                execution_time_ms: duration.as_millis() as u64,
                memory_used_bytes: 0,
                logs: Vec::new(),
            },
        };

        Ok(execution_result)
    }

    async fn execute_bytes_internal(&self, module_bytes: &[u8], input: &str) -> Result<String> {
        info!("Executing Wasm module from bytes");
        self.execute_wasm_with_engine(module_bytes, "main", input)
            .await
    }

    pub fn check_permissions(&self, operation: &str) -> Result<bool> {
        match operation {
            "network" => Ok(self.config.permissions.allow_network),
            "file_system" => Ok(self.config.permissions.allow_file_system),
            _ => Ok(true),
        }
    }

    pub fn get_config(&self) -> SandboxConfig {
        self.config.clone()
    }

    pub fn set_config(&mut self, config: SandboxConfig) {
        self.config = config;
    }
}

impl Default for WasmSandbox {
    fn default() -> Self {
        Self::with_default_config()
    }
}

pub struct DockerSandbox {
    docker: Docker,
}

impl Default for DockerSandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl DockerSandbox {
    pub fn new() -> Self {
        let docker = Docker::connect_with_local_defaults().unwrap_or_else(|_| {
            panic!("Failed to connect to Docker daemon. Make sure Docker is running.")
        });
        Self { docker }
    }

    pub async fn try_new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| AetherisError::Runtime(format!("Failed to connect to Docker: {}", e)))?;
        Ok(Self { docker })
    }

    pub fn new_with_docker(docker: Docker) -> Self {
        Self { docker }
    }

    pub async fn connect_with_defaults() -> Result<Self> {
        Self::try_new().await
    }

    pub async fn health_check(&self) -> Result<bool> {
        self.docker
            .ping()
            .await
            .map(|_| true)
            .map_err(|e| AetherisError::Runtime(format!("Docker health check failed: {}", e)))
    }

    pub async fn get_version(&self) -> Result<String> {
        let version =
            self.docker.version().await.map_err(|e| {
                AetherisError::Runtime(format!("Failed to get Docker version: {}", e))
            })?;
        Ok(version.version.unwrap_or_else(|| "Unknown".to_string()))
    }

    pub fn get_docker_client(&self) -> &Docker {
        &self.docker
    }

    pub async fn pull_image(&self, image: &str) -> Result<()> {
        info!("Pulling Docker image: {}", image);

        let options = bollard::query_parameters::CreateImageOptions {
            from_image: Some(image.to_string()),
            ..Default::default()
        };

        let mut stream = self.docker.create_image(Some(options), None, None);

        while let Some(result) = stream
            .try_next()
            .await
            .map_err(|e| AetherisError::Runtime(format!("Failed to pull image: {}", e)))?
        {
            debug!("Pull image progress: {:?}", result);
        }

        info!("Docker image pulled successfully: {}", image);
        Ok(())
    }

    pub async fn create_container(&self, image: &str, command: Option<&str>) -> Result<String> {
        info!("Creating Docker container from image: {}", image);

        let cmd: Vec<String> = command
            .map(|c| c.split_whitespace().map(|s| s.to_string()).collect())
            .unwrap_or_else(|| {
                vec![
                    "/bin/sh".to_string(),
                    "-c".to_string(),
                    "tail -f /dev/null".to_string(),
                ]
            });

        let config = ContainerCreateBody {
            image: Some(image.to_string()),
            cmd: Some(cmd),
            tty: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let container = self
            .docker
            .create_container(
                None::<bollard::query_parameters::CreateContainerOptions>,
                config,
            )
            .await
            .map_err(|e| AetherisError::Runtime(format!("Failed to create container: {}", e)))?;

        info!("Docker container created with ID: {}", container.id);
        Ok(container.id)
    }

    pub async fn start_container(&self, container_id: &str) -> Result<()> {
        info!("Starting Docker container: {}", container_id);

        self.docker
            .start_container(
                container_id,
                None::<bollard::query_parameters::StartContainerOptions>,
            )
            .await
            .map_err(|e| AetherisError::Runtime(format!("Failed to start container: {}", e)))?;

        info!("Docker container started: {}", container_id);
        Ok(())
    }

    pub async fn stop_container(&self, container_id: &str) -> Result<()> {
        info!("Stopping Docker container: {}", container_id);

        self.docker
            .stop_container(
                container_id,
                None::<bollard::query_parameters::StopContainerOptions>,
            )
            .await
            .map_err(|e| AetherisError::Runtime(format!("Failed to stop container: {}", e)))?;

        info!("Docker container stopped: {}", container_id);
        Ok(())
    }

    pub async fn remove_container(&self, container_id: &str, force: bool) -> Result<()> {
        info!("Removing Docker container: {}", container_id);

        let options = if force {
            Some(bollard::query_parameters::RemoveContainerOptions {
                force: true,
                ..Default::default()
            })
        } else {
            None
        };

        self.docker
            .remove_container(container_id, options)
            .await
            .map_err(|e| AetherisError::Runtime(format!("Failed to remove container: {}", e)))?;

        info!("Docker container removed: {}", container_id);
        Ok(())
    }

    pub async fn exec_command(
        &self,
        container_id: &str,
        command: &str,
    ) -> Result<(String, String)> {
        info!(
            "Executing command in container {}: {}",
            container_id, command
        );

        let cmd: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();

        let exec_config = CreateExecOptions {
            cmd: Some(cmd),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let exec = self
            .docker
            .create_exec(container_id, exec_config)
            .await
            .map_err(|e| {
                AetherisError::Runtime(format!("Failed to create exec instance: {}", e))
            })?;

        let start_options = StartExecOptions {
            detach: false,
            tty: false,
            ..Default::default()
        };

        let result = self
            .docker
            .start_exec(&exec.id, Some(start_options))
            .await
            .map_err(|e| AetherisError::Runtime(format!("Failed to start exec: {}", e)))?;

        let (stdout, stderr) =
            match result {
                StartExecResults::Attached { mut output, .. } => {
                    let mut stdout = String::new();
                    let mut stderr = String::new();

                    while let Some(msg) = output.try_next().await.map_err(|e| {
                        AetherisError::Runtime(format!("Error reading output: {}", e))
                    })? {
                        match msg {
                            bollard::container::LogOutput::StdOut { message } => {
                                stdout.push_str(&String::from_utf8_lossy(&message));
                            }
                            bollard::container::LogOutput::StdErr { message } => {
                                stderr.push_str(&String::from_utf8_lossy(&message));
                            }
                            _ => {}
                        }
                    }

                    (stdout, stderr)
                }
                StartExecResults::Detached => (String::new(), String::new()),
            };

        info!(
            "Command executed successfully in container {}",
            container_id
        );
        Ok((stdout, stderr))
    }

    pub async fn get_container_logs(&self, container_id: &str) -> Result<(String, String)> {
        info!("Getting logs from container: {}", container_id);

        let options = bollard::query_parameters::LogsOptions {
            stdout: true,
            stderr: true,
            tail: "all".to_string(),
            ..Default::default()
        };

        let mut logs = self.docker.logs(container_id, Some(options));

        let mut stdout = String::new();
        let mut stderr = String::new();

        while let Some(msg) = logs
            .try_next()
            .await
            .map_err(|e| AetherisError::Runtime(format!("Error reading logs: {}", e)))?
        {
            match msg {
                bollard::container::LogOutput::StdOut { message } => {
                    stdout.push_str(&String::from_utf8_lossy(&message));
                }
                bollard::container::LogOutput::StdErr { message } => {
                    stderr.push_str(&String::from_utf8_lossy(&message));
                }
                _ => {}
            }
        }

        Ok((stdout, stderr))
    }

    pub async fn execute(&self, image: &str, command: &str) -> Result<SandboxExecutionResult> {
        let start = std::time::Instant::now();
        info!(
            "Executing in Docker sandbox: image={}, command={}",
            image, command
        );

        let mut container_id: Option<String> = None;

        let result: Result<(String, String)> = async {
            self.pull_image(image).await?;

            let cid = self.create_container(image, None).await?;
            container_id = Some(cid.clone());

            self.start_container(&cid).await?;

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let (stdout, stderr) = self.exec_command(&cid, command).await?;

            Ok((stdout, stderr))
        }
        .await;

        let duration = start.elapsed();

        if let Some(cid) = container_id {
            let _ = self.stop_container(&cid).await;
            let _ = self.remove_container(&cid, true).await;
        }

        let execution_result = match result {
            Ok((stdout, stderr)) => {
                let output = if !stdout.is_empty() {
                    Some(stdout.trim().to_string())
                } else {
                    None
                };

                let error = if !stderr.is_empty() {
                    Some(stderr.trim().to_string())
                } else {
                    None
                };

                SandboxExecutionResult {
                    success: error.is_none() || stderr.is_empty(),
                    output,
                    error,
                    execution_time_ms: duration.as_millis() as u64,
                    memory_used_bytes: 0,
                    logs: Vec::new(),
                }
            }
            Err(e) => SandboxExecutionResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
                execution_time_ms: duration.as_millis() as u64,
                memory_used_bytes: 0,
                logs: Vec::new(),
            },
        };

        Ok(execution_result)
    }
}
