use super::{Anomaly, AnomalyDetectionMethod, AnomalyDetector};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Statistical3SigmaDetector {
    means: HashMap<String, f64>,
    stds: HashMap<String, f64>,
    n: u64,
    m2: HashMap<String, f64>,
    fitted: bool,
}

impl Statistical3SigmaDetector {
    pub fn new() -> Self {
        Self {
            means: HashMap::new(),
            stds: HashMap::new(),
            n: 0,
            m2: HashMap::new(),
            fitted: false,
        }
    }

    fn update_statistics(&mut self, features: &HashMap<String, f64>) {
        self.n += 1;
        for (key, &value) in features {
            let mean = self.means.entry(key.clone()).or_insert(0.0);
            let delta = value - *mean;
            *mean += delta / self.n as f64;

            let m2 = self.m2.entry(key.clone()).or_insert(0.0);
            *m2 += delta * (value - *mean);

            if self.n > 1 {
                let std = (*m2 / (self.n - 1) as f64).sqrt();
                self.stds.insert(key.clone(), std);
            }
        }
    }
}

impl Default for Statistical3SigmaDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AnomalyDetector for Statistical3SigmaDetector {
    fn name(&self) -> &str {
        "Statistical 3-Sigma Detector"
    }

    fn method(&self) -> AnomalyDetectionMethod {
        AnomalyDetectionMethod::Statistical3Sigma
    }

    async fn detect(&mut self, features: &HashMap<String, f64>) -> crate::utils::Result<Anomaly> {
        let mut max_score = 0.0;
        let mut is_anomaly = false;

        for (key, &value) in features {
            if let (Some(&mean), Some(&std)) = (self.means.get(key), self.stds.get(key)) {
                if std > 0.0 {
                    let z_score = (value - mean).abs() / std;
                    max_score = f64::max(max_score, z_score);

                    if z_score > 3.0 {
                        is_anomaly = true;
                    }
                }
            }
        }

        self.update_statistics(features);

        Ok(Anomaly::new(
            max_score,
            is_anomaly,
            features.clone(),
            self.method(),
        ))
    }

    async fn fit(&mut self, data: &[HashMap<String, f64>]) -> crate::utils::Result<()> {
        self.means.clear();
        self.stds.clear();
        self.m2.clear();
        self.n = 0;

        for features in data {
            self.update_statistics(features);
        }

        self.fitted = true;
        Ok(())
    }

    fn is_fitted(&self) -> bool {
        self.fitted
    }
}

#[derive(Debug, Clone)]
pub struct StatisticalIQRDetector {
    q1s: HashMap<String, f64>,
    q3s: HashMap<String, f64>,
    data_history: HashMap<String, Vec<f64>>,
    window_size: usize,
    fitted: bool,
}

impl StatisticalIQRDetector {
    pub fn new() -> Self {
        Self {
            q1s: HashMap::new(),
            q3s: HashMap::new(),
            data_history: HashMap::new(),
            window_size: 1000,
            fitted: false,
        }
    }

    pub fn with_window_size(window_size: usize) -> Self {
        Self {
            window_size,
            ..Self::new()
        }
    }

    fn update_quantiles(&mut self, features: &HashMap<String, f64>) {
        for (key, &value) in features {
            let history = self.data_history.entry(key.clone()).or_default();
            history.push(value);

            if history.len() > self.window_size {
                history.remove(0);
            }

            if history.len() >= 4 {
                let mut sorted = history.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                let q1_idx = (sorted.len() as f64 * 0.25) as usize;
                let q3_idx = (sorted.len() as f64 * 0.75) as usize;

                self.q1s.insert(key.clone(), sorted[q1_idx]);
                self.q3s.insert(key.clone(), sorted[q3_idx]);
            }
        }
    }
}

impl Default for StatisticalIQRDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AnomalyDetector for StatisticalIQRDetector {
    fn name(&self) -> &str {
        "Statistical IQR Detector"
    }

    fn method(&self) -> AnomalyDetectionMethod {
        AnomalyDetectionMethod::StatisticalIQR
    }

    async fn detect(&mut self, features: &HashMap<String, f64>) -> crate::utils::Result<Anomaly> {
        let mut max_score = 0.0;
        let mut is_anomaly = false;

        for (key, &value) in features {
            if let (Some(&q1), Some(&q3)) = (self.q1s.get(key), self.q3s.get(key)) {
                let iqr = q3 - q1;
                let lower_bound = q1 - 1.5 * iqr;
                let upper_bound = q3 + 1.5 * iqr;

                if iqr > 0.0 {
                    let score = if value < lower_bound {
                        (lower_bound - value) / iqr
                    } else if value > upper_bound {
                        (value - upper_bound) / iqr
                    } else {
                        0.0
                    };

                    max_score = f64::max(max_score, score);

                    if value < lower_bound || value > upper_bound {
                        is_anomaly = true;
                    }
                }
            }
        }

        self.update_quantiles(features);

        Ok(Anomaly::new(
            max_score,
            is_anomaly,
            features.clone(),
            self.method(),
        ))
    }

    async fn fit(&mut self, data: &[HashMap<String, f64>]) -> crate::utils::Result<()> {
        self.q1s.clear();
        self.q3s.clear();
        self.data_history.clear();

        for features in data {
            self.update_quantiles(features);
        }

        self.fitted = true;
        Ok(())
    }

    fn is_fitted(&self) -> bool {
        self.fitted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_statistical_3sigma_detector_new() {
        let detector = Statistical3SigmaDetector::new();
        assert!(!detector.is_fitted());
    }

    #[test]
    fn test_statistical_3sigma_detector_default() {
        let detector = Statistical3SigmaDetector::default();
        assert!(!detector.is_fitted());
    }

    #[tokio::test]
    async fn test_statistical_3sigma_detector_fit() {
        let mut detector = Statistical3SigmaDetector::new();

        let mut data = Vec::new();
        for i in 0..10 {
            let mut features = HashMap::new();
            features.insert("value".to_string(), 10.0 + i as f64);
            data.push(features);
        }

        let result = detector.fit(&data).await;
        assert!(result.is_ok());
        assert!(detector.is_fitted());
    }

    #[tokio::test]
    async fn test_statistical_3sigma_detector_detect_normal() {
        let mut detector = Statistical3SigmaDetector::new();

        let mut data = Vec::new();
        for i in 0..20 {
            let mut features = HashMap::new();
            features.insert("value".to_string(), 10.0 + (i % 5) as f64);
            data.push(features);
        }

        detector.fit(&data).await.unwrap();

        let mut features = HashMap::new();
        features.insert("value".to_string(), 12.0);

        let result = detector.detect(&features).await;
        assert!(result.is_ok());
        let anomaly = result.unwrap();
        assert!(!anomaly.is_anomaly);
    }

    #[tokio::test]
    async fn test_statistical_3sigma_detector_detect_anomaly() {
        let mut detector = Statistical3SigmaDetector::new();

        let mut data = Vec::new();
        for i in 0..20 {
            let mut features = HashMap::new();
            features.insert("value".to_string(), 10.0 + (i % 5) as f64);
            data.push(features);
        }

        detector.fit(&data).await.unwrap();

        let mut features = HashMap::new();
        features.insert("value".to_string(), 100.0);

        let result = detector.detect(&features).await;
        assert!(result.is_ok());
        let anomaly = result.unwrap();
        assert!(anomaly.is_anomaly);
    }

    #[test]
    fn test_statistical_3sigma_detector_name() {
        let detector = Statistical3SigmaDetector::new();
        assert_eq!(detector.name(), "Statistical 3-Sigma Detector");
    }

    #[test]
    fn test_statistical_3sigma_detector_method() {
        let detector = Statistical3SigmaDetector::new();
        assert_eq!(detector.method(), AnomalyDetectionMethod::Statistical3Sigma);
    }

    #[test]
    fn test_statistical_iqr_detector_new() {
        let detector = StatisticalIQRDetector::new();
        assert!(!detector.is_fitted());
    }

    #[test]
    fn test_statistical_iqr_detector_default() {
        let detector = StatisticalIQRDetector::default();
        assert!(!detector.is_fitted());
    }

    #[test]
    fn test_statistical_iqr_detector_with_window_size() {
        let detector = StatisticalIQRDetector::with_window_size(500);
        assert!(!detector.is_fitted());
    }

    #[tokio::test]
    async fn test_statistical_iqr_detector_fit() {
        let mut detector = StatisticalIQRDetector::new();

        let mut data = Vec::new();
        for i in 0..10 {
            let mut features = HashMap::new();
            features.insert("value".to_string(), 10.0 + i as f64);
            data.push(features);
        }

        let result = detector.fit(&data).await;
        assert!(result.is_ok());
        assert!(detector.is_fitted());
    }

    #[tokio::test]
    async fn test_statistical_iqr_detector_detect_normal() {
        let mut detector = StatisticalIQRDetector::new();

        let mut data = Vec::new();
        for i in 0..10 {
            let mut features = HashMap::new();
            features.insert("value".to_string(), 10.0 + i as f64);
            data.push(features);
        }

        detector.fit(&data).await.unwrap();

        let mut features = HashMap::new();
        features.insert("value".to_string(), 12.0);

        let result = detector.detect(&features).await;
        assert!(result.is_ok());
        let anomaly = result.unwrap();
        assert!(!anomaly.is_anomaly);
    }

    #[tokio::test]
    async fn test_statistical_iqr_detector_detect_anomaly() {
        let mut detector = StatisticalIQRDetector::new();

        let mut data = Vec::new();
        for i in 0..10 {
            let mut features = HashMap::new();
            features.insert("value".to_string(), 10.0 + i as f64);
            data.push(features);
        }

        detector.fit(&data).await.unwrap();

        let mut features = HashMap::new();
        features.insert("value".to_string(), 100.0);

        let result = detector.detect(&features).await;
        assert!(result.is_ok());
        let anomaly = result.unwrap();
        assert!(anomaly.is_anomaly);
    }

    #[test]
    fn test_statistical_iqr_detector_name() {
        let detector = StatisticalIQRDetector::new();
        assert_eq!(detector.name(), "Statistical IQR Detector");
    }

    #[test]
    fn test_statistical_iqr_detector_method() {
        let detector = StatisticalIQRDetector::new();
        assert_eq!(detector.method(), AnomalyDetectionMethod::StatisticalIQR);
    }
}
