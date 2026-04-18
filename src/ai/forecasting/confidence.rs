use crate::ai::forecasting::ConfidenceInterval;

#[async_trait::async_trait]
pub trait ConfidenceEstimator: Send + Sync {
    fn name(&self) -> &str;
    async fn estimate(
        &self,
        predictions: &[f64],
        confidence_level: f64,
    ) -> crate::utils::Result<Vec<ConfidenceInterval>>;
}

pub struct QuantileRegressionEstimator {
    quantiles: Vec<f64>,
}

impl QuantileRegressionEstimator {
    pub fn new(quantiles: Vec<f64>) -> Self {
        Self { quantiles }
    }
}

#[async_trait::async_trait]
impl ConfidenceEstimator for QuantileRegressionEstimator {
    fn name(&self) -> &str {
        "Quantile Regression Estimator"
    }

    async fn estimate(
        &self,
        predictions: &[f64],
        confidence_level: f64,
    ) -> crate::utils::Result<Vec<ConfidenceInterval>> {
        let _alpha = (1.0 - confidence_level) / 2.0;
        let mut intervals = Vec::with_capacity(predictions.len());

        let mean = predictions.iter().sum::<f64>() / predictions.len() as f64;
        let std = (predictions.iter().map(|&v| (v - mean).powi(2)).sum::<f64>()
            / predictions.len() as f64)
            .sqrt();

        for &value in predictions {
            let z_score = 1.96;
            let lower = value - z_score * std * (1.0 + 1.0 / predictions.len() as f64).sqrt();
            let upper = value + z_score * std * (1.0 + 1.0 / predictions.len() as f64).sqrt();

            intervals.push(ConfidenceInterval {
                lower,
                upper,
                confidence_level,
            });
        }

        Ok(intervals)
    }
}

pub struct MonteCarloDropoutEstimator {
    num_samples: usize,
    dropout_rate: f64,
}

impl MonteCarloDropoutEstimator {
    pub fn new(num_samples: usize, dropout_rate: f64) -> Self {
        Self {
            num_samples,
            dropout_rate,
        }
    }
}

#[async_trait::async_trait]
impl ConfidenceEstimator for MonteCarloDropoutEstimator {
    fn name(&self) -> &str {
        "Monte Carlo Dropout Estimator"
    }

    async fn estimate(
        &self,
        predictions: &[f64],
        confidence_level: f64,
    ) -> crate::utils::Result<Vec<ConfidenceInterval>> {
        let mut intervals = Vec::with_capacity(predictions.len());
        let alpha = (1.0 - confidence_level) / 2.0;

        for &value in predictions {
            let mut samples = Vec::with_capacity(self.num_samples);
            for _ in 0..self.num_samples {
                let dropout_noise = (rand::random::<f64>() - 0.5) * self.dropout_rate;
                samples.push(value * (1.0 + dropout_noise));
            }

            samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let lower_idx = (alpha * self.num_samples as f64) as usize;
            let upper_idx = ((1.0 - alpha) * self.num_samples as f64) as usize;

            let lower = samples[lower_idx.clamp(0, self.num_samples - 1)];
            let upper = samples[upper_idx.clamp(0, self.num_samples - 1)];

            intervals.push(ConfidenceInterval {
                lower,
                upper,
                confidence_level,
            });
        }

        Ok(intervals)
    }
}
