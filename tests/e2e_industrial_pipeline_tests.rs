use aetheris::protocol::industrial::*;
use aetheris::storage::timeseries::*;
use aetheris::streaming::state::InMemoryStateBackend;
use aetheris::streaming::*;
use aetheris::utils::Result;
use std::sync::Arc;
use std::sync::Mutex;

#[tokio::test]
async fn test_complete_industrial_pipeline() -> Result<()> {
    let protocol_config = IndustrialProtocolConfig::default();
    let mut protocol = MockIndustrialProtocol::new(protocol_config);
    protocol.connect().await?;

    let subscription_config = SubscriptionConfig {
        tag_names: vec![
            "Temperature".to_string(),
            "Pressure".to_string(),
            "Speed".to_string(),
        ],
        sampling_interval_ms: 50,
        queue_size: 100,
        discard_oldest: true,
    };

    let receiver = protocol.subscribe(subscription_config).await?;

    let timeseries_config = TimeSeriesConfig::default();
    let mut timeseries_db = InMemoryTimeSeries::new(timeseries_config);
    timeseries_db.connect().await?;

    let mut collected_data = Vec::new();

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    for _ in 0..10 {
        match receiver.try_recv() {
            Ok(data_point) => {
                collected_data.push(data_point.clone());

                let mut ts_point =
                    TimeSeriesPoint::new("industrial_data".to_string(), data_point.timestamp)
                        .add_tag("tag_name", data_point.tag_name.clone())
                        .add_tag("quality", format!("{:?}", data_point.quality));

                ts_point = match &data_point.value {
                    DataValue::Float64(v) => {
                        ts_point.add_field("value", TimeSeriesValue::Float64(*v))
                    }
                    DataValue::Int32(v) => {
                        ts_point.add_field("value", TimeSeriesValue::Int64(*v as i64))
                    }
                    DataValue::Boolean(v) => {
                        ts_point.add_field("value", TimeSeriesValue::Boolean(*v))
                    }
                    _ => ts_point,
                };

                timeseries_db.write_point(ts_point).await?;
            }
            Err(_) => break,
        }
    }

    assert!(!collected_data.is_empty());

    let query = TimeSeriesQuery {
        measurement: "industrial_data".to_string(),
        start_time: None,
        end_time: None,
        tags: None,
        fields: None,
        limit: None,
        offset: None,
        order: None,
    };

    let results = timeseries_db.query(query).await?;
    assert_eq!(results.len(), collected_data.len());

    protocol.unsubscribe().await?;
    protocol.disconnect().await?;
    timeseries_db.disconnect().await?;

    Ok(())
}

#[tokio::test]
async fn test_industrial_pipeline_with_stream_processing() -> Result<()> {
    let protocol_config = IndustrialProtocolConfig::default();
    let mut protocol = MockIndustrialProtocol::new(protocol_config);
    protocol.connect().await?;

    let timeseries_config = TimeSeriesConfig::default();
    let mut timeseries_db = InMemoryTimeSeries::new(timeseries_config);
    timeseries_db.connect().await?;

    let source = Arc::new(MockDataPointSource::new());
    let sink = Arc::new(CollectingDataPointSink::new());

    let mut pipeline = SimplePipeline::new(
        "industrial-processing-pipeline".to_string(),
        source.clone(),
        sink.clone(),
    )
    .add_filter(|dp: &DataPoint| matches!(dp.value, DataValue::Float64(v) if v > 102.0));

    let state_backend = Arc::new(InMemoryStateBackend::new());
    let mut state = state_backend.get_key_value_state("test").await?;

    let mut count = 0;
    loop {
        if let Some(event) = source.fetch_next().await? {
            let mut processed_event = event.clone();
            for operator in pipeline.operators() {
                processed_event = operator.process(processed_event, &mut state).await?;
            }
            if let Some(filtered) = processed_event.metadata.get("filtered") {
                if filtered != "true" {
                    sink.write(processed_event).await?;
                    count += 1;
                }
            } else {
                sink.write(processed_event).await?;
                count += 1;
            }
        } else {
            break;
        }
    }

    let collected = sink.get_collected();
    assert_eq!(collected.len(), 2);

    for dp in &collected {
        if let DataValue::Float64(v) = dp.data.value {
            assert!(v > 102.0);
        }
    }

    for event in collected {
        let ts_point = TimeSeriesPoint::new(
            "processed_industrial_data".to_string(),
            event.data.timestamp,
        )
        .add_tag("tag_name", event.data.tag_name.clone())
        .add_field(
            "value",
            match &event.data.value {
                DataValue::Float64(v) => TimeSeriesValue::Float64(*v),
                _ => TimeSeriesValue::Float64(0.0),
            },
        );
        timeseries_db.write_point(ts_point).await?;
    }

    let stats = timeseries_db.get_stats().await?;
    assert_eq!(stats.total_points_written, 2);

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

struct CollectingDataPointSink {
    collected: Arc<Mutex<Vec<StreamEvent<DataPoint>>>>,
}

impl CollectingDataPointSink {
    fn new() -> Self {
        Self {
            collected: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_collected(&self) -> Vec<StreamEvent<DataPoint>> {
        self.collected.lock().unwrap().clone()
    }
}

#[async_trait]
impl StreamSink<DataPoint> for CollectingDataPointSink {
    async fn open(&mut self) -> Result<()> {
        Ok(())
    }

    async fn write(&mut self, event: StreamEvent<DataPoint>) -> Result<()> {
        self.collected.lock().unwrap().push(event);
        Ok(())
    }

    async fn write_batch(&mut self, events: Vec<StreamEvent<DataPoint>>) -> Result<()> {
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
async fn test_industrial_data_conversion() -> Result<()> {
    let now = chrono::Utc::now();

    let data_point = DataPoint {
        tag_name: "Temperature".to_string(),
        timestamp: now,
        value: DataValue::Float64(25.5),
        quality: DataQuality::Good,
    };

    let ts_point = TimeSeriesPoint::new("industrial_sensor".to_string(), now)
        .add_tag("tag_name", data_point.tag_name.clone())
        .add_tag("quality", format!("{:?}", data_point.quality))
        .add_field("value", TimeSeriesValue::Float64(25.5));

    assert_eq!(ts_point.measurement, "industrial_sensor");
    assert_eq!(ts_point.timestamp, now);
    assert_eq!(
        ts_point.tags.get("tag_name"),
        Some(&"Temperature".to_string())
    );
    assert_eq!(ts_point.tags.get("quality"), Some(&"Good".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_multiple_protocol_subscriptions() -> Result<()> {
    let config1 = IndustrialProtocolConfig {
        protocol_type: IndustrialProtocolType::ModbusTcp,
        ..Default::default()
    };
    let mut protocol1 = MockIndustrialProtocol::new(config1);
    protocol1.connect().await?;

    let config2 = IndustrialProtocolConfig {
        protocol_type: IndustrialProtocolType::OpcUa,
        ..Default::default()
    };
    let mut protocol2 = MockIndustrialProtocol::new(config2);
    protocol2.connect().await?;

    let sub_config1 = SubscriptionConfig {
        tag_names: vec!["Temperature".to_string()],
        sampling_interval_ms: 100,
        queue_size: 10,
        discard_oldest: true,
    };

    let sub_config2 = SubscriptionConfig {
        tag_names: vec!["Pressure".to_string()],
        sampling_interval_ms: 100,
        queue_size: 10,
        discard_oldest: true,
    };

    let mut receiver1 = protocol1.subscribe(sub_config1).await?;
    let mut receiver2 = protocol2.subscribe(sub_config2).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    let mut count1 = 0;
    for _ in 0..3 {
        if receiver1.try_recv().is_ok() {
            count1 += 1;
        }
    }

    let mut count2 = 0;
    for _ in 0..3 {
        if receiver2.try_recv().is_ok() {
            count2 += 1;
        }
    }

    assert!(count1 > 0);
    assert!(count2 > 0);

    protocol1.unsubscribe().await?;
    protocol2.unsubscribe().await?;
    protocol1.disconnect().await?;
    protocol2.disconnect().await?;

    Ok(())
}

#[test]
fn test_end_to_end_configuration() -> Result<()> {
    let protocol_config = IndustrialProtocolConfig::default();
    assert_eq!(protocol_config.protocol_type, IndustrialProtocolType::OpcUa);
    assert_eq!(protocol_config.endpoint, "127.0.0.1");
    assert_eq!(protocol_config.port, 4840);

    let stream_config = StreamConfig::default();
    assert!(stream_config.parallelism > 0);
    assert!(stream_config.buffer_size > 0);

    let timeseries_config = TimeSeriesConfig::default();
    assert_eq!(
        timeseries_config.backend_type,
        TimeSeriesBackendType::InMemory
    );
    assert_eq!(timeseries_config.database, "aetheris");

    Ok(())
}
