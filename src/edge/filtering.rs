use super::{DataFilter, EdgeData};
use crate::ai::anomaly_detection::{
    AnomalyDetector, Statistical3SigmaDetector, StatisticalIQRDetector,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OutlierDetectionMethod {
    ThreeSigma,
    IQR,
    RuleBased,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    pub min_value: f64,
    pub max_value: f64,
}

pub struct OutlierDetector {
    method: OutlierDetectionMethod,
    statistical_detectors: HashMap<String, Box<dyn AnomalyDetector + Send + Sync>>,
    rule_configs: HashMap<String, RuleConfig>,
}

impl OutlierDetector {
    pub fn new(method: OutlierDetectionMethod) -> Self {
        Self {
            method,
            statistical_detectors: HashMap::new(),
            rule_configs: HashMap::new(),
        }
    }

    pub fn add_rule(&mut self, feature_name: String, config: RuleConfig) {
        self.rule_configs.insert(feature_name, config);
    }

    fn get_statistical_detector(
        &mut self,
        feature_name: &str,
        method: &OutlierDetectionMethod,
    ) -> &mut Box<dyn AnomalyDetector + Send + Sync> {
        self.statistical_detectors
            .entry(feature_name.to_string())
            .or_insert_with(|| match method {
                OutlierDetectionMethod::ThreeSigma => Box::new(Statistical3SigmaDetector::new()),
                OutlierDetectionMethod::IQR => Box::new(StatisticalIQRDetector::new()),
                _ => Box::new(Statistical3SigmaDetector::new()),
            })
    }

    fn is_outlier_statistical(&mut self, feature_name: &str, value: f64) -> bool {
        let method = self.method;
        let detector = self.get_statistical_detector(feature_name, &method);
        let mut features = HashMap::new();
        features.insert(feature_name.to_string(), value);

        if !detector.is_fitted() {
            let mut data = vec![features.clone()];
            for _ in 0..100 {
                data.push(features.clone());
            }
            let _ = futures::executor::block_on(detector.fit(&data));
        }

        if let Ok(anomaly) = futures::executor::block_on(detector.detect(&features)) {
            anomaly.is_anomaly
        } else {
            false
        }
    }

    fn is_outlier_rule_based(&self, feature_name: &str, value: f64) -> bool {
        if let Some(config) = self.rule_configs.get(feature_name) {
            value < config.min_value || value > config.max_value
        } else {
            false
        }
    }

    pub fn filter_outliers(&mut self, data: EdgeData) -> EdgeData {
        let mut filtered_values = HashMap::new();

        for (key, &value) in &data.values {
            let is_outlier = match self.method {
                OutlierDetectionMethod::ThreeSigma | OutlierDetectionMethod::IQR => {
                    self.is_outlier_statistical(key, value)
                }
                OutlierDetectionMethod::RuleBased => self.is_outlier_rule_based(key, value),
            };

            if !is_outlier {
                filtered_values.insert(key.clone(), value);
            }
        }

        let mut result = data.clone();
        result.values = filtered_values;
        result
    }
}

#[async_trait]
impl DataFilter for OutlierDetector {
    fn name(&self) -> &str {
        "OutlierDetector"
    }

    async fn filter(&mut self, data: EdgeData) -> crate::utils::Result<Vec<EdgeData>> {
        let filtered = self.filter_outliers(data);
        Ok(vec![filtered])
    }
}
