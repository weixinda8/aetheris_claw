use aetheris::storage::timeseries::*;
use aetheris::utils::Result;
use std::collections::HashMap;
use std::time::Duration;

#[tokio::test]
async fn test_in_memory_timeseries_connect() -> Result<()> {
    let config = TimeSeriesConfig::default();
    let mut db = InMemoryTimeSeries::new(config);

    assert!(!db.is_connected());

    db.connect().await?;
    assert!(db.is_connected());

    db.disconnect().await?;
    assert!(!db.is_connected());

    Ok(())
}

#[tokio::test]
async fn test_in_memory_write_single_point() -> Result<()> {
    let config = TimeSeriesConfig::default();
    let mut db = InMemoryTimeSeries::new(config);
    db.connect().await?;

    let now = chrono::Utc::now();
    let point = TimeSeriesPoint::new("temperature".to_string(), now)
        .add_tag("sensor", "sensor1")
        .add_tag("location", "factory1")
        .add_field("value", TimeSeriesValue::Float64(25.5))
        .add_field("unit", TimeSeriesValue::String("Celsius".to_string()));

    db.write_point(point).await?;

    let stats = db.get_stats().await?;
    assert_eq!(stats.total_points_written, 1);

    Ok(())
}

#[tokio::test]
async fn test_in_memory_write_batch_points() -> Result<()> {
    let config = TimeSeriesConfig::default();
    let mut db = InMemoryTimeSeries::new(config);
    db.connect().await?;

    let base_time = chrono::Utc::now();
    let mut points = Vec::new();

    for i in 0..10 {
        let timestamp = base_time + chrono::Duration::seconds(i);
        let point = TimeSeriesPoint::new("pressure".to_string(), timestamp)
            .add_tag("sensor", format!("sensor{}", i % 2))
            .add_field("value", TimeSeriesValue::Float64(100.0 + i as f64 * 0.5));
        points.push(point);
    }

    db.write_points(points).await?;

    let stats = db.get_stats().await?;
    assert_eq!(stats.total_points_written, 10);

    Ok(())
}

#[tokio::test]
async fn test_in_memory_query_by_time_range() -> Result<()> {
    let config = TimeSeriesConfig::default();
    let mut db = InMemoryTimeSeries::new(config);
    db.connect().await?;

    let base_time = chrono::Utc::now();
    for i in 0..10 {
        let timestamp = base_time + chrono::Duration::seconds(i);
        let point = TimeSeriesPoint::new("humidity".to_string(), timestamp)
            .add_tag("sensor", "sensor1")
            .add_field("value", TimeSeriesValue::Float64(50.0 + i as f64));
        db.write_point(point).await?;
    }

    let start_time = Some(base_time + chrono::Duration::seconds(2));
    let end_time = Some(base_time + chrono::Duration::seconds(7));
    let query = TimeSeriesQuery {
        measurement: "humidity".to_string(),
        start_time,
        end_time,
        tags: None,
        fields: None,
        limit: None,
        offset: None,
        order: None,
    };

    let results = db.query(query).await?;
    assert_eq!(results.len(), 6);

    Ok(())
}

#[tokio::test]
async fn test_in_memory_query_by_tags() -> Result<()> {
    let config = TimeSeriesConfig::default();
    let mut db = InMemoryTimeSeries::new(config);
    db.connect().await?;

    let now = chrono::Utc::now();

    for i in 0..5 {
        let point = TimeSeriesPoint::new("vibration".to_string(), now)
            .add_tag("machine", "machine1")
            .add_tag("component", format!("part{}", i))
            .add_field("value", TimeSeriesValue::Float64(i as f64 * 0.1));
        db.write_point(point).await?;
    }

    for i in 0..3 {
        let point = TimeSeriesPoint::new("vibration".to_string(), now)
            .add_tag("machine", "machine2")
            .add_tag("component", format!("part{}", i))
            .add_field("value", TimeSeriesValue::Float64(i as f64 * 0.2));
        db.write_point(point).await?;
    }

    let mut tags = HashMap::new();
    tags.insert("machine".to_string(), vec!["machine1".to_string()]);

    let query = TimeSeriesQuery {
        measurement: "vibration".to_string(),
        start_time: None,
        end_time: None,
        tags: Some(tags),
        fields: None,
        limit: None,
        offset: None,
        order: None,
    };

    let results = db.query(query).await?;
    assert_eq!(results.len(), 5);

    Ok(())
}

#[tokio::test]
async fn test_in_memory_query_with_order() -> Result<()> {
    let config = TimeSeriesConfig::default();
    let mut db = InMemoryTimeSeries::new(config);
    db.connect().await?;

    let base_time = chrono::Utc::now();
    for i in 0..5 {
        let timestamp = base_time + chrono::Duration::seconds(i);
        let point = TimeSeriesPoint::new("flow_rate".to_string(), timestamp)
            .add_tag("sensor", "sensor1")
            .add_field("value", TimeSeriesValue::Float64(i as f64 * 10.0));
        db.write_point(point).await?;
    }

    let query_desc = TimeSeriesQuery {
        measurement: "flow_rate".to_string(),
        start_time: None,
        end_time: None,
        tags: None,
        fields: None,
        limit: None,
        offset: None,
        order: Some(QueryOrder::Descending),
    };

    let results_desc = db.query(query_desc).await?;
    assert_eq!(results_desc.len(), 5);

    let query_asc = TimeSeriesQuery {
        measurement: "flow_rate".to_string(),
        start_time: None,
        end_time: None,
        tags: None,
        fields: None,
        limit: None,
        offset: None,
        order: Some(QueryOrder::Ascending),
    };

    let results_asc = db.query(query_asc).await?;
    assert_eq!(results_asc.len(), 5);

    Ok(())
}

#[tokio::test]
async fn test_in_memory_retention_policies() -> Result<()> {
    let config = TimeSeriesConfig::default();
    let db = InMemoryTimeSeries::new(config);

    let policies = db.list_retention_policies().await?;
    assert_eq!(policies.len(), 1);
    assert_eq!(policies[0].name, "autogen");

    let new_policy = RetentionPolicy {
        name: "one_week".to_string(),
        duration: Duration::from_secs(7 * 24 * 60 * 60),
        shard_duration: None,
        replication: None,
        is_default: false,
    };

    let mut db_mut = db;
    db_mut.create_retention_policy(new_policy).await?;

    let policies = db_mut.list_retention_policies().await?;
    assert_eq!(policies.len(), 1);

    db_mut.drop_retention_policy("one_week").await?;

    Ok(())
}

#[tokio::test]
async fn test_in_memory_database_operations() -> Result<()> {
    let config = TimeSeriesConfig::default();
    let mut db = InMemoryTimeSeries::new(config);

    db.create_database("test_db").await?;

    let databases = db.list_databases().await?;
    assert_eq!(databases.len(), 1);
    assert_eq!(databases[0], "aetheris");

    db.drop_database("test_db").await?;

    Ok(())
}

#[tokio::test]
async fn test_in_memory_stats() -> Result<()> {
    let config = TimeSeriesConfig::default();
    let mut db = InMemoryTimeSeries::new(config);
    db.connect().await?;

    let now = chrono::Utc::now();
    for i in 0..20 {
        let point = TimeSeriesPoint::new("test_stats".to_string(), now)
            .add_tag("series", format!("series{}", i % 4))
            .add_field("value", TimeSeriesValue::Int64(i));
        db.write_point(point).await?;
    }

    let stats = db.get_stats().await?;
    assert_eq!(stats.total_points_written, 20);
    assert!(stats.series_count > 0);

    Ok(())
}

#[tokio::test]
async fn test_in_memory_ping() -> Result<()> {
    let config = TimeSeriesConfig::default();
    let db = InMemoryTimeSeries::new(config);

    let latency = db.ping().await?;
    assert!(latency.as_millis() < 1000);

    Ok(())
}

#[test]
fn test_timeseries_config_default() -> Result<()> {
    let config = TimeSeriesConfig::default();
    assert_eq!(config.backend_type, TimeSeriesBackendType::InMemory);
    assert_eq!(config.database, "aetheris");
    assert_eq!(config.batch_size, 1000);
    Ok(())
}

#[test]
fn test_timeseries_point_builder() -> Result<()> {
    let now = chrono::Utc::now();
    let point = TimeSeriesPoint::new("test".to_string(), now)
        .add_tag("tag1", "value1")
        .add_tag("tag2", "value2")
        .add_field("field1", TimeSeriesValue::Float64(42.0))
        .add_field("field2", TimeSeriesValue::String("test".to_string()));

    assert_eq!(point.measurement, "test");
    assert_eq!(point.timestamp, now);
    assert_eq!(point.tags.len(), 2);
    assert_eq!(point.fields.len(), 2);

    Ok(())
}

#[test]
fn test_timeseries_manager() -> Result<()> {
    let mut manager = TimeSeriesManager::new();
    let factory = InMemoryTimeSeriesFactory;

    manager.register_backend(TimeSeriesBackendType::InMemory, Box::new(factory));

    let supported = manager.supported_backends();
    assert_eq!(supported.len(), 1);
    assert!(supported.contains(&TimeSeriesBackendType::InMemory));

    let config = TimeSeriesConfig::default();
    let db = manager.create_database(config)?;
    assert!(db.is_connected() == false);

    Ok(())
}
