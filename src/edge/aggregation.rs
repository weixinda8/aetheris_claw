use super::{DataFilter, EdgeData};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TimeWindow {
    Minutes,
    Hours,
    Days,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AggregationFunction {
    Avg,
    Sum,
    Min,
    Max,
    Count,
    Median,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WindowType {
    Sliding,
    Tumbling,
}

struct WindowData {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    data: Vec<EdgeData>,
}

pub struct DataAggregator {
    window_size: TimeWindow,
    window_count: u32,
    function: AggregationFunction,
    window_type: WindowType,
    windows: VecDeque<WindowData>,
    buffer: Vec<EdgeData>,
}

impl DataAggregator {
    pub fn new(
        window_size: TimeWindow,
        window_count: u32,
        function: AggregationFunction,
        window_type: WindowType,
    ) -> Self {
        Self {
            window_size,
            window_count,
            function,
            window_type,
            windows: VecDeque::new(),
            buffer: Vec::new(),
        }
    }

    fn get_window_duration(&self) -> Duration {
        match self.window_size {
            TimeWindow::Minutes => Duration::minutes(1),
            TimeWindow::Hours => Duration::hours(1),
            TimeWindow::Days => Duration::days(1),
        }
    }

    fn get_window_key(&self, timestamp: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        let duration = self.get_window_duration();
        let start = match self.window_size {
            TimeWindow::Minutes => {
                let ts = timestamp.timestamp();
                let start_ts = ts - (ts % 60);
                DateTime::from_timestamp(start_ts, 0).unwrap()
            }
            TimeWindow::Hours => {
                let ts = timestamp.timestamp();
                let start_ts = ts - (ts % 3600);
                DateTime::from_timestamp(start_ts, 0).unwrap()
            }
            TimeWindow::Days => {
                let ts = timestamp.timestamp();
                let start_ts = ts - (ts % 86400);
                DateTime::from_timestamp(start_ts, 0).unwrap()
            }
        };
        let end = start + duration;
        (start, end)
    }

    fn aggregate_values(&self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }

        match self.function {
            AggregationFunction::Avg => values.iter().sum::<f64>() / values.len() as f64,
            AggregationFunction::Sum => values.iter().sum(),
            AggregationFunction::Min => values.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
            AggregationFunction::Max => values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
            AggregationFunction::Count => values.len() as f64,
            AggregationFunction::Median => {
                let mut sorted = values.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let mid = sorted.len() / 2;
                if sorted.len().is_multiple_of(2) {
                    (sorted[mid - 1] + sorted[mid]) / 2.0
                } else {
                    sorted[mid]
                }
            }
        }
    }

    fn create_aggregated_data(&self, window_data: &WindowData) -> Option<EdgeData> {
        if window_data.data.is_empty() {
            return None;
        }

        let mut feature_values: HashMap<String, Vec<f64>> = HashMap::new();
        let stream_id = window_data.data[0].stream_id.clone();

        for data in &window_data.data {
            for (key, &value) in &data.values {
                feature_values.entry(key.clone()).or_default().push(value);
            }
        }

        let mut aggregated_values = HashMap::new();
        for (key, values) in feature_values {
            aggregated_values.insert(key, self.aggregate_values(&values));
        }

        let mut aggregated = EdgeData::new(stream_id, aggregated_values);
        aggregated.timestamp = window_data.start;
        Some(aggregated)
    }

    pub fn add_data(&mut self, data: EdgeData) {
        let (window_start, window_end) = self.get_window_key(data.timestamp);

        let window = self.windows.iter_mut().find(|w| w.start == window_start);

        if let Some(window) = window {
            window.data.push(data);
        } else {
            let new_window = WindowData {
                start: window_start,
                end: window_end,
                data: vec![data],
            };

            self.windows.push_back(new_window);

            while self.windows.len() > self.window_count as usize {
                self.windows.pop_front();
            }
        }
    }

    pub fn get_aggregated(&self) -> Vec<EdgeData> {
        self.windows
            .iter()
            .filter_map(|w| self.create_aggregated_data(w))
            .collect()
    }

    pub fn aggregate_and_clear(&mut self) -> Vec<EdgeData> {
        let results = self.get_aggregated();
        self.windows.clear();
        self.buffer.clear();
        results
    }
}

#[async_trait]
impl DataFilter for DataAggregator {
    fn name(&self) -> &str {
        "DataAggregator"
    }

    async fn filter(&mut self, data: EdgeData) -> crate::utils::Result<Vec<EdgeData>> {
        self.add_data(data);
        Ok(self.get_aggregated())
    }

    async fn batch_filter(&mut self, data: Vec<EdgeData>) -> crate::utils::Result<Vec<EdgeData>> {
        for item in data {
            self.add_data(item);
        }
        Ok(self.aggregate_and_clear())
    }
}
