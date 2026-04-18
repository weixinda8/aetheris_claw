pub mod atomic;
pub mod cpu_gpu_scheduler;
pub mod executor;
pub mod sandbox;
pub mod wasm_component;

use crate::core::Task;
use crate::utils::{AetherisError, Result};
use atomic::AtomicExecutor;
use cpu_gpu_scheduler::{ComputeDevice, ComputeDeviceType, CpuGpuScheduler, TaskRequirements};
use executor::TaskExecutor;
use sandbox::{SandboxConfig, WasmSandbox};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

#[derive(Clone)]
pub struct RuntimeConfig {
    pub max_concurrent_tasks: usize,
    pub enable_sandbox: bool,
    pub sandbox_config: SandboxConfig,
    pub auto_register_local_devices: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 100,
            enable_sandbox: true,
            sandbox_config: SandboxConfig::default(),
            auto_register_local_devices: true,
        }
    }
}

pub struct ExecutionRuntime {
    config: RuntimeConfig,
    task_executor: Arc<TaskExecutor>,
    atomic_executor: Arc<AtomicExecutor>,
    wasm_sandbox: Arc<WasmSandbox>,
    cpu_gpu_scheduler: Arc<CpuGpuScheduler>,
    is_initialized: Arc<Mutex<bool>>,
}

impl ExecutionRuntime {
    pub fn new(config: RuntimeConfig) -> Self {
        Self {
            config: config.clone(),
            task_executor: Arc::new(TaskExecutor::new(config.max_concurrent_tasks)),
            atomic_executor: Arc::new(AtomicExecutor::default()),
            wasm_sandbox: Arc::new(WasmSandbox::new(config.sandbox_config)),
            cpu_gpu_scheduler: Arc::new(CpuGpuScheduler::default()),
            is_initialized: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        let mut initialized = self.is_initialized.lock().await;
        if *initialized {
            debug!("Execution runtime already initialized");
            return Ok(());
        }

        info!("Initializing execution runtime");

        if self.config.auto_register_local_devices {
            self.register_local_devices().await?;
        }

        *initialized = true;
        info!("Execution runtime initialized successfully");
        Ok(())
    }

    async fn register_local_devices(&self) -> Result<()> {
        info!("Registering local compute devices");

        let cpu_device = ComputeDevice::new(
            "cpu-local-0".to_string(),
            ComputeDeviceType::Cpu,
            "Generic".to_string(),
            "CPU".to_string(),
            16 * 1024 * 1024 * 1024,
            num_cpus::get(),
        );
        self.cpu_gpu_scheduler.register_device(cpu_device)?;

        Ok(())
    }

    pub async fn execute(&self, task: Task) -> Result<Task> {
        info!("Executing task: {}", task.id);

        self.ensure_initialized().await?;

        let requirements = TaskRequirements::new(task.id.clone(), 1024 * 1024 * 1024);

        let _decision = self.cpu_gpu_scheduler.schedule_task(requirements).await?;

        self.atomic_executor.start_execution(&task, 1, 3)?;
        self.atomic_executor.create_snapshot(&task).await?;

        let result = self.task_executor.execute(task.clone()).await;

        match &result {
            Ok(task) => {
                self.atomic_executor.complete_execution(&task.id).await?;
            }
            Err(e) => {
                self.atomic_executor
                    .fail_execution(&task.id, e.to_string())
                    .await?;
            }
        }

        self.cpu_gpu_scheduler.complete_task(&task.id).await?;

        result
    }

    pub async fn execute_with_sandbox(&self, task: Task, module_id: &str) -> Result<Task> {
        info!("Executing task with sandbox: {}", task.id);

        self.ensure_initialized().await?;

        if !self.config.enable_sandbox {
            return Err(AetherisError::Runtime(
                "Sandbox execution is disabled".to_string(),
            ));
        }

        let input = serde_json::to_string(&task)?;
        let sandbox_result = self.wasm_sandbox.execute(module_id, &input).await?;

        if !sandbox_result.success {
            return Err(AetherisError::Runtime(format!(
                "Sandbox execution failed: {:?}",
                sandbox_result.error
            )));
        }

        Ok(task)
    }

    pub async fn pause_task(&self, task_id: &str) -> Result<()> {
        info!("Pausing task: {}", task_id);
        self.atomic_executor.pause_execution(task_id).await
    }

    pub async fn resume_task(&self, task_id: &str) -> Result<()> {
        info!("Resuming task: {}", task_id);
        self.atomic_executor.resume_execution(task_id).await
    }

    pub async fn rollback_task(&self, task_id: &str, snapshot_id: Option<String>) -> Result<Task> {
        info!("Rolling back task: {}", task_id);
        self.atomic_executor.rollback(task_id, snapshot_id).await
    }

    pub fn get_atomic_executor(&self) -> Arc<AtomicExecutor> {
        self.atomic_executor.clone()
    }

    pub fn get_wasm_sandbox(&self) -> Arc<WasmSandbox> {
        self.wasm_sandbox.clone()
    }

    pub fn get_cpu_gpu_scheduler(&self) -> Arc<CpuGpuScheduler> {
        self.cpu_gpu_scheduler.clone()
    }

    pub fn get_config(&self) -> RuntimeConfig {
        self.config.clone()
    }

    async fn ensure_initialized(&self) -> Result<()> {
        let initialized = self.is_initialized.lock().await;
        if !*initialized {
            drop(initialized);
            self.initialize().await?;
        }
        Ok(())
    }
}

impl Default for ExecutionRuntime {
    fn default() -> Self {
        Self::new(RuntimeConfig::default())
    }
}
