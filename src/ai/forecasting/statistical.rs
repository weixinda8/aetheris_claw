use crate::ai::forecasting::{
    ConfidenceInterval, Forecast, ForecastingMethod, TimeSeriesForecaster,
};

pub struct ARIMAForecaster {
    p: usize,
    d: usize,
    q: usize,
    fitted: bool,
    historical_data: Vec<(chrono::DateTime<chrono::Utc>, f64)>,
    residuals: Vec<f64>,
}

impl ARIMAForecaster {
    pub fn new(p: usize, d: usize, q: usize) -> Self {
        Self {
            p,
            d,
            q,
            fitted: false,
            historical_data: Vec::new(),
            residuals: Vec::new(),
        }
    }

    fn compute_residuals(&self) -> Vec<f64> {
        let values: Vec<f64> = self.historical_data.iter().map(|&(_, v)| v).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        values.iter().map(|&v| v - mean).collect()
    }
}

#[async_trait::async_trait]
impl TimeSeriesForecaster for ARIMAForecaster {
    fn name(&self) -> &str {
        "ARIMA Forecaster"
    }

    fn method(&self) -> ForecastingMethod {
        ForecastingMethod::ARIMA
    }

    async fn fit(
        &mut self,
        timestamps: &[chrono::DateTime<chrono::Utc>],
        values: &[f64],
    ) -> crate::utils::Result<()> {
        self.historical_data = timestamps
            .iter()
            .zip(values.iter())
            .map(|(&t, &v)| (t, v))
            .collect();
        self.residuals = self.compute_residuals();
        self.fitted = true;
        Ok(())
    }

    async fn predict(&self, horizon: usize) -> crate::utils::Result<Forecast> {
        if !self.fitted {
            return Err(crate::utils::AetherisError::ModelError(
                "ARIMA model not fitted".to_string(),
            ));
        }

        let last_timestamp = self.historical_data.last().unwrap().0;
        let mut timestamps = Vec::with_capacity(horizon);
        let mut values = Vec::with_capacity(horizon);
        let mut confidence_intervals = Vec::with_capacity(horizon);

        let historical_values: Vec<f64> = self.historical_data.iter().map(|&(_, v)| v).collect();
        let mean = historical_values.iter().sum::<f64>() / historical_values.len() as f64;
        let std = (historical_values
            .iter()
            .map(|&v| (v - mean).powi(2))
            .sum::<f64>()
            / historical_values.len() as f64)
            .sqrt();

        for i in 0..horizon {
            let timestamp = last_timestamp + chrono::Duration::hours((i + 1) as i64);
            timestamps.push(timestamp);

            let noise = (rand::random::<f64>() - 0.5) * std * 0.1;
            let value = mean + noise * (i as f64 + 1.0).sqrt();
            values.push(value);

            let ci_width = 1.96 * std * (1.0 + (i as f64 / historical_values.len() as f64)).sqrt();
            confidence_intervals.push(ConfidenceInterval {
                lower: value - ci_width,
                upper: value + ci_width,
                confidence_level: 0.95,
            });
        }

        let mut forecast = Forecast::new(timestamps, values, self.method());
        forecast.confidence_intervals = Some(confidence_intervals);
        Ok(forecast)
    }

    fn is_fitted(&self) -> bool {
        self.fitted
    }
}

pub struct ETSForecaster {
    error_type: String,
    trend_type: String,
    season_type: String,
    season_period: usize,
    fitted: bool,
    historical_data: Vec<(chrono::DateTime<chrono::Utc>, f64)>,
    level: f64,
    trend: f64,
    seasonals: Vec<f64>,
}

impl ETSForecaster {
    pub fn new(
        error_type: String,
        trend_type: String,
        season_type: String,
        season_period: usize,
    ) -> Self {
        Self {
            error_type,
            trend_type,
            season_type,
            season_period,
            fitted: false,
            historical_data: Vec::new(),
            level: 0.0,
            trend: 0.0,
            seasonals: Vec::new(),
        }
    }
}

#[async_trait::async_trait]
impl TimeSeriesForecaster for ETSForecaster {
    fn name(&self) -> &str {
        "ETS Forecaster"
    }

    fn method(&self) -> ForecastingMethod {
        ForecastingMethod::ETS
    }

    async fn fit(
        &mut self,
        timestamps: &[chrono::DateTime<chrono::Utc>],
        values: &[f64],
    ) -> crate::utils::Result<()> {
        self.historical_data = timestamps
            .iter()
            .zip(values.iter())
            .map(|(&t, &v)| (t, v))
            .collect();

        let values: Vec<f64> = self.historical_data.iter().map(|&(_, v)| v).collect();
        self.level = values.iter().sum::<f64>() / values.len() as f64;

        if values.len() >= 2 {
            self.trend =
                (values.last().unwrap() - values.first().unwrap()) / (values.len() - 1) as f64;
        }

        self.seasonals = vec![1.0; self.season_period];
        self.fitted = true;
        Ok(())
    }

    async fn predict(&self, horizon: usize) -> crate::utils::Result<Forecast> {
        if !self.fitted {
            return Err(crate::utils::AetherisError::ModelError(
                "ETS model not fitted".to_string(),
            ));
        }

        let last_timestamp = self.historical_data.last().unwrap().0;
        let mut timestamps = Vec::with_capacity(horizon);
        let mut values = Vec::with_capacity(horizon);
        let mut confidence_intervals = Vec::with_capacity(horizon);

        let historical_values: Vec<f64> = self.historical_data.iter().map(|&(_, v)| v).collect();
        let mean = historical_values.iter().sum::<f64>() / historical_values.len() as f64;
        let std = (historical_values
            .iter()
            .map(|&v| (v - mean).powi(2))
            .sum::<f64>()
            / historical_values.len() as f64)
            .sqrt();

        for i in 0..horizon {
            let timestamp = last_timestamp + chrono::Duration::hours((i + 1) as i64);
            timestamps.push(timestamp);

            let seasonal_index = i % self.season_period;
            let trend_component = self.trend * (i + 1) as f64;
            let seasonal_component = self.seasonals[seasonal_index];
            let noise = (rand::random::<f64>() - 0.5) * std * 0.05;

            let value = self.level + trend_component + seasonal_component + noise;
            values.push(value);

            let ci_width = 1.96 * std * (1.0 + (i as f64 / historical_values.len() as f64)).sqrt();
            confidence_intervals.push(ConfidenceInterval {
                lower: value - ci_width,
                upper: value + ci_width,
                confidence_level: 0.95,
            });
        }

        let mut forecast = Forecast::new(timestamps, values, self.method());
        forecast.confidence_intervals = Some(confidence_intervals);
        Ok(forecast)
    }

    fn is_fitted(&self) -> bool {
        self.fitted
    }
}
