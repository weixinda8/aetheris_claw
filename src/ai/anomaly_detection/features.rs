use std::collections::HashMap;

#[async_trait::async_trait]
pub trait FeatureExtractor: Send + Sync {
    fn name(&self) -> &str;
    fn extract(&mut self, data: &[f64]) -> crate::utils::Result<HashMap<String, f64>>;
    fn extract_batch(
        &mut self,
        data: &[Vec<f64>],
    ) -> crate::utils::Result<Vec<HashMap<String, f64>>>;
}

#[derive(Debug, Clone)]
pub struct StreamingFeatureExtractor {
    window_size: usize,
    data_windows: HashMap<String, Vec<f64>>,
    n: u64,
    means: HashMap<String, f64>,
    m2s: HashMap<String, f64>,
    mins: HashMap<String, f64>,
    maxs: HashMap<String, f64>,
}

impl StreamingFeatureExtractor {
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            data_windows: HashMap::new(),
            n: 0,
            means: HashMap::new(),
            m2s: HashMap::new(),
            mins: HashMap::new(),
            maxs: HashMap::new(),
        }
    }

    pub fn update(&mut self, features: &HashMap<String, f64>) {
        self.n += 1;

        for (key, &value) in features {
            let window = self.data_windows.entry(key.clone()).or_default();
            window.push(value);

            if window.len() > self.window_size {
                window.remove(0);
            }

            let mean = self.means.entry(key.clone()).or_insert(0.0);
            let delta = value - *mean;
            *mean += delta / self.n as f64;

            let m2 = self.m2s.entry(key.clone()).or_insert(0.0);
            *m2 += delta * (value - *mean);

            let min = self.mins.entry(key.clone()).or_insert(f64::INFINITY);
            *min = min.min(value);

            let max = self.maxs.entry(key.clone()).or_insert(f64::NEG_INFINITY);
            *max = max.max(value);
        }
    }

    pub fn extract_statistical_features(&self) -> HashMap<String, f64> {
        let mut features = HashMap::new();

        for (key, window) in &self.data_windows {
            if window.len() >= 2 {
                let mean = window.iter().sum::<f64>() / window.len() as f64;
                let variance =
                    window.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / window.len() as f64;
                let std = variance.sqrt();

                let mut sorted = window.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                let q1 = sorted[(sorted.len() as f64 * 0.25) as usize];
                let q3 = sorted[(sorted.len() as f64 * 0.75) as usize];
                let median = sorted[sorted.len() / 2];

                features.insert(format!("{}_mean", key), mean);
                features.insert(format!("{}_std", key), std);
                features.insert(
                    format!("{}_min", key),
                    *window
                        .iter()
                        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                        .unwrap(),
                );
                features.insert(
                    format!("{}_max", key),
                    *window
                        .iter()
                        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                        .unwrap(),
                );
                features.insert(format!("{}_q1", key), q1);
                features.insert(format!("{}_q3", key), q3);
                features.insert(format!("{}_median", key), median);
                features.insert(format!("{}_iqr", key), q3 - q1);
            }
        }

        features
    }

    pub fn extract_temporal_features(&self) -> HashMap<String, f64> {
        let mut features = HashMap::new();

        for (key, window) in &self.data_windows {
            if window.len() >= 3 {
                let n = window.len() as f64;
                let x_mean = (n - 1.0) / 2.0;
                let y_mean = window.iter().sum::<f64>() / n;

                let mut numerator = 0.0;
                let mut denominator = 0.0;

                for (i, &y) in window.iter().enumerate() {
                    let x = i as f64;
                    numerator += (x - x_mean) * (y - y_mean);
                    denominator += (x - x_mean).powi(2);
                }

                let slope = if denominator > 0.0 {
                    numerator / denominator
                } else {
                    0.0
                };

                features.insert(format!("{}_trend", key), slope);

                if window.len() >= 4 {
                    let half_len = window.len() / 2;
                    let first_half_mean = window[..half_len].iter().sum::<f64>() / half_len as f64;
                    let second_half_mean = window[half_len..].iter().sum::<f64>() / half_len as f64;
                    features.insert(
                        format!("{}_seasonality", key),
                        second_half_mean - first_half_mean,
                    );
                }

                if window.len() >= 3 {
                    let mut autocorrelation = 0.0;
                    let mean = window.iter().sum::<f64>() / window.len() as f64;
                    let mut var = 0.0;

                    for &x in window {
                        var += (x - mean).powi(2);
                    }

                    if var > 0.0 {
                        for i in 0..window.len() - 1 {
                            autocorrelation += (window[i] - mean) * (window[i + 1] - mean);
                        }
                        autocorrelation /= var;
                    }

                    features.insert(format!("{}_autocorrelation", key), autocorrelation);
                }
            }
        }

        features
    }
}

impl Default for StreamingFeatureExtractor {
    fn default() -> Self {
        Self::new(100)
    }
}

#[async_trait::async_trait]
impl FeatureExtractor for StreamingFeatureExtractor {
    fn name(&self) -> &str {
        "Streaming Feature Extractor"
    }

    fn extract(&mut self, data: &[f64]) -> crate::utils::Result<HashMap<String, f64>> {
        let mut features = HashMap::new();

        for (i, &value) in data.iter().enumerate() {
            features.insert(format!("feature_{}", i), value);
        }

        self.update(&features);

        let mut result = HashMap::new();
        result.extend(features);
        result.extend(self.extract_statistical_features());
        result.extend(self.extract_temporal_features());

        Ok(result)
    }

    fn extract_batch(
        &mut self,
        data: &[Vec<f64>],
    ) -> crate::utils::Result<Vec<HashMap<String, f64>>> {
        let mut results = Vec::new();

        for row in data {
            results.push(self.extract(row)?);
        }

        Ok(results)
    }
}
