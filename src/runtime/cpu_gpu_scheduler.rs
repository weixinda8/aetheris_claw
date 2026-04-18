use crate::utils::{AetherisError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ComputeDeviceType {
    Cpu,
    Gpu,
    Tpu,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeDevice {
    pub device_id: String,
    pub device_type: ComputeDeviceType,
    pub vendor: String,
    pub model: String,
    pub memory_total_bytes: usize,
    pub memory_available_bytes: usize,
    pub cores: usize,
    pub is_available: bool,
}

impl ComputeDevice {
    pub fn new(
        device_id: String,
        device_type: ComputeDeviceType,
        vendor: String,
        model: String,
        memory_total_bytes: usize,
        cores: usize,
    ) -> Self {
        Self {
            device_id,
            device_type,
            vendor,
            model,
            memory_total_bytes,
            memory_available_bytes: memory_total_bytes,
            cores,
            is_available: true,
        }
    }

    pub fn update_memory_usage(&mut self, used_bytes: usize) {
        self.memory_available_bytes = self.memory_total_bytes.saturating_sub(used_bytes);
    }

    pub fn compute_capacity_score(&self) -> f64 {
        let mut score = 0.0;

        match self.device_type {
            ComputeDeviceType::Gpu => score += 100.0,
            ComputeDeviceType::Tpu => score += 150.0,
            ComputeDeviceType::Cpu => score += 50.0,
        }

        let memory_ratio = self.memory_available_bytes as f64 / self.memory_total_bytes as f64;
        score += memory_ratio * 50.0;

        score += self.cores as f64 * 0.1;

        if self.is_available {
            score += 1000.0;
        }

        score
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequirements {
    pub task_id: String,
    pub priority: TaskPriority,
    pub preferred_device_type: Option<ComputeDeviceType>,
    pub required_memory_bytes: usize,
    pub estimated_duration_ms: u64,
    pub is_gpu_optimized: bool,
}

impl TaskRequirements {
    pub fn new(task_id: String, required_memory_bytes: usize) -> Self {
        Self {
            task_id,
            priority: TaskPriority::Normal,
            preferred_device_type: None,
            required_memory_bytes,
            estimated_duration_ms: 1000,
            is_gpu_optimized: false,
        }
    }

    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_preferred_device(mut self, device_type: ComputeDeviceType) -> Self {
        self.preferred_device_type = Some(device_type);
        self
    }

    pub fn with_gpu_optimized(mut self, optimized: bool) -> Self {
        self.is_gpu_optimized = optimized;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingDecision {
    pub task_id: String,
    pub assigned_device_id: String,
    pub assigned_device_type: ComputeDeviceType,
    pub estimated_start_time: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct CpuGpuScheduler {
    devices: Arc<DashMap<String, ComputeDevice>>,
    device_semaphores: Arc<DashMap<String, Semaphore>>,
    task_assignments: Arc<DashMap<String, String>>,
    max_tasks_per_gpu: usize,
    max_tasks_per_cpu: usize,
}

impl CpuGpuScheduler {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(DashMap::new()),
            device_semaphores: Arc::new(DashMap::new()),
            task_assignments: Arc::new(DashMap::new()),
            max_tasks_per_gpu: 4,
            max_tasks_per_cpu: 8,
        }
    }

    pub fn with_max_tasks_per_gpu(mut self, max: usize) -> Self {
        self.max_tasks_per_gpu = max;
        self
    }

    pub fn with_max_tasks_per_cpu(mut self, max: usize) -> Self {
        self.max_tasks_per_cpu = max;
        self
    }

    pub fn register_device(&self, device: ComputeDevice) -> Result<()> {
        info!(
            "Registering compute device: {} ({:?})",
            device.device_id, device.device_type
        );

        let device_id = device.device_id.clone();
        let max_permits = match device.device_type {
            ComputeDeviceType::Gpu => self.max_tasks_per_gpu,
            ComputeDeviceType::Tpu => self.max_tasks_per_gpu,
            ComputeDeviceType::Cpu => self.max_tasks_per_cpu,
        };

        self.devices.insert(device_id.clone(), device);
        self.device_semaphores
            .insert(device_id, Semaphore::new(max_permits));

        debug!("Device registered successfully");
        Ok(())
    }

    pub fn unregister_device(&self, device_id: &str) -> Result<()> {
        info!("Unregistering compute device: {}", device_id);
        self.devices.remove(device_id);
        self.device_semaphores.remove(device_id);
        Ok(())
    }

    pub fn get_devices(&self) -> Vec<ComputeDevice> {
        self.devices
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_device(&self, device_id: &str) -> Option<ComputeDevice> {
        self.devices.get(device_id).map(|d| d.value().clone())
    }

    pub async fn schedule_task(
        &self,
        requirements: TaskRequirements,
    ) -> Result<SchedulingDecision> {
        info!(
            "Scheduling task: {} (priority: {:?})",
            requirements.task_id, requirements.priority
        );

        let selected_device = self.select_best_device(&requirements)?;

        let semaphore = self
            .device_semaphores
            .get(&selected_device.device_id)
            .ok_or_else(|| {
                AetherisError::Runtime(format!(
                    "Semaphore not found for device: {}",
                    selected_device.device_id
                ))
            })?;

        let _permit = semaphore
            .acquire()
            .await
            .map_err(|e| AetherisError::Runtime(e.to_string()))?;

        self.task_assignments.insert(
            requirements.task_id.clone(),
            selected_device.device_id.clone(),
        );

        if let Some(mut device) = self.devices.get_mut(&selected_device.device_id) {
            device.update_memory_usage(requirements.required_memory_bytes);
        }

        let decision = SchedulingDecision {
            task_id: requirements.task_id.clone(),
            assigned_device_id: selected_device.device_id,
            assigned_device_type: selected_device.device_type,
            estimated_start_time: Some(chrono::Utc::now()),
        };

        info!(
            "Task {} assigned to device {} ({:?})",
            decision.task_id, decision.assigned_device_id, decision.assigned_device_type
        );

        Ok(decision)
    }

    fn select_best_device(&self, requirements: &TaskRequirements) -> Result<ComputeDevice> {
        let eligible_devices: Vec<ComputeDevice> = self
            .devices
            .iter()
            .filter(|entry| {
                let device = entry.value();
                if !device.is_available {
                    return false;
                }
                if device.memory_available_bytes < requirements.required_memory_bytes {
                    return false;
                }
                if let Some(preferred_type) = &requirements.preferred_device_type {
                    if device.device_type != *preferred_type {
                        return false;
                    }
                }
                true
            })
            .map(|entry| entry.value().clone())
            .collect();

        if eligible_devices.is_empty() {
            return Err(AetherisError::Runtime(
                "No eligible compute devices available".to_string(),
            ));
        }

        let mut best_device = eligible_devices[0].clone();
        let mut best_score = best_device.compute_capacity_score();

        if requirements.is_gpu_optimized {
            for device in &eligible_devices {
                let mut score = device.compute_capacity_score();
                if matches!(
                    device.device_type,
                    ComputeDeviceType::Gpu | ComputeDeviceType::Tpu
                ) {
                    score += 500.0;
                }
                if score > best_score {
                    best_score = score;
                    best_device = device.clone();
                }
            }
        } else {
            for device in &eligible_devices {
                let score = device.compute_capacity_score();
                if score > best_score {
                    best_score = score;
                    best_device = device.clone();
                }
            }
        }

        Ok(best_device)
    }

    pub async fn complete_task(&self, task_id: &str) -> Result<()> {
        info!("Completing task on assigned device: {}", task_id);

        let device_id = self
            .task_assignments
            .remove(task_id)
            .map(|(_, device_id)| device_id)
            .ok_or_else(|| AetherisError::Runtime("Task not found in assignments".to_string()))?;

        if let Some(semaphore) = self.device_semaphores.get(&device_id) {
            semaphore.add_permits(1);
        }

        if let Some(mut device) = self.devices.get_mut(&device_id) {
            device.memory_available_bytes = device.memory_total_bytes;
        }

        debug!("Task {} completed and resources released", task_id);
        Ok(())
    }

    pub async fn fail_task(&self, task_id: &str) -> Result<()> {
        info!("Failing task and releasing resources: {}", task_id);
        self.complete_task(task_id).await
    }

    pub fn get_task_assignment(&self, task_id: &str) -> Option<String> {
        self.task_assignments
            .get(task_id)
            .map(|d| d.value().clone())
    }

    pub fn get_device_load(&self, device_id: &str) -> Result<f64> {
        let device = self
            .get_device(device_id)
            .ok_or_else(|| AetherisError::Runtime("Device not found".to_string()))?;

        let semaphore = self
            .device_semaphores
            .get(device_id)
            .ok_or_else(|| AetherisError::Runtime("Semaphore not found".to_string()))?;

        let max_permits = match device.device_type {
            ComputeDeviceType::Gpu => self.max_tasks_per_gpu,
            ComputeDeviceType::Tpu => self.max_tasks_per_gpu,
            ComputeDeviceType::Cpu => self.max_tasks_per_cpu,
        };

        let available_permits = semaphore.available_permits();
        let used_permits = max_permits - available_permits;

        Ok(used_permits as f64 / max_permits as f64)
    }

    pub fn cleanup(&self) {
        debug!("Cleaning up scheduler state");
    }
}

impl Default for CpuGpuScheduler {
    fn default() -> Self {
        Self::new()
    }
}
