use aetheris::streaming::state::InMemoryStateBackend;
use aetheris::streaming::*;
use aetheris::utils::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

struct MockStreamSourceWithTimestamps {
    data: Vec<StreamEvent<i32>>,
    index: usize,
}

impl MockStreamSourceWithTimestamps {
    fn new() -> Self {
        let mut data = Vec::new();
        let base_time = chrono::Utc::now();
        for i in 1..=10 {
            let event_time = base_time + chrono::Duration::seconds(i as i64);
            let mut event = StreamEvent::new(i);
            event.event_time = event_time;
            data.push(event);
        }
        Self { data, index: 0 }
    }
}

#[async_trait]
impl StreamSource<i32> for MockStreamSourceWithTimestamps {
    async fn open(&mut self) -> Result<()> {
        Ok(())
    }

    async fn fetch_next(&mut self) -> Result<Option<StreamEvent<i32>>> {
        if self.index < self.data.len() {
            let event = self.data[self.index].clone();
            self.index += 1;
            Ok(Some(event))
        } else {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok(None)
        }
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

struct CollectingSink<T> {
    collected: Arc<Mutex<Vec<StreamEvent<T>>>>,
}

impl<T> CollectingSink<T> {
    fn new() -> Self {
        Self {
            collected: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_collected(&self) -> Vec<StreamEvent<T>> {
        self.collected.lock().unwrap().clone()
    }
}

#[async_trait]
impl<T: Clone + Send + Sync + 'static> StreamSink<T> for CollectingSink<T> {
    async fn open(&mut self) -> Result<()> {
        Ok(())
    }

    async fn write(&mut self, event: StreamEvent<T>) -> Result<()> {
        self.collected.lock().unwrap().push(event);
        Ok(())
    }

    async fn write_batch(&mut self, events: Vec<StreamEvent<T>>) -> Result<()> {
        for event in events {
            self.write(event).await?;
        }
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test_aggregate_window_operator() -> Result<()> {
    let mut state_backend = InMemoryStateBackend::new();
    let mut state = state_backend.get_key_value_state("test").await?;

    let mut aggregate_op = AggregateWindowOperator::new(|data: Vec<i32>| data.iter().sum::<i32>());

    let events = vec![
        StreamEvent::new(1),
        StreamEvent::new(2),
        StreamEvent::new(3),
    ];

    let window_start = chrono::Utc::now();
    let window_end = window_start + chrono::Duration::seconds(10);

    let result = aggregate_op
        .process_window(events, window_start, window_end, &mut state)
        .await?;
    assert_eq!(result.data, 6);

    Ok(())
}

#[tokio::test]
async fn test_reduce_window_operator() -> Result<()> {
    let mut state_backend = InMemoryStateBackend::new();
    let mut state = state_backend.get_key_value_state("test").await?;

    let mut reduce_op = ReduceWindowOperator::new(|a: i32, b: i32| a + b);

    let events = vec![
        StreamEvent::new(1),
        StreamEvent::new(2),
        StreamEvent::new(3),
    ];

    let window_start = chrono::Utc::now();
    let window_end = window_start + chrono::Duration::seconds(10);

    let result = reduce_op
        .process_window(events, window_start, window_end, &mut state)
        .await?;
    assert_eq!(result.data, 6);

    Ok(())
}

#[test]
fn test_window_config() -> Result<()> {
    let tumbling_config = WindowConfig {
        window_type: WindowType::Tumbling,
        size: Duration::from_secs(60),
        slide: None,
        gap: None,
        allowed_lateness: Duration::from_secs(5),
    };
    assert_eq!(tumbling_config.window_type, WindowType::Tumbling);

    let sliding_config = WindowConfig {
        window_type: WindowType::Sliding,
        size: Duration::from_secs(60),
        slide: Some(Duration::from_secs(30)),
        gap: None,
        allowed_lateness: Duration::from_secs(5),
    };
    assert_eq!(sliding_config.window_type, WindowType::Sliding);

    let session_config = WindowConfig {
        window_type: WindowType::Session,
        size: Duration::from_secs(300),
        slide: None,
        gap: Some(Duration::from_secs(60)),
        allowed_lateness: Duration::from_secs(5),
    };
    assert_eq!(session_config.window_type, WindowType::Session);

    Ok(())
}

#[tokio::test]
async fn test_window_assigner_tumbling() -> Result<()> {
    let config = WindowConfig {
        window_type: WindowType::Tumbling,
        size: Duration::from_secs(10),
        slide: None,
        gap: None,
        allowed_lateness: Duration::from_secs(0),
    };

    let assigner = WindowAssigner::new(config);
    let event = StreamEvent::new(42);

    let windows = assigner.assign((), event).await?;
    assert_eq!(windows.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_window_assigner_sliding() -> Result<()> {
    let config = WindowConfig {
        window_type: WindowType::Sliding,
        size: Duration::from_secs(10),
        slide: Some(Duration::from_secs(5)),
        gap: None,
        allowed_lateness: Duration::from_secs(0),
    };

    let assigner = WindowAssigner::new(config);
    let event = StreamEvent::new(42);

    let windows = assigner.assign((), event).await?;
    assert!(windows.len() >= 1);

    Ok(())
}

#[tokio::test]
async fn test_window_assigner_session() -> Result<()> {
    let config = WindowConfig {
        window_type: WindowType::Session,
        size: Duration::from_secs(300),
        slide: None,
        gap: Some(Duration::from_secs(60)),
        allowed_lateness: Duration::from_secs(0),
    };

    let assigner = WindowAssigner::new(config);
    let event = StreamEvent::new(42);

    let windows = assigner.assign((), event).await?;
    assert_eq!(windows.len(), 1);

    Ok(())
}
