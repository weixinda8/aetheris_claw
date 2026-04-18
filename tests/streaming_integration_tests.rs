use aetheris::protocol::industrial::types::DataPoint;
use aetheris::protocol::industrial::types::DataQuality;
use aetheris::protocol::industrial::types::DataValue;
use aetheris::streaming::state::InMemoryStateBackend;
use aetheris::streaming::*;
use aetheris::utils::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::Mutex;

struct MockStreamSource {
    data: Vec<StreamEvent<i32>>,
    index: usize,
}

impl MockStreamSource {
    fn new() -> Self {
        let mut data = Vec::new();
        for i in 1..=10 {
            data.push(StreamEvent::new(i));
        }
        Self { data, index: 0 }
    }
}

#[async_trait]
impl StreamSource<i32> for MockStreamSource {
    async fn open(&mut self) -> Result<()> {
        Ok(())
    }

    async fn fetch_next(&mut self) -> Result<Option<StreamEvent<i32>>> {
        if self.index < self.data.len() {
            let event = self.data[self.index].clone();
            self.index += 1;
            Ok(Some(event))
        } else {
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
async fn test_map_operator() -> Result<()> {
    let source = Arc::new(MockStreamSource::new());
    let sink = Arc::new(CollectingSink::new());
    let config = StreamConfig {
        parallelism: 1,
        checkpoint_interval: std::time::Duration::from_secs(3600),
        checkpoint_interval_ms: 3600000,
        max_concurrent_checkpoints: 1,
        max_checkpoints: 10,
        backpressure_enabled: false,
        backpressure_threshold: 10000,
        exactly_once_enabled: false,
        buffer_size: 100,
        batch_size: 100,
    };
    let state_backend = Arc::new(InMemoryStateBackend::new());

    let mut map_op = MapOperator::new(|x: i32| x * 2);

    let data = vec![1, 2, 3];
    let mut state = state_backend.get_key_value_state("test").await?;

    for value in data {
        let event = StreamEvent::new(value);
        let result = map_op.process(event, &mut state).await?;
        assert_eq!(result.data, value * 2);
    }

    Ok(())
}

#[tokio::test]
async fn test_filter_operator() -> Result<()> {
    let source = Arc::new(MockStreamSource::new());
    let sink = Arc::new(CollectingSink::new());
    let state_backend = Arc::new(InMemoryStateBackend::new());

    let mut filter_op = FilterOperator::new(|x: &i32| *x % 2 == 0);

    let data = vec![1, 2, 3, 4, 5];
    let mut state = state_backend.get_key_value_state("test").await?;

    for value in data {
        let event = StreamEvent::new(value);
        let result = filter_op.process(event, &mut state).await?;
        let filtered = result.metadata.get("filtered").unwrap() == "true";
        if value % 2 == 0 {
            assert!(!filtered);
        } else {
            assert!(filtered);
        }
    }

    Ok(())
}

#[test]
fn test_simple_pipeline_creation() -> Result<()> {
    let source = Arc::new(MockStreamSource::new());
    let sink = Arc::new(CollectingSink::new());

    let pipeline = SimplePipeline::new("test-pipeline".to_string(), source.clone(), sink.clone());

    assert_eq!(pipeline.name(), "test-pipeline");
    assert!(pipeline.operators().is_empty());

    let filter_op = FilterOperator::new(|x: &i32| *x > 0);
    let pipeline = pipeline.add_operator(filter_op);
    assert_eq!(pipeline.operators().len(), 1);

    Ok(())
}

#[test]
fn test_pipeline_filter_chaining() -> Result<()> {
    let source = Arc::new(MockStreamSource::new());
    let sink = Arc::new(CollectingSink::new());

    let pipeline = SimplePipeline::new("filter-pipeline".to_string(), source.clone(), sink.clone())
        .add_filter(|x: &i32| *x % 2 == 0)
        .add_filter(|x: &i32| *x > 4);

    assert_eq!(pipeline.operators().len(), 2);

    Ok(())
}

struct MockDataPointSource {
    data: Vec<StreamEvent<DataPoint>>,
    index: usize,
}

impl MockDataPointSource {
    fn new() -> Self {
        let mut data = Vec::new();
        let now = chrono::Utc::now();
        for i in 1..=5 {
            let dp = DataPoint {
                tag_name: format!("Tag{}", i),
                timestamp: now,
                value: DataValue::Float64(100.0 + i as f64),
                quality: DataQuality::Good,
            };
            data.push(StreamEvent::new(dp));
        }
        Self { data, index: 0 }
    }
}

#[async_trait]
impl StreamSource<DataPoint> for MockDataPointSource {
    async fn open(&mut self) -> Result<()> {
        Ok(())
    }

    async fn fetch_next(&mut self) -> Result<Option<StreamEvent<DataPoint>>> {
        if self.index < self.data.len() {
            let event = self.data[self.index].clone();
            self.index += 1;
            Ok(Some(event))
        } else {
            Ok(None)
        }
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

#[test]
fn test_datapoint_pipeline() -> Result<()> {
    let source = Arc::new(MockDataPointSource::new());
    let sink = Arc::new(CollectingSink::new());

    let pipeline = SimplePipeline::new(
        "datapoint-pipeline".to_string(),
        source.clone(),
        sink.clone(),
    )
    .add_filter(|dp: &DataPoint| matches!(dp.value, DataValue::Float64(v) if v > 102.0));

    assert_eq!(pipeline.name(), "datapoint-pipeline");
    assert_eq!(pipeline.operators().len(), 1);

    Ok(())
}

#[test]
fn test_stream_config_default() -> Result<()> {
    let config = StreamConfig::default();
    assert!(config.parallelism > 0);
    assert_eq!(config.backpressure_enabled, true);
    assert!(config.buffer_size > 0);
    Ok(())
}

#[test]
fn test_stream_event_creation() -> Result<()> {
    let event = StreamEvent::new(42);
    assert_eq!(event.data, 42);
    assert!(!event.event_id.is_empty());

    let now = chrono::Utc::now();
    let event = StreamEvent::new("test".to_string())
        .with_event_time(now)
        .with_metadata("key", "value");
    assert_eq!(event.data, "test");
    assert_eq!(event.event_time, now);
    assert_eq!(event.metadata.get("key"), Some(&"value".to_string()));

    Ok(())
}
