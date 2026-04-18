use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceMetricsData {
    pub total_inferences: u64,
    pub successful_inferences: u64,
    pub failed_inferences: u64,
    pub total_latency_ms: u64,
    pub average_latency_ms: f64,
    pub total_tokens_used: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct InferenceMetrics {
    total_inferences: AtomicU64,
    successful_inferences: AtomicU64,
    failed_inferences: AtomicU64,
    total_latency_ms: AtomicU64,
    total_tokens_used: AtomicU64,
}

impl InferenceMetrics {
    pub fn new() -> Self {
        Self {
            total_inferences: AtomicU64::new(0),
            successful_inferences: AtomicU64::new(0),
            failed_inferences: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            total_tokens_used: AtomicU64::new(0),
        }
    }

    pub fn record_inference(&self, success: bool, latency_ms: u64, tokens_used: u64) {
        self.total_inferences.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ms
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.total_tokens_used
            .fetch_add(tokens_used, Ordering::Relaxed);

        if success {
            self.successful_inferences.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_inferences.fetch_add(1, Ordering::Relaxed);
        }

        info!(
            "Recorded inference: success={}, latency={}ms, tokens={}",
            success, latency_ms, tokens_used
        );
    }

    pub fn get_metrics(&self) -> InferenceMetricsData {
        let total_inferences = self.total_inferences.load(Ordering::Relaxed);
        let successful_inferences = self.successful_inferences.load(Ordering::Relaxed);
        let failed_inferences = self.failed_inferences.load(Ordering::Relaxed);
        let total_latency_ms = self.total_latency_ms.load(Ordering::Relaxed);
        let total_tokens_used = self.total_tokens_used.load(Ordering::Relaxed);

        let average_latency_ms = if total_inferences > 0 {
            total_latency_ms as f64 / total_inferences as f64
        } else {
            0.0
        };

        InferenceMetricsData {
            total_inferences,
            successful_inferences,
            failed_inferences,
            total_latency_ms,
            average_latency_ms,
            total_tokens_used,
            timestamp: chrono::Utc::now(),
        }
    }
}

impl Default for InferenceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inference_metrics_new() {
        let metrics = InferenceMetrics::new();
        let data = metrics.get_metrics();
        assert_eq!(data.total_inferences, 0);
        assert_eq!(data.successful_inferences, 0);
        assert_eq!(data.failed_inferences, 0);
        assert_eq!(data.total_latency_ms, 0);
        assert_eq!(data.average_latency_ms, 0.0);
        assert_eq!(data.total_tokens_used, 0);
    }

    #[test]
    fn test_inference_metrics_default() {
        let metrics = InferenceMetrics::default();
        let data = metrics.get_metrics();
        assert_eq!(data.total_inferences, 0);
    }

    #[test]
    fn test_record_successful_inference() {
        let metrics = InferenceMetrics::new();

        metrics.record_inference(true, 100, 1000);

        let data = metrics.get_metrics();
        assert_eq!(data.total_inferences, 1);
        assert_eq!(data.successful_inferences, 1);
        assert_eq!(data.failed_inferences, 0);
        assert_eq!(data.total_latency_ms, 100);
        assert_eq!(data.average_latency_ms, 100.0);
        assert_eq!(data.total_tokens_used, 1000);
    }

    #[test]
    fn test_record_failed_inference() {
        let metrics = InferenceMetrics::new();

        metrics.record_inference(false, 50, 500);

        let data = metrics.get_metrics();
        assert_eq!(data.total_inferences, 1);
        assert_eq!(data.successful_inferences, 0);
        assert_eq!(data.failed_inferences, 1);
        assert_eq!(data.total_latency_ms, 50);
        assert_eq!(data.average_latency_ms, 50.0);
        assert_eq!(data.total_tokens_used, 500);
    }

    #[test]
    fn test_record_multiple_inferences() {
        let metrics = InferenceMetrics::new();

        metrics.record_inference(true, 100, 1000);
        metrics.record_inference(true, 150, 1500);
        metrics.record_inference(false, 80, 800);
        metrics.record_inference(true, 120, 1200);

        let data = metrics.get_metrics();
        assert_eq!(data.total_inferences, 4);
        assert_eq!(data.successful_inferences, 3);
        assert_eq!(data.failed_inferences, 1);
        assert_eq!(data.total_latency_ms, 450);
        assert_eq!(data.average_latency_ms, 112.5);
        assert_eq!(data.total_tokens_used, 4500);
    }

    #[test]
    fn test_average_latency_zero() {
        let metrics = InferenceMetrics::new();
        let data = metrics.get_metrics();
        assert_eq!(data.average_latency_ms, 0.0);
    }

    #[test]
    fn test_timestamp_populated() {
        let metrics = InferenceMetrics::new();
        metrics.record_inference(true, 100, 1000);
        let data = metrics.get_metrics();
        assert!(data.timestamp <= chrono::Utc::now());
    }
}
