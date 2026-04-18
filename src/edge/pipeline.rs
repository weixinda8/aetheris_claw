use super::{
    AggregationFunction, CompressionLevel, DataAggregator, DataCompressor, DataFilter, EdgeData,
    FilterConfig, FilterStrategy, OutlierDetectionMethod, OutlierDetector, StreamConfig,
    TimeWindow, WindowType,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputStats {
    pub total_records: u64,
    pub filtered_records: u64,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub last_record_time: Option<chrono::DateTime<chrono::Utc>>,
    pub average_processing_time_ms: f64,
}

impl Default for ThroughputStats {
    fn default() -> Self {
        Self {
            total_records: 0,
            filtered_records: 0,
            start_time: chrono::Utc::now(),
            last_record_time: None,
            average_processing_time_ms: 0.0,
        }
    }
}

pub struct EdgeFilterPipeline {
    outlier_detector: Option<OutlierDetector>,
    aggregator: Option<DataAggregator>,
    compressor: Option<DataCompressor>,
    config: FilterConfig,
    config_key: String,
    stats: ThroughputStats,
    processing_times: Vec<f64>,
}

impl EdgeFilterPipeline {
    pub fn new(config_key: String, config: FilterConfig) -> Self {
        Self {
            outlier_detector: None,
            aggregator: None,
            compressor: None,
            config,
            config_key,
            stats: ThroughputStats::default(),
            processing_times: Vec::new(),
        }
    }

    pub fn with_outlier_detector(mut self, method: OutlierDetectionMethod) -> Self {
        self.outlier_detector = Some(OutlierDetector::new(method));
        self
    }

    pub fn with_aggregator(
        mut self,
        window_size: TimeWindow,
        window_count: u32,
        function: AggregationFunction,
        window_type: WindowType,
    ) -> Self {
        self.aggregator = Some(DataAggregator::new(
            window_size,
            window_count,
            function,
            window_type,
        ));
        self
    }

    pub fn with_compressor(mut self, level: CompressionLevel) -> Self {
        self.compressor = Some(DataCompressor::new(level));
        self
    }

    pub fn from_stream_config(
        config_key: String,
        config: FilterConfig,
        stream_config: &StreamConfig,
    ) -> Self {
        let mut pipeline = Self::new(config_key, config);

        match stream_config.strategy {
            FilterStrategy::None => {}
            FilterStrategy::Aggregate => {
                if let Some(outlier_method) = stream_config.outlier_method {
                    pipeline = pipeline.with_outlier_detector(outlier_method);
                }
                if let (Some(agg_fn), Some(time_window), Some(window_count), Some(window_type)) = (
                    stream_config.aggregation_function,
                    stream_config.time_window,
                    stream_config.window_count,
                    stream_config.window_type,
                ) {
                    pipeline =
                        pipeline.with_aggregator(time_window, window_count, agg_fn, window_type);
                }
            }
            FilterStrategy::Compress => {
                if let Some(outlier_method) = stream_config.outlier_method {
                    pipeline = pipeline.with_outlier_detector(outlier_method);
                }
                pipeline = pipeline.with_compressor(stream_config.compression_level);
            }
            FilterStrategy::AggregateAndCompress => {
                if let Some(outlier_method) = stream_config.outlier_method {
                    pipeline = pipeline.with_outlier_detector(outlier_method);
                }
                if let (Some(agg_fn), Some(time_window), Some(window_count), Some(window_type)) = (
                    stream_config.aggregation_function,
                    stream_config.time_window,
                    stream_config.window_count,
                    stream_config.window_type,
                ) {
                    pipeline =
                        pipeline.with_aggregator(time_window, window_count, agg_fn, window_type);
                }
                pipeline = pipeline.with_compressor(stream_config.compression_level);
            }
        }

        pipeline
    }

    pub fn update_stats(
        &mut self,
        original_count: usize,
        filtered_count: usize,
        processing_time_ms: f64,
    ) {
        self.stats.total_records += original_count as u64;
        self.stats.filtered_records += filtered_count as u64;
        self.stats.last_record_time = Some(chrono::Utc::now());
        self.processing_times.push(processing_time_ms);

        if self.processing_times.len() > 1000 {
            self.processing_times.remove(0);
        }

        let avg = self.processing_times.iter().sum::<f64>() / self.processing_times.len() as f64;
        self.stats.average_processing_time_ms = avg;
    }

    pub fn get_stats(&self) -> &ThroughputStats {
        &self.stats
    }

    pub fn get_compression_ratio(&self) -> Option<f64> {
        self.compressor.as_ref().map(|c| c.get_average_ratio())
    }

    pub async fn process_single(&mut self, data: EdgeData) -> crate::utils::Result<Vec<EdgeData>> {
        let start = std::time::Instant::now();
        let mut result = vec![data.clone()];

        if let Some(detector) = &mut self.outlier_detector {
            result = detector.batch_filter(result).await?;
        }

        if let Some(aggregator) = &mut self.aggregator {
            result = aggregator.batch_filter(result).await?;
        }

        if let Some(compressor) = &mut self.compressor {
            result = compressor.batch_filter(result).await?;
        }

        let duration = start.elapsed();
        let processing_time_ms = duration.as_secs_f64() * 1000.0;
        self.update_stats(1, result.len(), processing_time_ms);

        Ok(result)
    }

    pub async fn process_batch(
        &mut self,
        data: Vec<EdgeData>,
    ) -> crate::utils::Result<Vec<EdgeData>> {
        let start = std::time::Instant::now();
        let original_count = data.len();
        let mut result = data;

        if let Some(detector) = &mut self.outlier_detector {
            result = detector.batch_filter(result).await?;
        }

        if let Some(aggregator) = &mut self.aggregator {
            result = aggregator.batch_filter(result).await?;
        }

        if let Some(compressor) = &mut self.compressor {
            result = compressor.batch_filter(result).await?;
        }

        let duration = start.elapsed();
        let processing_time_ms = duration.as_secs_f64() * 1000.0 / original_count as f64;
        self.update_stats(original_count, result.len(), processing_time_ms);

        Ok(result)
    }
}

#[async_trait]
impl DataFilter for EdgeFilterPipeline {
    fn name(&self) -> &str {
        "EdgeFilterPipeline"
    }

    async fn filter(&mut self, data: EdgeData) -> crate::utils::Result<Vec<EdgeData>> {
        self.process_single(data).await
    }

    async fn batch_filter(&mut self, data: Vec<EdgeData>) -> crate::utils::Result<Vec<EdgeData>> {
        self.process_batch(data).await
    }
}
