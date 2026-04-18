use crate::core::realtime_scheduler::traits::*;
use crate::core::realtime_scheduler::types::*;
use crate::utils::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::BinaryHeap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

pub struct PriorityQueueTask {
    task: Arc<dyn RealTimeTask>,
    submission_time: chrono::DateTime<chrono::Utc>,
}

impl PartialEq for PriorityQueueTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.config().priority == other.task.config().priority
            && self.submission_time == other.submission_time
    }
}

impl Eq for PriorityQueueTask {}

impl PartialOrd for PriorityQueueTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityQueueTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let priority_cmp = self
            .task
            .config()
            .priority
            .cmp(&other.task.config().priority);
        if priority_cmp != std::cmp::Ordering::Equal {
            priority_cmp.reverse()
        } else {
            self.submission_time.cmp(&other.submission_time)
        }
    }
}

pub struct DeterministicRealtimeScheduler {
    config: SchedulerConfig,
    status: Arc<RwLock<SchedulerStatus>>,
    task_queue: Arc<RwLock<BinaryHeap<PriorityQueueTask>>>,
    running_tasks: Arc<DashMap<String, tokio::task::JoinHandle<()>>>,
    task_stats: Arc<DashMap<String, TaskExecutionStats>>,
    total_tasks: Arc<AtomicU64>,
    overload_count: Arc<AtomicU64>,
    deadline_misses: Arc<AtomicU64>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    join_set: Option<JoinSet<()>>,
}

impl DeterministicRealtimeScheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            config,
            status: Arc::new(RwLock::new(SchedulerStatus::Idle)),
            task_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            running_tasks: Arc::new(DashMap::new()),
            task_stats: Arc::new(DashMap::new()),
            total_tasks: Arc::new(AtomicU64::new(0)),
            overload_count: Arc::new(AtomicU64::new(0)),
            deadline_misses: Arc::new(AtomicU64::new(0)),
            shutdown_tx: None,
            join_set: None,
        }
    }

    async fn execute_task(
        task: Arc<dyn RealTimeTask>,
        stats: Arc<DashMap<String, TaskExecutionStats>>,
        deadline_misses: Arc<AtomicU64>,
        config: SchedulerConfig,
    ) {
        let task_id = task.task_id().to_string();
        let start_time = std::time::Instant::now();
        let deadline = task.config().deadline;

        let result = task.execute().await;
        let execution_time = start_time.elapsed();

        let mut stats_entry = stats
            .entry(task_id.clone())
            .or_insert_with(|| TaskExecutionStats {
                task_id: task_id.clone(),
                total_executions: 0,
                total_success: 0,
                total_failures: 0,
                total_deadline_misses: 0,
                average_execution_time_ms: 0.0,
                max_execution_time_ms: 0.0,
                p99_execution_time_ms: 0.0,
            });

        stats_entry.total_executions += 1;

        match result {
            Ok(_) => stats_entry.total_success += 1,
            Err(_) => stats_entry.total_failures += 1,
        }

        let exec_ms = execution_time.as_secs_f64() * 1000.0;
        let avg = (stats_entry.average_execution_time_ms
            * (stats_entry.total_executions - 1) as f64
            + exec_ms)
            / stats_entry.total_executions as f64;
        stats_entry.average_execution_time_ms = avg;
        stats_entry.max_execution_time_ms = stats_entry.max_execution_time_ms.max(exec_ms);

        if config.enable_deadline_monitoring {
            if let Some(d) = deadline {
                if execution_time > d {
                    stats_entry.total_deadline_misses += 1;
                    deadline_misses.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
    }
}

#[async_trait]
impl RealTimeScheduler for DeterministicRealtimeScheduler {
    async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        let mut join_set = JoinSet::new();

        self.shutdown_tx = Some(shutdown_tx);

        *self.status.write().await = SchedulerStatus::Running;

        let task_queue = self.task_queue.clone();
        let running_tasks = self.running_tasks.clone();
        let task_stats = self.task_stats.clone();
        let deadline_misses = self.deadline_misses.clone();
        let config = self.config.clone();
        let status = self.status.clone();
        let overload_count = self.overload_count.clone();

        join_set.spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(1)) => {
                        let current_running = running_tasks.len();
                        
                        if current_running < config.max_concurrent_tasks {
                            let mut queue = task_queue.write().await;
                            
                            if let Some(pq_task) = queue.pop() {
                                let task = pq_task.task.clone();
                                let task_id = task.task_id().to_string();
                                let task_id_clone = task_id.clone();
                                
                                let stats_clone = task_stats.clone();
                                let deadline_misses_clone = deadline_misses.clone();
                                let config_clone = config.clone();
                                let running_tasks_clone = running_tasks.clone();
                                
                                let handle = tokio::spawn(async move {
                                    Self::execute_task(
                                        task,
                                        stats_clone,
                                        deadline_misses_clone,
                                        config_clone,
                                    ).await;
                                    running_tasks_clone.remove(&task_id_clone);
                                });
                                
                                running_tasks.insert(task_id, handle);
                            }
                        } else {
                            let queue_len = task_queue.read().await.len();
                            let overload_ratio = (current_running + queue_len) as f64 / config.max_concurrent_tasks as f64;
                            
                            if overload_ratio > config.overload_threshold {
                                *status.write().await = SchedulerStatus::Overloaded;
                                overload_count.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                    }
                }
            }
        });

        self.join_set = Some(join_set);

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.status.write().await = SchedulerStatus::Idle;

        if let Some(tx) = self.shutdown_tx.take() {
            std::mem::drop(tx.send(()));
        }

        if let Some(mut join_set) = self.join_set.take() {
            while let Some(result) = join_set.join_next().await {
                if let Err(e) = result {
                    log::error!("Scheduler task error: {}", e);
                }
            }
        }

        for entry in self.running_tasks.iter() {
            entry.value().abort();
        }
        self.running_tasks.clear();

        Ok(())
    }

    async fn pause(&mut self) -> Result<()> {
        *self.status.write().await = SchedulerStatus::Paused;
        Ok(())
    }

    async fn resume(&mut self) -> Result<()> {
        *self.status.write().await = SchedulerStatus::Running;
        Ok(())
    }

    async fn submit_task(&mut self, task: Arc<dyn RealTimeTask>) -> Result<()> {
        let pq_task = PriorityQueueTask {
            task: task.clone(),
            submission_time: chrono::Utc::now(),
        };

        let mut queue = self.task_queue.write().await;
        queue.push(pq_task);
        self.total_tasks.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    async fn cancel_task(&mut self, task_id: &str) -> Result<()> {
        if let Some((_, handle)) = self.running_tasks.remove(task_id) {
            handle.abort();
        }
        Ok(())
    }

    fn status(&self) -> SchedulerStatus {
        *self.status.blocking_read()
    }

    fn metrics(&self) -> SchedulerMetrics {
        SchedulerMetrics {
            status: self.status(),
            current_tasks: self.running_tasks.len(),
            total_tasks: self.total_tasks.load(Ordering::SeqCst),
            cpu_utilization: 0.0,
            memory_utilization: 0.0,
            overload_count: self.overload_count.load(Ordering::SeqCst),
            deadline_misses: self.deadline_misses.load(Ordering::SeqCst),
            average_latency_ms: 0.0,
            p99_latency_ms: 0.0,
        }
    }

    fn task_stats(&self, task_id: &str) -> Option<TaskExecutionStats> {
        self.task_stats.get(task_id).map(|s| s.value().clone())
    }

    fn list_tasks(&self) -> Vec<RealTimeTaskConfig> {
        let queue = self.task_queue.blocking_read();
        queue.iter().map(|t| t.task.config().clone()).collect()
    }
}
