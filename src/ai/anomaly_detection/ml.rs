use super::{Anomaly, AnomalyDetectionMethod, AnomalyDetector};
use rand::Rng;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct IsolationForestDetector {
    n_trees: usize,
    max_samples: usize,
    trees: Vec<IsolationTree>,
    fitted: bool,
    random_seed: u64,
}

#[derive(Debug, Clone)]
struct IsolationTree {
    root: Option<IsolationNode>,
    height_limit: usize,
}

#[derive(Debug, Clone)]
struct IsolationNode {
    feature: String,
    split_value: f64,
    left: Option<Box<IsolationNode>>,
    right: Option<Box<IsolationNode>>,
    is_leaf: bool,
}

impl IsolationForestDetector {
    pub fn new() -> Self {
        Self {
            n_trees: 100,
            max_samples: 256,
            trees: Vec::new(),
            fitted: false,
            random_seed: 42,
        }
    }

    pub fn with_params(n_trees: usize, max_samples: usize) -> Self {
        Self {
            n_trees,
            max_samples,
            ..Self::new()
        }
    }

    fn build_tree(
        &mut self,
        data: &[HashMap<String, f64>],
        height: usize,
    ) -> Option<IsolationNode> {
        if data.len() <= 1 || height >= self.max_samples.trailing_zeros() as usize {
            return Some(IsolationNode {
                feature: String::new(),
                split_value: 0.0,
                left: None,
                right: None,
                is_leaf: true,
            });
        }

        let features: Vec<String> = data[0].keys().cloned().collect();
        let mut rng = rand::thread_rng();
        let feature_idx = rng.gen_range(0..features.len());
        let feature = features[feature_idx].clone();

        let values: Vec<f64> = data.iter().map(|d| d[&feature]).collect();
        let min_val = *values
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();
        let max_val = *values
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        let split_value = rng.gen_range(min_val..max_val);

        let left_data: Vec<_> = data
            .iter()
            .filter(|d| d[&feature] < split_value)
            .cloned()
            .collect();
        let right_data: Vec<_> = data
            .iter()
            .filter(|d| d[&feature] >= split_value)
            .cloned()
            .collect();

        Some(IsolationNode {
            feature,
            split_value,
            left: self.build_tree(&left_data, height + 1).map(Box::new),
            right: self.build_tree(&right_data, height + 1).map(Box::new),
            is_leaf: false,
        })
    }

    fn path_length(&self, features: &HashMap<String, f64>, tree: &IsolationTree) -> f64 {
        let mut node = tree.root.as_ref();
        let mut length = 0.0;

        while let Some(n) = node {
            if n.is_leaf {
                break;
            }
            length += 1.0;
            let value = features.get(&n.feature).unwrap_or(&0.0);
            if *value < n.split_value {
                node = n.left.as_deref();
            } else {
                node = n.right.as_deref();
            }
        }

        length
    }

    fn c(n: f64) -> f64 {
        if n <= 1.0 {
            0.0
        } else {
            2.0 * (n.ln() + 0.5772156649) - 2.0 * (n - 1.0) / n
        }
    }
}

impl Default for IsolationForestDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AnomalyDetector for IsolationForestDetector {
    fn name(&self) -> &str {
        "Isolation Forest Detector"
    }

    fn method(&self) -> AnomalyDetectionMethod {
        AnomalyDetectionMethod::IsolationForest
    }

    async fn detect(&mut self, features: &HashMap<String, f64>) -> crate::utils::Result<Anomaly> {
        let avg_path_length: f64 = self
            .trees
            .iter()
            .map(|tree| self.path_length(features, tree))
            .sum::<f64>()
            / self.trees.len() as f64;

        let c = Self::c(self.max_samples as f64);
        let score = if c > 0.0 {
            2.0f64.powf(-avg_path_length / c)
        } else {
            0.5
        };

        let is_anomaly = score > 0.7;

        Ok(Anomaly::new(
            score,
            is_anomaly,
            features.clone(),
            self.method(),
        ))
    }

    async fn fit(&mut self, data: &[HashMap<String, f64>]) -> crate::utils::Result<()> {
        self.trees.clear();
        let sample_size = data.len().min(self.max_samples);

        for _ in 0..self.n_trees {
            let mut rng = rand::thread_rng();
            let sample: Vec<_> = (0..sample_size)
                .map(|_| data[rng.gen_range(0..data.len())].clone())
                .collect();

            let root = self.build_tree(&sample, 0);
            self.trees.push(IsolationTree {
                root,
                height_limit: self.max_samples.trailing_zeros() as usize,
            });
        }

        self.fitted = true;
        Ok(())
    }

    fn is_fitted(&self) -> bool {
        self.fitted
    }
}

#[derive(Debug, Clone)]
pub struct LOFDetector {
    k: usize,
    data: Vec<HashMap<String, f64>>,
    fitted: bool,
}

impl LOFDetector {
    pub fn new() -> Self {
        Self {
            k: 20,
            data: Vec::new(),
            fitted: false,
        }
    }

    pub fn with_k(k: usize) -> Self {
        Self { k, ..Self::new() }
    }

    fn distance(&self, a: &HashMap<String, f64>, b: &HashMap<String, f64>) -> f64 {
        let mut sum = 0.0;
        for (key, &val_a) in a {
            let val_b = b.get(key).unwrap_or(&0.0);
            sum += (val_a - val_b).powi(2);
        }
        sum.sqrt()
    }

    fn k_distance(&self, point: &HashMap<String, f64>) -> f64 {
        let mut distances: Vec<f64> = self.data.iter().map(|d| self.distance(point, d)).collect();
        distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        distances[self.k.min(distances.len() - 1)]
    }

    fn reachability_distance(&self, a: &HashMap<String, f64>, b: &HashMap<String, f64>) -> f64 {
        self.distance(a, b).max(self.k_distance(b))
    }

    fn local_reachability_density(&self, point: &HashMap<String, f64>) -> f64 {
        let mut neighbors: Vec<_> = self
            .data
            .iter()
            .map(|d| (d, self.distance(point, d)))
            .collect();
        neighbors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        neighbors.truncate(self.k);

        let sum_reach_dist: f64 = neighbors
            .iter()
            .map(|(d, _)| self.reachability_distance(point, d))
            .sum();

        if sum_reach_dist > 0.0 {
            neighbors.len() as f64 / sum_reach_dist
        } else {
            f64::INFINITY
        }
    }
}

impl Default for LOFDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AnomalyDetector for LOFDetector {
    fn name(&self) -> &str {
        "LOF Detector"
    }

    fn method(&self) -> AnomalyDetectionMethod {
        AnomalyDetectionMethod::LOF
    }

    async fn detect(&mut self, features: &HashMap<String, f64>) -> crate::utils::Result<Anomaly> {
        let lrd_point = self.local_reachability_density(features);

        let mut neighbors: Vec<_> = self
            .data
            .iter()
            .map(|d| (d, self.distance(features, d)))
            .collect();
        neighbors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        neighbors.truncate(self.k);

        let avg_lrd_neighbors: f64 = neighbors
            .iter()
            .map(|(d, _)| self.local_reachability_density(d))
            .sum::<f64>()
            / neighbors.len() as f64;

        let score = if lrd_point > 0.0 {
            avg_lrd_neighbors / lrd_point
        } else {
            0.0
        };

        let is_anomaly = score > 1.5;

        Ok(Anomaly::new(
            score,
            is_anomaly,
            features.clone(),
            self.method(),
        ))
    }

    async fn fit(&mut self, data: &[HashMap<String, f64>]) -> crate::utils::Result<()> {
        self.data = data.to_vec();
        self.fitted = true;
        Ok(())
    }

    fn is_fitted(&self) -> bool {
        self.fitted
    }
}
