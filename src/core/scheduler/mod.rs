use crate::core::Task;
use std::collections::BinaryHeap;

#[derive(Debug, Clone)]
struct ScheduledTask {
    task: Task,
    priority: u8,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.timestamp == other.timestamp
    }
}

impl Eq for ScheduledTask {}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.timestamp.cmp(&self.timestamp))
    }
}

pub struct TaskScheduler {
    queue: BinaryHeap<ScheduledTask>,
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
        }
    }

    pub fn schedule(&mut self, task: Task) {
        let priority = task.priority;
        self.queue.push(ScheduledTask {
            task,
            priority,
            timestamp: chrono::Utc::now(),
        });
    }

    pub fn pop_task(&mut self) -> Option<Task> {
        self.queue.pop().map(|st| st.task)
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}
