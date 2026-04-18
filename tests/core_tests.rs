use aetheris::core::{Task, TaskStatus};
use aetheris::memory::{LongTermMemory, MemoryItem, MidTermMemory, ShortTermMemory};
use serde_json::json;
use uuid::Uuid;

#[test]
fn test_task_creation() {
    let task = Task::new("Test task".to_string(), 1);
    assert_eq!(task.description, "Test task");
    assert_eq!(task.title, "Test task");
    assert_eq!(task.priority, 1);
    assert_eq!(task.status, TaskStatus::Pending);
    assert!(!task.id.is_empty());
}

#[test]
fn test_task_with_tags() {
    let task = Task::new("Test task".to_string(), 1)
        .with_tags(vec!["tag1".to_string(), "tag2".to_string()]);
    assert_eq!(task.tags.len(), 2);
    assert_eq!(task.tags[0], "tag1");
    assert_eq!(task.tags[1], "tag2");
}

#[test]
fn test_task_status_transition() {
    let mut task = Task::new("Test".to_string(), 1);
    assert_eq!(task.status, TaskStatus::Pending);

    task.mark_running();
    assert_eq!(task.status, TaskStatus::Running);

    task.mark_completed();
    assert_eq!(task.status, TaskStatus::Completed);
}

#[test]
fn test_task_mark_failed() {
    let mut task = Task::new("Test".to_string(), 1);
    task.mark_running();
    task.mark_failed();
    assert_eq!(task.status, TaskStatus::Failed);
}

#[test]
fn test_task_mark_paused() {
    let mut task = Task::new("Test".to_string(), 1);
    task.mark_running();
    task.mark_paused();
    assert_eq!(task.status, TaskStatus::Paused);
}

#[test]
fn test_task_result() {
    let mut task = Task::new("Test".to_string(), 1);
    task.result = Some("Done".to_string());
    assert!(task.result.is_some());
    assert_eq!(task.result.as_ref().unwrap(), "Done");
}

#[test]
fn test_task_metadata() {
    let mut task = Task::new("Test task".to_string(), 1);
    task.metadata = json!({"key": "value"});
    assert_eq!(task.metadata["key"], "value");
}

#[tokio::test]
async fn test_short_term_memory() {
    let memory = ShortTermMemory::new();

    let item = MemoryItem::new(json!({"task": "test"}), vec!["test".to_string()], 0.5);

    memory.store(item.clone()).await.unwrap();

    let retrieved = memory.retrieve("test").await.unwrap();
    assert_eq!(retrieved.len(), 1);

    let recent = memory.get_recent(1).await.unwrap();
    assert_eq!(recent.len(), 1);

    let queue = memory.get_queue().await.unwrap();
    assert_eq!(queue.len(), 1);

    memory.clear().await;
    let empty = memory.retrieve("test").await.unwrap();
    assert_eq!(empty.len(), 0);
}

#[tokio::test]
async fn test_short_term_memory_get() {
    let memory = ShortTermMemory::new();

    let item = MemoryItem::new(json!({"task": "test"}), vec!["test".to_string()], 0.5);
    let item_id = item.id.clone();

    memory.store(item.clone()).await.unwrap();

    let retrieved = memory.get(&item_id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, item_id);
}

#[tokio::test]
async fn test_short_term_memory_context() {
    let memory = ShortTermMemory::new();

    let session_id = Uuid::new_v4();
    let context_id = memory.create_context(session_id).await.unwrap();

    let context = memory.get_context(context_id).await.unwrap();
    assert!(context.is_some());

    memory
        .set_context_variable(context_id, "key".to_string(), json!("value"))
        .await
        .unwrap();

    let value = memory
        .get_context_variable(context_id, "key")
        .await
        .unwrap();
    assert_eq!(value, Some(json!("value")));

    memory
        .push_call_frame(context_id, "frame1".to_string())
        .await
        .unwrap();

    let frame = memory.pop_call_frame(context_id).await.unwrap();
    assert_eq!(frame, Some("frame1".to_string()));

    memory.remove_context(context_id).await.unwrap();
    let removed = memory.get_context(context_id).await.unwrap();
    assert!(removed.is_none());
}

#[tokio::test]
async fn test_short_term_memory_task_storage() {
    let memory = ShortTermMemory::new();

    let task = Task::new("Test task".to_string(), 1);
    let task_id = task.id.clone();

    memory.store_task(task.clone());

    let retrieved = memory.get_task(&task_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, task_id);

    let all_tasks = memory.get_all_tasks();
    assert_eq!(all_tasks.len(), 1);
}

#[test]
fn test_mid_term_memory_new() {
    let _memory = MidTermMemory::new();
    assert!(true);
}

#[test]
fn test_long_term_memory_new() {
    let _memory = LongTermMemory::new();
    assert!(true);
}

#[test]
fn test_memory_type() {
    use aetheris::memory::MemoryType;

    assert_eq!(MemoryType::Experience.as_str(), "experience");
    assert_eq!(MemoryType::Skill.as_str(), "skill");
    assert_eq!(MemoryType::Case.as_str(), "case");
    assert_eq!(MemoryType::Pattern.as_str(), "pattern");
    assert_eq!(MemoryType::Heuristic.as_str(), "heuristic");

    assert_eq!(MemoryType::from_str("experience"), MemoryType::Experience);
    assert_eq!(MemoryType::from_str("skill"), MemoryType::Skill);
    assert_eq!(MemoryType::from_str("unknown"), MemoryType::Experience);
}

#[test]
fn test_preloadable_type() {
    use aetheris::core::smart_preload::PreloadableType;

    assert_eq!(PreloadableType::from_u8(0), Some(PreloadableType::Skill));
    assert_eq!(PreloadableType::from_u8(1), Some(PreloadableType::Agent));
    assert_eq!(PreloadableType::from_u8(2), Some(PreloadableType::Plugin));
    assert_eq!(
        PreloadableType::from_u8(3),
        Some(PreloadableType::Component)
    );
    assert_eq!(PreloadableType::from_u8(4), Some(PreloadableType::Soul));
    assert_eq!(PreloadableType::from_u8(5), Some(PreloadableType::Config));
    assert_eq!(PreloadableType::from_u8(255), None);
}
