use crate::storage::timeseries::traits::*;
use crate::storage::timeseries::types::*;
use crate::utils::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub struct InMemoryTimeSeries {
    config: TimeSeriesConfig,
    connected: bool,
    data: Arc<DashMap<String, BTreeMap<chrono::DateTime<chrono::Utc>, TimeSeriesPoint>>>,
    stats: Arc<TimeSeriesStatsInternal>,
}

struct TimeSeriesStatsInternal {
    total_points_written: AtomicU64,
    total_points_read: AtomicU64,
    total_write_errors: AtomicU64,
    total_query_errors: AtomicU64,
}

impl InMemoryTimeSeries {
    pub fn new(config: TimeSeriesConfig) -> Self {
        Self {
            config,
            connected: false,
            data: Arc::new(DashMap::new()),
            stats: Arc::new(TimeSeriesStatsInternal {
                total_points_written: AtomicU64::new(0),
                total_points_read: AtomicU64::new(0),
                total_write_errors: AtomicU64::new(0),
                total_query_errors: AtomicU64::new(0),
            }),
        }
    }

    fn get_series_key(&self, measurement: &str, tags: &HashMap<String, String>) -> String {
        let mut sorted_tags: Vec<(&String, &String)> = tags.iter().collect();
        sorted_tags.sort_by(|a, b| a.0.cmp(b.0));

        let mut key = measurement.to_string();
        for (k, v) in sorted_tags {
            key.push(',');
            key.push_str(k);
            key.push('=');
            key.push_str(v);
        }
        key
    }
}

#[async_trait]
impl TimeSeriesDatabase for InMemoryTimeSeries {
    async fn connect(&mut self) -> Result<()> {
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }

    async fn write_point(&mut self, point: TimeSeriesPoint) -> Result<()> {
        let key = self.get_series_key(&point.measurement, &point.tags);
        let mut series = self.data.entry(key).or_default();
        series.insert(point.timestamp, point);
        self.stats
            .total_points_written
            .fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn write_points(&mut self, points: Vec<TimeSeriesPoint>) -> Result<()> {
        for point in points {
            self.write_point(point).await?;
        }
        Ok(())
    }

    async fn query(&self, query: TimeSeriesQuery) -> Result<Vec<TimeSeriesPoint>> {
        let mut results = Vec::new();

        for entry in self.data.iter() {
            let (key, series) = entry.pair();
            if key.starts_with(&format!("{},", query.measurement)) || key.as_str() == query.measurement.as_str() {
                let points = series
                    .range(
                        query.start_time.unwrap_or(chrono::DateTime::UNIX_EPOCH)
                            ..=query.end_time.unwrap_or(chrono::Utc::now()),
                    )
                    .map(|(_, p)| p.clone());

                if let Some(order) = query.order {
                    if order == QueryOrder::Descending {
                        let mut vec: Vec<_> = points.collect();
                        vec.reverse();
                        results.extend(vec);
                    } else {
                        results.extend(points);
                    }
                } else {
                    results.extend(points);
                }
            }
        }

        self.stats
            .total_points_read
            .fetch_add(results.len() as u64, Ordering::SeqCst);
        Ok(results)
    }

    async fn query_raw(&self, _query: &str) -> Result<Vec<TimeSeriesPoint>> {
        Ok(Vec::new())
    }

    async fn create_database(&mut self, _name: &str) -> Result<()> {
        Ok(())
    }

    async fn drop_database(&mut self, _name: &str) -> Result<()> {
        Ok(())
    }

    async fn list_databases(&self) -> Result<Vec<String>> {
        Ok(vec![self.config.database.clone()])
    }

    async fn create_retention_policy(&mut self, _policy: RetentionPolicy) -> Result<()> {
        Ok(())
    }

    async fn drop_retention_policy(&mut self, _name: &str) -> Result<()> {
        Ok(())
    }

    async fn list_retention_policies(&self) -> Result<Vec<RetentionPolicy>> {
        Ok(self.config.retention_policies.clone())
    }

    async fn create_downsampling_rule(&mut self, _rule: DownsamplingRule) -> Result<()> {
        Ok(())
    }

    async fn drop_downsampling_rule(&mut self, _name: &str) -> Result<()> {
        Ok(())
    }

    async fn list_downsampling_rules(&self) -> Result<Vec<DownsamplingRule>> {
        Ok(self.config.downsampling_rules.clone())
    }

    async fn ping(&self) -> Result<Duration> {
        let start = Instant::now();
        Ok(start.elapsed())
    }

    async fn get_stats(&self) -> Result<TimeSeriesStats> {
        Ok(TimeSeriesStats {
            total_points_written: self.stats.total_points_written.load(Ordering::SeqCst),
            total_points_read: self.stats.total_points_read.load(Ordering::SeqCst),
            total_write_errors: self.stats.total_write_errors.load(Ordering::SeqCst),
            total_query_errors: self.stats.total_query_errors.load(Ordering::SeqCst),
            average_write_latency_ms: 0.0,
            average_query_latency_ms: 0.0,
            database_size_bytes: 0,
            series_count: self.data.len() as u64,
        })
    }
}

pub struct InMemoryTimeSeriesFactory;

impl TimeSeriesDatabaseFactory for InMemoryTimeSeriesFactory {
    fn create(&self, config: TimeSeriesConfig) -> Arc<RwLock<dyn TimeSeriesDatabase + Send + Sync>> {
        Arc::new(RwLock::new(InMemoryTimeSeries::new(config)))
    }

    fn supported_backends(&self) -> Vec<TimeSeriesBackendType> {
        vec![TimeSeriesBackendType::InMemory]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_timeseries_connect_disconnect() {
        let config = TimeSeriesConfig::default();
        let mut db = InMemoryTimeSeries::new(config);
        
        assert!(!db.is_connected().await);
        
        db.connect().await.unwrap();
        assert!(db.is_connected().await);
        
        db.disconnect().await.unwrap();
        assert!(!db.is_connected().await);
    }

    #[tokio::test]
    async fn test_in_memory_timeseries_write_point() {
        let config = TimeSeriesConfig::default();
        let mut db = InMemoryTimeSeries::new(config);
        db.connect().await.unwrap();
        
        let now = chrono::Utc::now();
        let point = TimeSeriesPoint::new("temperature".to_string(), now)
            .add_tag("sensor", "sensor_001")
            .add_field("value", TimeSeriesValue::Float64(25.5));
        
        db.write_point(point).await.unwrap();
        
        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.total_points_written, 1);
        assert_eq!(stats.series_count, 1);
    }

    #[tokio::test]
    async fn test_in_memory_timeseries_write_points() {
        let config = TimeSeriesConfig::default();
        let mut db = InMemoryTimeSeries::new(config);
        db.connect().await.unwrap();
        
        let now = chrono::Utc::now();
        let points = vec![
            TimeSeriesPoint::new("temperature".to_string(), now)
                .add_tag("sensor", "sensor_001")
                .add_field("value", TimeSeriesValue::Float64(25.5)),
            TimeSeriesPoint::new("temperature".to_string(), now + Duration::seconds(1))
                .add_tag("sensor", "sensor_001")
                .add_field("value", TimeSeriesValue::Float64(26.0)),
            TimeSeriesPoint::new("temperature".to_string(), now + Duration::seconds(2))
                .add_tag("sensor", "sensor_002")
                .add_field("value", TimeSeriesValue::Float64(24.5)),
        ];
        
        db.write_points(points).await.unwrap();
        
        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.total_points_written, 3);
        assert_eq!(stats.series_count, 2);
    }

    #[tokio::test]
    async fn test_in_memory_timeseries_query() {
        let config = TimeSeriesConfig::default();
        let mut db = InMemoryTimeSeries::new(config);
        db.connect().await.unwrap();
        
        let now = chrono::Utc::now();
        let point1 = TimeSeriesPoint::new("temperature".to_string(), now - Duration::hours(2))
            .add_tag("sensor", "sensor_001")
            .add_field("value", TimeSeriesValue::Float64(25.5));
        let point2 = TimeSeriesPoint::new("temperature".to_string(), now - Duration::hours(1))
            .add_tag("sensor", "sensor_001")
            .add_field("value", TimeSeriesValue::Float64(26.0));
        
        db.write_point(point1).await.unwrap();
        db.write_point(point2).await.unwrap();
        
        let query = TimeSeriesQuery {
            measurement: "temperature".to_string(),
            start_time: Some(now - Duration::hours(3)),
            end_time: Some(now + Duration::hours(1)),
            tags: None,
            fields: None,
            limit: None,
            offset: None,
            order: Some(QueryOrder::Ascending),
        };
        
        let results = db.query(query).await.unwrap();
        assert_eq!(results.len(), 2);
        
        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.total_points_read, 2);
    }

    #[tokio::test]
    async fn test_in_memory_timeseries_query_order_descending() {
        let config = TimeSeriesConfig::default();
        let mut db = InMemoryTimeSeries::new(config);
        db.connect().await.unwrap();
        
        let now = chrono::Utc::now();
        let point1 = TimeSeriesPoint::new("temperature".to_string(), now - Duration::hours(2))
            .add_tag("sensor", "sensor_001")
            .add_field("value", TimeSeriesValue::Float64(25.5));
        let point2 = TimeSeriesPoint::new("temperature".to_string(), now - Duration::hours(1))
            .add_tag("sensor", "sensor_001")
            .add_field("value", TimeSeriesValue::Float64(26.0));
        
        db.write_point(point1).await.unwrap();
        db.write_point(point2).await.unwrap();
        
        let query = TimeSeriesQuery {
            measurement: "temperature".to_string(),
            start_time: Some(now - Duration::hours(3)),
            end_time: Some(now + Duration::hours(1)),
            tags: None,
            fields: None,
            limit: None,
            offset: None,
            order: Some(QueryOrder::Descending),
        };
        
        let results = db.query(query).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].timestamp > results[1].timestamp);
    }

    #[tokio::test]
    async fn test_in_memory_timeseries_ping() {
        let config = TimeSeriesConfig::default();
        let db = InMemoryTimeSeries::new(config);
        
        let latency = db.ping().await.unwrap();
        assert!(latency < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_in_memory_timeseries_list_databases() {
        let config = TimeSeriesConfig::default();
        let db = InMemoryTimeSeries::new(config);
        
        let databases = db.list_databases().await.unwrap();
        assert_eq!(databases.len(), 1);
        assert_eq!(databases[0], "aetheris");
    }

    #[tokio::test]
    async fn test_in_memory_timeseries_list_retention_policies() {
        let config = TimeSeriesConfig::default();
        let db = InMemoryTimeSeries::new(config);
        
        let policies = db.list_retention_policies().await.unwrap();
        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].name, "autogen");
        assert!(policies[0].is_default);
    }

    #[tokio::test]
    async fn test_in_memory_timeseries_list_downsampling_rules() {
        let config = TimeSeriesConfig::default();
        let db = InMemoryTimeSeries::new(config);
        
        let rules = db.list_downsampling_rules().await.unwrap();
        assert!(rules.is_empty());
    }

    #[test]
    fn test_in_memory_timeseries_factory() {
        let factory = InMemoryTimeSeriesFactory;
        
        let supported = factory.supported_backends();
        assert_eq!(supported.len(), 1);
        assert_eq!(supported[0], TimeSeriesBackendType::InMemory);
        
        let config = TimeSeriesConfig::default();
        let db = factory.create(config);
        assert!(!db.is_closed());
    }
}
