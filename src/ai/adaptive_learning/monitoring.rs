use super::*;
use dashmap::DashMap;
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub accuracy: Option<f64>,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1_score: Option<f64>,
    pub latency_ms: f64,
    pub throughput: f64,
    pub error_rate: f64,
    pub timestamp: DateTime<Utc>,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            accuracy: None,
            precision: None,
            recall: None,
            f1_score: None,
            latency_ms: 0.0,
            throughput: 0.0,
            error_rate: 0.0,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftDetectionResult {
    pub has_drift: bool,
    pub drift_score: f64,
    pub drift_type: Option<String>,
    pub feature_drifts: HashMap<String, f64>,
    pub timestamp: DateTime<Utc>,
}

pub struct ModelPerformanceMonitor {
    metrics_history: DashMap<String, VecDeque<PerformanceMetrics>>,
    drift_results: DashMap<String, VecDeque<DriftDetectionResult>>,
    alerts: DashMap<String, Vec<PerformanceAlert>>,
    config: MonitoringConfig,
}

impl ModelPerformanceMonitor {
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            metrics_history: DashMap::new(),
            drift_results: DashMap::new(),
            alerts: DashMap::new(),
            config,
        }
    }

    pub fn record_metrics(&self, model_id: &str, metrics: PerformanceMetrics) {
        let mut history = self
            .metrics_history
            .entry(model_id.to_string())
            .or_insert_with(|| VecDeque::with_capacity(self.config.max_history_size));

        history.push_back(metrics);

        while history.len() > self.config.max_history_size {
            history.pop_front();
        }

        self.check_alerts(model_id);
    }

    pub fn get_latest_metrics(&self, model_id: &str) -> Option<PerformanceMetrics> {
        self.metrics_history
            .get(model_id)
            .and_then(|history| history.back().cloned())
    }

    pub fn get_metrics_history(
        &self,
        model_id: &str,
        limit: Option<usize>,
    ) -> Vec<PerformanceMetrics> {
        self.metrics_history
            .get(model_id)
            .map(|history| {
                let mut metrics: Vec<PerformanceMetrics> = history.iter().cloned().collect();
                if let Some(limit) = limit {
                    metrics.truncate(limit);
                }
                metrics
            })
            .unwrap_or_default()
    }

    pub fn detect_drift(
        &self,
        model_id: &str,
        current_data: &[f64],
        baseline_data: &[f64],
    ) -> DriftDetectionResult {
        let drift_score = self.compute_ks_test(current_data, baseline_data);

        let result = DriftDetectionResult {
            has_drift: drift_score > self.config.drift_threshold,
            drift_score,
            drift_type: if drift_score > self.config.drift_threshold {
                Some("data_drift".to_string())
            } else {
                None
            },
            feature_drifts: HashMap::new(),
            timestamp: Utc::now(),
        };

        let mut history = self
            .drift_results
            .entry(model_id.to_string())
            .or_insert_with(|| VecDeque::with_capacity(100));

        history.push_back(result.clone());

        while history.len() > 100 {
            history.pop_front();
        }

        result
    }

    fn compute_ks_test(&self, data1: &[f64], data2: &[f64]) -> f64 {
        if data1.is_empty() || data2.is_empty() {
            return 0.0;
        }

        let mut sorted1 = data1.to_vec();
        let mut sorted2 = data2.to_vec();
        sorted1.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        sorted2.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let n1 = sorted1.len() as f64;
        let n2 = sorted2.len() as f64;
        let mut i = 0;
        let mut j = 0;
        let mut max_diff = 0.0;

        while i < sorted1.len() && j < sorted2.len() {
            let d1 = (i + 1) as f64 / n1;
            let d2 = (j + 1) as f64 / n2;
            let diff = (d1 - d2).abs();

            if diff > max_diff {
                max_diff = diff;
            }

            if sorted1[i] <= sorted2[j] {
                i += 1;
            } else {
                j += 1;
            }
        }

        max_diff
    }

    fn check_alerts(&self, model_id: &str) {
        if let Some(metrics) = self.get_latest_metrics(model_id) {
            if let Some(accuracy) = metrics.accuracy {
                if accuracy < self.config.accuracy_threshold {
                    self.add_alert(
                        model_id,
                        PerformanceAlert::new(
                            "accuracy_drop".to_string(),
                            format!("Accuracy dropped below threshold: {:.2}", accuracy),
                            AlertSeverity::Warning,
                        ),
                    );
                }
            }

            if metrics.error_rate > self.config.error_rate_threshold {
                self.add_alert(
                    model_id,
                    PerformanceAlert::new(
                        "high_error_rate".to_string(),
                        format!("High error rate: {:.2}%", metrics.error_rate * 100.0),
                        AlertSeverity::Error,
                    ),
                );
            }

            if metrics.latency_ms > self.config.latency_threshold_ms {
                self.add_alert(
                    model_id,
                    PerformanceAlert::new(
                        "high_latency".to_string(),
                        format!("High latency: {:.2}ms", metrics.latency_ms),
                        AlertSeverity::Warning,
                    ),
                );
            }
        }
    }

    fn add_alert(&self, model_id: &str, alert: PerformanceAlert) {
        self.alerts
            .entry(model_id.to_string())
            .or_default()
            .push(alert);
    }

    pub fn get_alerts(&self, model_id: &str, limit: Option<usize>) -> Vec<PerformanceAlert> {
        self.alerts
            .get(model_id)
            .map(|alerts| {
                let mut result = alerts.clone();
                result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                if let Some(limit) = limit {
                    result.truncate(limit);
                }
                result
            })
            .unwrap_or_default()
    }

    pub fn compute_average_metrics(
        &self,
        model_id: &str,
        window_size: usize,
    ) -> Option<PerformanceMetrics> {
        let history = self.get_metrics_history(model_id, Some(window_size));

        if history.is_empty() {
            return None;
        }

        let mut avg = PerformanceMetrics::default();
        let count = history.len() as f64;

        for metrics in &history {
            avg.latency_ms += metrics.latency_ms;
            avg.throughput += metrics.throughput;
            avg.error_rate += metrics.error_rate;
        }

        avg.latency_ms /= count;
        avg.throughput /= count;
        avg.error_rate /= count;

        Some(avg)
    }
}

impl Default for ModelPerformanceMonitor {
    fn default() -> Self {
        Self::new(MonitoringConfig::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub max_history_size: usize,
    pub accuracy_threshold: f64,
    pub error_rate_threshold: f64,
    pub latency_threshold_ms: f64,
    pub drift_threshold: f64,
    pub evaluation_interval_seconds: u64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            max_history_size: 1000,
            accuracy_threshold: 0.8,
            error_rate_threshold: 0.1,
            latency_threshold_ms: 1000.0,
            drift_threshold: 0.2,
            evaluation_interval_seconds: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    pub id: String,
    pub alert_type: String,
    pub message: String,
    pub severity: AlertSeverity,
    pub timestamp: DateTime<Utc>,
    pub resolved: bool,
}

impl PerformanceAlert {
    pub fn new(alert_type: String, message: String, severity: AlertSeverity) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            alert_type,
            message,
            severity,
            timestamp: Utc::now(),
            resolved: false,
        }
    }
}
