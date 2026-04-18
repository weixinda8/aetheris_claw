pub mod auto_select;
pub mod confidence;
pub mod deep_learning;
pub mod ml;
pub mod multi_step;
pub mod statistical;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfidenceInterval {
    pub lower: f64,
    pub upper: f64,
    pub confidence_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Forecast {
    pub id: String,
    pub timestamps: Vec<chrono::DateTime<chrono::Utc>>,
    pub values: Vec<f64>,
    pub confidence_intervals: Option<Vec<ConfidenceInterval>>,
    pub method: ForecastingMethod,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl Forecast {
    pub fn new(
        timestamps: Vec<chrono::DateTime<chrono::Utc>>,
        values: Vec<f64>,
        method: ForecastingMethod,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamps,
            values,
            confidence_intervals: None,
            method,
            created_at: chrono::Utc::now(),
            metadata: None,
        }
    }

    pub fn with_confidence_intervals(mut self, intervals: Vec<ConfidenceInterval>) -> Self {
        self.confidence_intervals = Some(intervals);
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ForecastingMethod {
    ARIMA,
    ETS,
    XGBoost,
    LightGBM,
    LSTM,
    Transformer,
}

#[async_trait::async_trait]
pub trait TimeSeriesForecaster: Send + Sync {
    fn name(&self) -> &str;
    fn method(&self) -> ForecastingMethod;
    async fn fit(
        &mut self,
        timestamps: &[chrono::DateTime<chrono::Utc>],
        values: &[f64],
    ) -> crate::utils::Result<()>;
    async fn predict(&self, horizon: usize) -> crate::utils::Result<Forecast>;
    fn is_fitted(&self) -> bool;
}

#[async_trait::async_trait]
pub trait MultiStepForecast: TimeSeriesForecaster {
    async fn predict_multi_step(
        &self,
        horizon: usize,
        strategy: multi_step::MultiStepStrategy,
    ) -> crate::utils::Result<Forecast>;
}

pub use auto_select::{AutoForecaster, ModelSelectionCriteria};
pub use confidence::{
    ConfidenceEstimator, MonteCarloDropoutEstimator, QuantileRegressionEstimator,
};
pub use deep_learning::{LSTMForecaster, TransformerForecaster};
pub use ml::{LightGBMForecaster, XGBoostForecaster};
pub use multi_step::{
    DirRecMultiStepStrategy, DirectMultiStepStrategy, MultiStepStrategy, RecursiveMultiStepStrategy,
};
pub use statistical::{ARIMAForecaster, ETSForecaster};
