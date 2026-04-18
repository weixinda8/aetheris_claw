use std::collections::HashMap;

#[async_trait::async_trait]
pub trait OnlineLearner: Send + Sync {
    fn name(&self) -> &str;
    async fn update(&mut self, features: &HashMap<String, f64>) -> crate::utils::Result<()>;
    fn should_retrain(&self) -> bool;
    async fn reset(&mut self) -> crate::utils::Result<()>;
}

#[derive(Debug, Clone)]
pub struct DriftDetector {
    window_size: usize,
    reference_window: Vec<f64>,
    current_window: Vec<f64>,
    threshold: f64,
    drift_detected: bool,
    concept_drift_detected: bool,
    n_updates: u64,
}

impl DriftDetector {
    pub fn new(window_size: usize, threshold: f64) -> Self {
        Self {
            window_size,
            reference_window: Vec::new(),
            current_window: Vec::new(),
            threshold,
            drift_detected: false,
            concept_drift_detected: false,
            n_updates: 0,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(100, 0.05)
    }

    fn compute_ks_test(&self, a: &[f64], b: &[f64]) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let mut a_sorted = a.to_vec();
        let mut b_sorted = b.to_vec();
        a_sorted.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        b_sorted.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));

        let mut i = 0;
        let mut j = 0;
        let mut max_diff = 0.0;

        while i < a_sorted.len() && j < b_sorted.len() {
            let cdf_a = (i + 1) as f64 / a_sorted.len() as f64;
            let cdf_b = (j + 1) as f64 / b_sorted.len() as f64;

            let diff = (cdf_a - cdf_b).abs();
            max_diff = f64::max(max_diff, diff);

            if a_sorted[i] <= b_sorted[j] {
                i += 1;
            } else {
                j += 1;
            }
        }

        max_diff
    }

    fn compute_kl_divergence(&self, p: &[f64], q: &[f64]) -> f64 {
        if p.len() != q.len() || p.is_empty() {
            return f64::INFINITY;
        }

        let sum_p: f64 = p.iter().sum();
        let sum_q: f64 = q.iter().sum();

        if sum_p <= 0.0 || sum_q <= 0.0 {
            return f64::INFINITY;
        }

        let mut kl = 0.0;
        for (pi, qi) in p.iter().zip(q.iter()) {
            let p_norm = pi / sum_p;
            let q_norm = qi / sum_q;

            if p_norm > 0.0 && q_norm > 0.0 {
                kl += p_norm * (p_norm / q_norm).ln();
            }
        }

        kl
    }

    pub fn update(&mut self, value: f64) {
        self.n_updates += 1;

        if self.reference_window.len() < self.window_size {
            self.reference_window.push(value);
        } else {
            self.current_window.push(value);

            if self.current_window.len() >= self.window_size {
                let ks_stat = self.compute_ks_test(&self.reference_window, &self.current_window);

                if ks_stat > self.threshold {
                    self.drift_detected = true;

                    let ref_mean = self.reference_window.iter().sum::<f64>()
                        / self.reference_window.len() as f64;
                    let curr_mean =
                        self.current_window.iter().sum::<f64>() / self.current_window.len() as f64;

                    if (ref_mean - curr_mean).abs() > self.threshold * 2.0 {
                        self.concept_drift_detected = true;
                    }
                }

                self.reference_window = self.current_window.clone();
                self.current_window.clear();
            }
        }
    }

    pub fn has_model_drift(&self) -> bool {
        self.drift_detected
    }

    pub fn has_concept_drift(&self) -> bool {
        self.concept_drift_detected
    }

    pub fn reset(&mut self) {
        self.reference_window.clear();
        self.current_window.clear();
        self.drift_detected = false;
        self.concept_drift_detected = false;
        self.n_updates = 0;
    }
}

impl Default for DriftDetector {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[async_trait::async_trait]
impl OnlineLearner for DriftDetector {
    fn name(&self) -> &str {
        "Drift Detector"
    }

    async fn update(&mut self, features: &HashMap<String, f64>) -> crate::utils::Result<()> {
        for &value in features.values() {
            self.update(value);
        }
        Ok(())
    }

    fn should_retrain(&self) -> bool {
        self.has_model_drift() || self.has_concept_drift()
    }

    async fn reset(&mut self) -> crate::utils::Result<()> {
        self.reset();
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct IncrementalLearner {
    drift_detector: DriftDetector,
    learning_rate: f64,
    n_updates: u64,
}

impl IncrementalLearner {
    pub fn new(learning_rate: f64) -> Self {
        Self {
            drift_detector: DriftDetector::default(),
            learning_rate,
            n_updates: 0,
        }
    }
}

impl Default for IncrementalLearner {
    fn default() -> Self {
        Self::new(0.01)
    }
}

#[async_trait::async_trait]
impl OnlineLearner for IncrementalLearner {
    fn name(&self) -> &str {
        "Incremental Learner"
    }

    async fn update(&mut self, features: &HashMap<String, f64>) -> crate::utils::Result<()> {
        self.n_updates += 1;
        if let Some(value) = features.values().next() {
            self.drift_detector.update(*value);
        }
        Ok(())
    }

    fn should_retrain(&self) -> bool {
        self.drift_detector.has_model_drift()
    }

    async fn reset(&mut self) -> crate::utils::Result<()> {
        self.drift_detector.reset();
        self.n_updates = 0;
        Ok(())
    }
}
