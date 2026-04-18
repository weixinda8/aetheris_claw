use aetheris::config::AppConfig;
use aetheris::core::{Task, TaskExecutor};
use aetheris::memory::ShortTermMemory;
use criterion::{Criterion, criterion_group, criterion_main};
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Barrier;

struct SimpleExecutor;

#[async_trait::async_trait]
impl TaskExecutor for SimpleExecutor {
    async fn execute(&self, mut task: Task) -> aetheris::utils::Result<Task> {
        // 模拟任务执行时间
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        task.status = aetheris::core::TaskStatus::Completed;
        task.result = Some("Task executed successfully".to_string());
        Ok(task)
    }

    fn can_execute(&self, _task: &Task) -> bool {
        true
    }
}

fn bench_concurrent_tasks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("concurrent_tasks_100", |b| {
        b.iter(|| {
            rt.block_on(async {
                let memory = ShortTermMemory::new();
                let barrier = Arc::new(Barrier::new(100));
                
                let mut handles = Vec::with_capacity(100);
                for i in 0..100 {
                    let memory = memory.clone();
                    let barrier = barrier.clone();
                    
                    let handle = tokio::spawn(async move {
                        barrier.wait().await;
                        
                        let task = Task::new(format!("Task {}", i), 1);
                        memory.store_task(task);
                    });
                    
                    handles.push(handle);
                }
                
                for handle in handles {
                    handle.await.unwrap();
                }
                
                // 测试获取所有任务
                let tasks = memory.get_all_tasks();
                assert_eq!(tasks.len(), 100);
            });
        });
    });
}

fn bench_task_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("task_throughput", |b| {
        b.iter(|| {
            rt.block_on(async {
                let memory = ShortTermMemory::new();
                
                // 测试1000个任务的存储和获取
                for i in 0..1000 {
                    let task = Task::new(format!("Task {}", i), 1);
                    memory.store_task(task);
                }
                
                let tasks = memory.get_all_tasks();
                assert_eq!(tasks.len(), 1000);
            });
        });
    });
}

criterion_group!(benches, bench_concurrent_tasks, bench_task_throughput);
criterion_main!(benches);
