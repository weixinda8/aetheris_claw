use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub value: f64,
    pub feature_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyPoint {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub score: f64,
    pub is_anomaly: bool,
    pub feature_values: HashMap<String, f64>,
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureStats {
    pub name: String,
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyVisualizationData {
    pub time_series: Vec<TimeSeriesPoint>,
    pub anomalies: Vec<AnomalyPoint>,
    pub feature_stats: Vec<FeatureStats>,
    pub score_history: Vec<(chrono::DateTime<chrono::Utc>, f64)>,
    pub threshold: f64,
    pub detector_name: String,
    pub detector_method: String,
    pub total_anomalies: usize,
    pub recent_anomalies: usize,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
}

impl AnomalyVisualizationData {
    pub fn new(detector_name: String, detector_method: String) -> Self {
        Self {
            time_series: Vec::new(),
            anomalies: Vec::new(),
            feature_stats: Vec::new(),
            score_history: Vec::new(),
            threshold: 0.7,
            detector_name,
            detector_method,
            total_anomalies: 0,
            recent_anomalies: 0,
            start_time: chrono::Utc::now(),
            end_time: chrono::Utc::now(),
        }
    }

    pub fn add_time_series_point(
        &mut self,
        timestamp: chrono::DateTime<chrono::Utc>,
        value: f64,
        feature_name: String,
    ) {
        self.time_series.push(TimeSeriesPoint {
            timestamp,
            value,
            feature_name,
        });

        if timestamp < self.start_time {
            self.start_time = timestamp;
        }
        if timestamp > self.end_time {
            self.end_time = timestamp;
        }
    }

    pub fn add_anomaly(&mut self, anomaly: &crate::ai::anomaly_detection::Anomaly) {
        self.anomalies.push(AnomalyPoint {
            id: anomaly.id.clone(),
            timestamp: anomaly.timestamp,
            score: anomaly.score,
            is_anomaly: anomaly.is_anomaly,
            feature_values: anomaly.feature_values.clone(),
            method: format!("{:?}", anomaly.method),
        });

        if anomaly.is_anomaly {
            self.total_anomalies += 1;

            let now = chrono::Utc::now();
            let one_hour_ago = now - chrono::Duration::hours(1);
            if anomaly.timestamp > one_hour_ago {
                self.recent_anomalies += 1;
            }
        }

        self.score_history.push((anomaly.timestamp, anomaly.score));

        if anomaly.timestamp < self.start_time {
            self.start_time = anomaly.timestamp;
        }
        if anomaly.timestamp > self.end_time {
            self.end_time = anomaly.timestamp;
        }
    }

    pub fn compute_feature_stats(&mut self) {
        let mut feature_data: HashMap<String, Vec<f64>> = HashMap::new();

        for point in &self.time_series {
            feature_data
                .entry(point.feature_name.clone())
                .or_default()
                .push(point.value);
        }

        self.feature_stats.clear();

        for (name, values) in feature_data {
            if !values.is_empty() {
                let mean = values.iter().sum::<f64>() / values.len() as f64;
                let variance =
                    values.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
                let std = variance.sqrt();
                let min = *values
                    .iter()
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap();
                let max = *values
                    .iter()
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap();

                self.feature_stats.push(FeatureStats {
                    name,
                    mean,
                    std,
                    min,
                    max,
                });
            }
        }
    }

    pub fn trim_old_data(&mut self, max_points: usize) {
        if self.time_series.len() > max_points {
            let excess = self.time_series.len() - max_points;
            self.time_series.drain(0..excess);
        }

        if self.score_history.len() > max_points {
            let excess = self.score_history.len() - max_points;
            self.score_history.drain(0..excess);
        }

        let one_week_ago = chrono::Utc::now() - chrono::Duration::days(7);
        self.anomalies.retain(|a| a.timestamp > one_week_ago);
    }
}

impl Default for AnomalyVisualizationData {
    fn default() -> Self {
        Self::new("Unknown Detector".to_string(), "Unknown Method".to_string())
    }
}
