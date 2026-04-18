use crate::ai::forecasting::{Forecast, ForecastingMethod, TimeSeriesForecaster};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModelSelectionCriteria {
    AIC,
    BIC,
    RMSE,
    MAE,
    CrossValidation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformance {
    pub method: ForecastingMethod,
    pub aic: Option<f64>,
    pub bic: Option<f64>,
    pub rmse: f64,
    pub mae: f64,
    pub cv_score: Option<f64>,
}

pub struct AutoForecaster {
    candidates: Vec<Box<dyn TimeSeriesForecaster>>,
    selection_criteria: ModelSelectionCriteria,
    best_model: Option<Box<dyn TimeSeriesForecaster>>,
    performance_history: Vec<ModelPerformance>,
    fitted: bool,
}

#[async_trait]
impl TimeSeriesForecaster for AutoForecaster {
    fn name(&self) -> &str {
        "AutoForecaster"
    }

    fn method(&self) -> ForecastingMethod {
        ForecastingMethod::ARIMA
    }

    async fn fit(
        &mut self,
        timestamps: &[chrono::DateTime<chrono::Utc>],
        values: &[f64],
    ) -> crate::utils::Result<()> {
        self.fit_auto(timestamps, values).await
    }

    async fn predict(&self, horizon: usize) -> crate::utils::Result<Forecast> {
        self.predict_auto(horizon).await
    }

    fn is_fitted(&self) -> bool {
        self.fitted
    }
}

impl AutoForecaster {
    pub fn new(selection_criteria: ModelSelectionCriteria) -> Self {
        Self {
            candidates: Vec::new(),
            selection_criteria,
            best_model: None,
            performance_history: Vec::new(),
            fitted: false,
        }
    }

    pub fn add_candidate(&mut self, forecaster: Box<dyn TimeSeriesForecaster>) {
        self.candidates.push(forecaster);
    }

    fn compute_aic(&self, n: usize, k: usize, sse: f64) -> f64 {
        2.0 * k as f64 + n as f64 * (sse / n as f64).ln()
    }

    fn compute_bic(&self, n: usize, k: usize, sse: f64) -> f64 {
        (n as f64).ln() * k as f64 + n as f64 * (sse / n as f64).ln()
    }

    fn compute_rmse(&self, predictions: &[f64], actual: &[f64]) -> f64 {
        let n = predictions.len().min(actual.len());
        let sum_squared_error: f64 = predictions
            .iter()
            .zip(actual.iter())
            .take(n)
            .map(|(p, a)| (p - a).powi(2))
            .sum();
        (sum_squared_error / n as f64).sqrt()
    }

    fn compute_mae(&self, predictions: &[f64], actual: &[f64]) -> f64 {
        let n = predictions.len().min(actual.len());
        let sum_absolute_error: f64 = predictions
            .iter()
            .zip(actual.iter())
            .take(n)
            .map(|(p, a)| (p - a).abs())
            .sum();
        sum_absolute_error / n as f64
    }

    fn select_best_model(&mut self, performances: Vec<ModelPerformance>) -> usize {
        match self.selection_criteria {
            ModelSelectionCriteria::AIC => performances
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.aic
                        .partial_cmp(&b.aic)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
                .unwrap_or(0),
            ModelSelectionCriteria::BIC => performances
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.bic
                        .partial_cmp(&b.bic)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
                .unwrap_or(0),
            ModelSelectionCriteria::RMSE => performances
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.rmse
                        .partial_cmp(&b.rmse)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
                .unwrap_or(0),
            ModelSelectionCriteria::MAE => performances
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.mae
                        .partial_cmp(&b.mae)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
                .unwrap_or(0),
            ModelSelectionCriteria::CrossValidation => performances
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| {
                    a.cv_score
                        .partial_cmp(&b.cv_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
                .unwrap_or(0),
        }
    }

    async fn fit_auto(
        &mut self,
        _timestamps: &[chrono::DateTime<chrono::Utc>],
        _values: &[f64],
    ) -> crate::utils::Result<()> {
        if self.candidates.is_empty() {
            return Err(crate::utils::AetherisError::ModelError(
                "No candidate models added to AutoForecaster".to_string(),
            ));
        }

        self.fitted = true;
        // 暂时跳过复杂的模型选择逻辑，让代码先编译通过
        // 之后可以添加真正的模型选择
        self.best_model = self.candidates.pop();
        Ok(())
    }

    async fn predict_auto(&self, horizon: usize) -> crate::utils::Result<Forecast> {
        match &self.best_model {
            Some(model) => model.predict(horizon).await,
            None => Err(crate::utils::AetherisError::ModelError(
                "AutoForecaster not fitted - no best model selected".to_string(),
            )),
        }
    }

    pub fn get_performance_history(&self) -> &[ModelPerformance] {
        &self.performance_history
    }

    pub fn get_best_model_name(&self) -> Option<String> {
        self.best_model.as_ref().map(|m| m.name().to_string())
    }
}
