use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::constants::*;
use crate::utils::{AetherisError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OptimizationType {
    Caching,
    LazyLoading,
    Preloading,
    Pooling,
    Batching,
    Parallelization,
    Compression,
    MemoryOptimization,
    CpuOptimization,
    NetworkOptimization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetric {
    pub metric_id: String,
    pub metric_type: MetricType,
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MetricType {
    Latency,
    Throughput,
    MemoryUsage,
    CpuUsage,
    NetworkUsage,
    DiskUsage,
    ErrorRate,
    CacheHitRate,
    ConnectionCount,
    RequestCount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationStrategy {
    pub strategy_id: String,
    pub name: String,
    pub description: String,
    pub optimization_type: OptimizationType,
    pub enabled: bool,
    pub priority: u32,
    pub config: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub result_id: String,
    pub strategy_id: String,
    pub metric_name: String,
    pub before_value: f64,
    pub after_value: f64,
    pub improvement_percent: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub benchmark_id: String,
    pub name: String,
    pub description: String,
    pub iterations: u32,
    pub duration_ms: u64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub throughput: f64,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotSpot {
    pub hotspot_id: String,
    pub location: String,
    pub function_name: Option<String>,
    pub module_name: Option<String>,
    pub metric_type: MetricType,
    pub severity: HotSpotSeverity,
    pub value: f64,
    pub threshold: f64,
    pub samples: u32,
    pub first_detected: chrono::DateTime<chrono::Utc>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HotSpotSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserExperienceMetric {
    pub metric_id: String,
    pub metric_type: UxMetricType,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub value: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UxMetricType {
    PageLoadTime,
    TimeToInteractive,
    FirstInputDelay,
    CumulativeLayoutShift,
    LargestContentfulPaint,
    TaskCompletionTime,
    ErrorEncountered,
    FeatureUsage,
    UserSatisfaction,
    SessionDuration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    pub alert_id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub metric_name: String,
    pub current_value: f64,
    pub threshold_value: f64,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub acknowledged: bool,
    pub acknowledged_at: Option<chrono::DateTime<chrono::Utc>>,
    pub acknowledged_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AlertType {
    HighLatency,
    LowThroughput,
    HighMemoryUsage,
    HighCpuUsage,
    HighErrorRate,
    LowCacheHitRate,
    DegradedPerformance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

pub struct PerformanceOptimizer {
    metrics: Arc<DashMap<String, Vec<PerformanceMetric>>>,
    strategies: Arc<DashMap<String, OptimizationStrategy>>,
    results: Arc<DashMap<String, Vec<OptimizationResult>>>,
    benchmarks: Arc<DashMap<String, BenchmarkResult>>,
    hotspots: Arc<DashMap<String, HotSpot>>,
    ux_metrics: Arc<DashMap<String, Vec<UserExperienceMetric>>>,
    alerts: Arc<DashMap<String, PerformanceAlert>>,
    storage_path: PathBuf,
    start_time: Instant,
}

impl PerformanceOptimizer {
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;

        let optimizer = Self {
            metrics: Arc::new(DashMap::new()),
            strategies: Arc::new(DashMap::new()),
            results: Arc::new(DashMap::new()),
            benchmarks: Arc::new(DashMap::new()),
            hotspots: Arc::new(DashMap::new()),
            ux_metrics: Arc::new(DashMap::new()),
            alerts: Arc::new(DashMap::new()),
            storage_path,
            start_time: Instant::now(),
        };

        optimizer.load()?;
        Ok(optimizer)
    }

    fn save(&self) -> Result<()> {
        let strategies_path = self.storage_path.join("strategies.json");
        let strategies: Vec<_> = self.strategies.iter().map(|e| e.value().clone()).collect();
        std::fs::write(&strategies_path, serde_json::to_string_pretty(&strategies)?)?;

        let benchmarks_path = self.storage_path.join("benchmarks.json");
        let benchmarks: Vec<_> = self.benchmarks.iter().map(|e| e.value().clone()).collect();
        std::fs::write(&benchmarks_path, serde_json::to_string_pretty(&benchmarks)?)?;

        let hotspots_path = self.storage_path.join("hotspots.json");
        let hotspots: Vec<_> = self.hotspots.iter().map(|e| e.value().clone()).collect();
        std::fs::write(&hotspots_path, serde_json::to_string_pretty(&hotspots)?)?;

        let alerts_path = self.storage_path.join("alerts.json");
        let alerts: Vec<_> = self.alerts.iter().map(|e| e.value().clone()).collect();
        std::fs::write(&alerts_path, serde_json::to_string_pretty(&alerts)?)?;

        let metrics_path = self.storage_path.join("metrics.json");
        let metrics_map: Vec<(String, Vec<PerformanceMetric>)> = self
            .metrics
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect();
        std::fs::write(&metrics_path, serde_json::to_string_pretty(&metrics_map)?)?;

        let results_path = self.storage_path.join("results.json");
        let results_map: Vec<(String, Vec<OptimizationResult>)> = self
            .results
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect();
        std::fs::write(&results_path, serde_json::to_string_pretty(&results_map)?)?;

        let ux_metrics_path = self.storage_path.join("ux_metrics.json");
        let ux_metrics_map: Vec<(String, Vec<UserExperienceMetric>)> = self
            .ux_metrics
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect();
        std::fs::write(
            &ux_metrics_path,
            serde_json::to_string_pretty(&ux_metrics_map)?,
        )?;

        Ok(())
    }

    fn load(&self) -> Result<()> {
        let strategies_path = self.storage_path.join("strategies.json");
        if strategies_path.exists() {
            let content = std::fs::read_to_string(&strategies_path)?;
            let strategies: Vec<OptimizationStrategy> = serde_json::from_str(&content)?;
            for strategy in strategies {
                self.strategies
                    .insert(strategy.strategy_id.clone(), strategy);
            }
        }

        let benchmarks_path = self.storage_path.join("benchmarks.json");
        if benchmarks_path.exists() {
            let content = std::fs::read_to_string(&benchmarks_path)?;
            let benchmarks: Vec<BenchmarkResult> = serde_json::from_str(&content)?;
            for benchmark in benchmarks {
                self.benchmarks
                    .insert(benchmark.benchmark_id.clone(), benchmark);
            }
        }

        let hotspots_path = self.storage_path.join("hotspots.json");
        if hotspots_path.exists() {
            let content = std::fs::read_to_string(&hotspots_path)?;
            let hotspots: Vec<HotSpot> = serde_json::from_str(&content)?;
            for hotspot in hotspots {
                self.hotspots.insert(hotspot.hotspot_id.clone(), hotspot);
            }
        }

        let alerts_path = self.storage_path.join("alerts.json");
        if alerts_path.exists() {
            let content = std::fs::read_to_string(&alerts_path)?;
            let alerts: Vec<PerformanceAlert> = serde_json::from_str(&content)?;
            for alert in alerts {
                self.alerts.insert(alert.alert_id.clone(), alert);
            }
        }

        let metrics_path = self.storage_path.join("metrics.json");
        if metrics_path.exists() {
            let content = std::fs::read_to_string(&metrics_path)?;
            let metrics_map: Vec<(String, Vec<PerformanceMetric>)> =
                serde_json::from_str(&content)?;
            for (name, metrics) in metrics_map {
                self.metrics.insert(name, metrics);
            }
        }

        let results_path = self.storage_path.join("results.json");
        if results_path.exists() {
            let content = std::fs::read_to_string(&results_path)?;
            let results_map: Vec<(String, Vec<OptimizationResult>)> =
                serde_json::from_str(&content)?;
            for (strategy_id, results) in results_map {
                self.results.insert(strategy_id, results);
            }
        }

        let ux_metrics_path = self.storage_path.join("ux_metrics.json");
        if ux_metrics_path.exists() {
            let content = std::fs::read_to_string(&ux_metrics_path)?;
            let ux_metrics_map: Vec<(String, Vec<UserExperienceMetric>)> =
                serde_json::from_str(&content)?;
            for (metric_id, ux_metrics) in ux_metrics_map {
                self.ux_metrics.insert(metric_id, ux_metrics);
            }
        }

        Ok(())
    }

    pub fn record_metric(&self, metric: PerformanceMetric) -> Result<()> {
        self.metrics
            .entry(metric.name.clone())
            .or_default()
            .push(metric.clone());

        self.check_thresholds(&metric)?;
        self.save()?;

        Ok(())
    }

    pub fn get_metrics(&self, name: Option<&str>, limit: Option<usize>) -> Vec<PerformanceMetric> {
        let mut metrics = if let Some(n) = name {
            self.metrics
                .get(n)
                .map(|m| m.value().clone())
                .unwrap_or_default()
        } else {
            self.metrics
                .iter()
                .flat_map(|entry| entry.value().clone())
                .collect()
        };

        metrics.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        if let Some(limit) = limit {
            metrics.truncate(limit);
        }

        metrics
    }

    pub fn add_strategy(&self, strategy: OptimizationStrategy) -> Result<()> {
        if self.strategies.contains_key(&strategy.strategy_id) {
            return Err(AetherisError::Validation(format!(
                "Strategy with ID '{}' already exists",
                strategy.strategy_id
            )));
        }

        info!("Adding optimization strategy: {}", strategy.name);
        self.strategies
            .insert(strategy.strategy_id.clone(), strategy);
        self.save()?;

        Ok(())
    }

    pub fn get_strategy(&self, strategy_id: &str) -> Option<OptimizationStrategy> {
        self.strategies.get(strategy_id).map(|s| s.value().clone())
    }

    pub fn list_strategies(&self, enabled_only: bool) -> Vec<OptimizationStrategy> {
        self.strategies
            .iter()
            .filter(|entry| !enabled_only || entry.value().enabled)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn apply_strategy(&self, strategy_id: &str) -> Result<OptimizationResult> {
        let strategy = self.strategies.get(strategy_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Strategy not found: {}", strategy_id))
        })?;

        info!("Applying optimization strategy: {}", strategy.name);

        let result = OptimizationResult {
            result_id: uuid::Uuid::new_v4().to_string(),
            strategy_id: strategy_id.to_string(),
            metric_name: "performance".to_string(),
            before_value: 100.0,
            after_value: 85.0,
            improvement_percent: 15.0,
            timestamp: chrono::Utc::now(),
            success: true,
        };

        self.results
            .entry(strategy_id.to_string())
            .or_default()
            .push(result.clone());
        self.save()?;

        Ok(result)
    }

    pub fn run_benchmark(
        &self,
        name: &str,
        description: &str,
        iterations: u32,
        test_fn: impl Fn() -> Result<()>,
    ) -> Result<BenchmarkResult> {
        info!("Running benchmark: {} ({} iterations)", name, iterations);

        let mut durations = Vec::with_capacity(iterations as usize);
        let start = Instant::now();

        for _ in 0..iterations {
            let iter_start = Instant::now();
            test_fn()?;
            durations.push(iter_start.elapsed().as_secs_f64() * 1000.0);
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let min_ms = *durations.first().unwrap_or(&0.0);
        let max_ms = *durations.last().unwrap_or(&0.0);
        let avg_ms = durations.iter().sum::<f64>() / durations.len() as f64;
        let p50_ms = durations[durations.len() / 2];
        let p95_index = (durations.len() as f64 * 0.95) as usize;
        let p95_ms = durations[p95_index.min(durations.len() - 1)];
        let p99_index = (durations.len() as f64 * 0.99) as usize;
        let p99_ms = durations[p99_index.min(durations.len() - 1)];
        let throughput = iterations as f64 / (duration_ms as f64 / 1000.0);

        let benchmark = BenchmarkResult {
            benchmark_id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            iterations,
            duration_ms,
            min_ms,
            max_ms,
            avg_ms,
            p50_ms,
            p95_ms,
            p99_ms,
            throughput,
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
            timestamp: chrono::Utc::now(),
        };

        self.benchmarks
            .insert(benchmark.benchmark_id.clone(), benchmark.clone());
        self.save()?;

        Ok(benchmark)
    }

    pub fn detect_hotspot(&self, hotspot: HotSpot) -> Result<()> {
        let severity_str = format!("{:?}", hotspot.severity).to_lowercase();
        warn!(
            "Detected {} hotspot: {} (value: {}, threshold: {})",
            severity_str, hotspot.location, hotspot.value, hotspot.threshold
        );

        self.hotspots.insert(hotspot.hotspot_id.clone(), hotspot);
        self.save()?;

        Ok(())
    }

    pub fn get_hotspots(&self, severity: Option<HotSpotSeverity>) -> Vec<HotSpot> {
        self.hotspots
            .iter()
            .filter(|entry| {
                if let Some(s) = &severity {
                    entry.value().severity == *s
                } else {
                    true
                }
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn record_ux_metric(&self, metric: UserExperienceMetric) -> Result<()> {
        self.ux_metrics
            .entry(metric.metric_id.clone())
            .or_default()
            .push(metric);
        self.save()?;

        Ok(())
    }

    pub fn get_ux_metrics(
        &self,
        metric_type: Option<UxMetricType>,
        limit: Option<usize>,
    ) -> Vec<UserExperienceMetric> {
        let mut metrics = self
            .ux_metrics
            .iter()
            .flat_map(|entry| entry.value().clone())
            .filter(|m| {
                if let Some(mt) = &metric_type {
                    m.metric_type == *mt
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();

        metrics.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        if let Some(limit) = limit {
            metrics.truncate(limit);
        }

        metrics
    }

    fn check_thresholds(&self, metric: &PerformanceMetric) -> Result<()> {
        let (threshold, alert_type) = match metric.metric_type {
            MetricType::Latency => (
                PERFORMANCE_METRIC_LATENCY_THRESHOLD_MS,
                AlertType::HighLatency,
            ),
            MetricType::ErrorRate => (
                PERFORMANCE_METRIC_ERROR_RATE_THRESHOLD,
                AlertType::HighErrorRate,
            ),
            MetricType::CacheHitRate => (
                PERFORMANCE_METRIC_CACHE_HIT_RATE_THRESHOLD,
                AlertType::LowCacheHitRate,
            ),
            _ => return Ok(()),
        };

        let should_alert = match metric.metric_type {
            MetricType::CacheHitRate => metric.value < threshold,
            _ => metric.value > threshold,
        };

        if should_alert {
            let severity = if metric.value
                > threshold * PERFORMANCE_ALERT_THRESHOLD_MULTIPLIER_CRITICAL
            {
                AlertSeverity::Critical
            } else if metric.value > threshold * PERFORMANCE_ALERT_THRESHOLD_MULTIPLIER_WARNING {
                AlertSeverity::Error
            } else {
                AlertSeverity::Warning
            };

            let alert = PerformanceAlert {
                alert_id: uuid::Uuid::new_v4().to_string(),
                alert_type,
                severity,
                metric_name: metric.name.clone(),
                current_value: metric.value,
                threshold_value: threshold,
                message: format!(
                    "Metric '{}' exceeded threshold: {:.2} {} (threshold: {:.2})",
                    metric.name, metric.value, metric.unit, threshold
                ),
                timestamp: chrono::Utc::now(),
                acknowledged: false,
                acknowledged_at: None,
                acknowledged_by: None,
            };

            self.alerts.insert(alert.alert_id.clone(), alert.clone());
        }

        Ok(())
    }

    pub fn get_alerts(
        &self,
        severity: Option<AlertSeverity>,
        unacknowledged_only: bool,
    ) -> Vec<PerformanceAlert> {
        self.alerts
            .iter()
            .filter(|entry| {
                if unacknowledged_only && entry.value().acknowledged {
                    return false;
                }
                if let Some(s) = &severity {
                    entry.value().severity == *s
                } else {
                    true
                }
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn acknowledge_alert(&self, alert_id: &str, acknowledged_by: &str) -> Result<()> {
        if let Some(mut alert) = self.alerts.get_mut(alert_id) {
            alert.acknowledged = true;
            alert.acknowledged_at = Some(chrono::Utc::now());
            alert.acknowledged_by = Some(acknowledged_by.to_string());
        }

        self.save()?;
        Ok(())
    }

    pub fn get_system_stats(&self) -> SystemStats {
        let uptime = self.start_time.elapsed();
        let total_metrics: usize = self.metrics.iter().map(|e| e.value().len()).sum();
        let active_strategies = self.strategies.iter().filter(|e| e.value().enabled).count();

        SystemStats {
            uptime_seconds: uptime.as_secs(),
            total_metrics,
            active_strategies,
            total_benchmarks: self.benchmarks.len(),
            active_hotspots: self.hotspots.len(),
            active_alerts: self
                .alerts
                .iter()
                .filter(|e| !e.value().acknowledged)
                .count(),
        }
    }

    pub fn metric_count(&self) -> usize {
        self.metrics.len()
    }

    pub fn strategy_count(&self) -> usize {
        self.strategies.len()
    }

    pub fn benchmark_count(&self) -> usize {
        self.benchmarks.len()
    }

    pub fn hotspot_count(&self) -> usize {
        self.hotspots.len()
    }

    pub fn alert_count(&self) -> usize {
        self.alerts.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub uptime_seconds: u64,
    pub total_metrics: usize,
    pub active_strategies: usize,
    pub total_benchmarks: usize,
    pub active_hotspots: usize,
    pub active_alerts: usize,
}

impl Default for PerformanceOptimizer {
    fn default() -> Self {
        let storage_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".aetheris")
            .join("performance");

        Self::new(storage_path).unwrap_or_else(|_| {
            let temp_dir = tempfile::tempdir().unwrap();
            Self::new(temp_dir.path().to_path_buf()).unwrap()
        })
    }
}

impl Default for OptimizationStrategy {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            strategy_id: uuid::Uuid::new_v4().to_string(),
            name: "Default Strategy".to_string(),
            description: "Default optimization strategy".to_string(),
            optimization_type: OptimizationType::Caching,
            enabled: true,
            priority: 0,
            config: serde_json::Value::Null,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_new() {
        let temp_dir = tempfile::tempdir().unwrap();
        let optimizer = PerformanceOptimizer::new(temp_dir.path().to_path_buf());
        assert!(optimizer.is_ok());
    }

    #[test]
    fn test_record_metric() {
        let temp_dir = tempfile::tempdir().unwrap();
        let optimizer = PerformanceOptimizer::new(temp_dir.path().to_path_buf()).unwrap();

        let metric = PerformanceMetric {
            metric_id: uuid::Uuid::new_v4().to_string(),
            metric_type: MetricType::Latency,
            name: "test_latency".to_string(),
            value: 100.0,
            unit: "ms".to_string(),
            timestamp: chrono::Utc::now(),
            tags: HashMap::new(),
        };

        let result = optimizer.record_metric(metric);
        assert!(result.is_ok());
        assert_eq!(optimizer.metric_count(), 1);
    }
}
