use crate::storage::timeseries::types::*;
use crate::utils::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[async_trait]
pub trait TimeSeriesDatabase: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn is_connected(&self) -> bool;

    async fn write_point(&mut self, point: TimeSeriesPoint) -> Result<()>;
    async fn write_points(&mut self, points: Vec<TimeSeriesPoint>) -> Result<()>;

    async fn query(&self, query: TimeSeriesQuery) -> Result<Vec<TimeSeriesPoint>>;
    async fn query_raw(&self, query: &str) -> Result<Vec<TimeSeriesPoint>>;

    async fn create_database(&mut self, name: &str) -> Result<()>;
    async fn drop_database(&mut self, name: &str) -> Result<()>;
    async fn list_databases(&self) -> Result<Vec<String>>;

    async fn create_retention_policy(&mut self, policy: RetentionPolicy) -> Result<()>;
    async fn drop_retention_policy(&mut self, name: &str) -> Result<()>;
    async fn list_retention_policies(&self) -> Result<Vec<RetentionPolicy>>;

    async fn create_downsampling_rule(&mut self, rule: DownsamplingRule) -> Result<()>;
    async fn drop_downsampling_rule(&mut self, name: &str) -> Result<()>;
    async fn list_downsampling_rules(&self) -> Result<Vec<DownsamplingRule>>;

    async fn ping(&self) -> Result<Duration>;
    async fn get_stats(&self) -> Result<TimeSeriesStats>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesStats {
    pub total_points_written: u64,
    pub total_points_read: u64,
    pub total_write_errors: u64,
    pub total_query_errors: u64,
    pub average_write_latency_ms: f64,
    pub average_query_latency_ms: f64,
    pub database_size_bytes: u64,
    pub series_count: u64,
}

#[async_trait]
pub trait TimeSeriesDatabaseFactory: Send + Sync {
    fn create(&self, config: TimeSeriesConfig) -> Arc<RwLock<dyn TimeSeriesDatabase + Send + Sync>>;
    fn supported_backends(&self) -> Vec<TimeSeriesBackendType>;
}
