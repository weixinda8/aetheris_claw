use super::{CompressionLevel, DataFilter, EdgeData};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionStats {
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_ratio: f64,
    pub lossy: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompressionMethod {
    Lossless,
    Quantization,
    Sampling,
}

struct LZ77Match {
    offset: usize,
    length: usize,
    literal: f64,
}

pub struct DataCompressor {
    level: CompressionLevel,
    method: CompressionMethod,
    stats: Vec<CompressionStats>,
    dictionary: HashMap<Vec<u8>, u16>,
    quantization_bits: u8,
    sampling_rate: f64,
}

impl DataCompressor {
    pub fn new(level: CompressionLevel) -> Self {
        let (method, quantization_bits, sampling_rate) = match level {
            CompressionLevel::Lossless => (CompressionMethod::Lossless, 64, 1.0),
            CompressionLevel::Low => (CompressionMethod::Quantization, 32, 1.0),
            CompressionLevel::Medium => (CompressionMethod::Quantization, 16, 0.5),
            CompressionLevel::High => (CompressionMethod::Sampling, 8, 0.2),
        };

        Self {
            level,
            method,
            stats: Vec::new(),
            dictionary: HashMap::new(),
            quantization_bits,
            sampling_rate,
        }
    }

    fn lz77_encode(&self, data: &[f64]) -> Vec<LZ77Match> {
        let mut matches = Vec::new();
        let window_size = 4096;
        let lookahead_size = 15;
        let mut i = 0;

        while i < data.len() {
            let mut best_match = (0, 0);
            let window_start = i.saturating_sub(window_size);

            for j in window_start..i {
                let mut length = 0;
                while length < lookahead_size
                    && i + length < data.len()
                    && data[j + length] == data[i + length]
                {
                    length += 1;
                }
                if length > best_match.1 {
                    best_match = (i - j, length);
                }
            }

            if best_match.1 >= 3 {
                matches.push(LZ77Match {
                    offset: best_match.0,
                    length: best_match.1,
                    literal: 0.0,
                });
                i += best_match.1;
            } else {
                matches.push(LZ77Match {
                    offset: 0,
                    length: 0,
                    literal: data[i],
                });
                i += 1;
            }
        }

        matches
    }

    fn lz77_decode(&self, matches: &[LZ77Match]) -> Vec<f64> {
        let mut data = Vec::new();
        for m in matches {
            if m.length > 0 {
                let start = data.len() - m.offset;
                for i in 0..m.length {
                    data.push(data[start + i]);
                }
            } else {
                data.push(m.literal);
            }
        }
        data
    }

    fn quantize(&self, value: f64) -> f64 {
        if self.quantization_bits == 64 {
            return value;
        }
        let max = 1e6;
        let min = -1e6;
        let steps = (1 << self.quantization_bits) as f64;
        let normalized = (value - min) / (max - min);
        let quantized = (normalized * steps).round() / steps;
        quantized * (max - min) + min
    }

    fn compress_lossless(&mut self, data: &[f64]) -> (Vec<f64>, CompressionStats) {
        let original_size = data.len() * 8;
        let matches = self.lz77_encode(data);
        let compressed_size = matches.len() * 12;
        let compression_ratio = original_size as f64 / compressed_size as f64;
        let decoded = self.lz77_decode(&matches);

        let stats = CompressionStats {
            original_size,
            compressed_size,
            compression_ratio,
            lossy: false,
        };

        (decoded, stats)
    }

    fn compress_quantization(&mut self, data: &[f64]) -> (Vec<f64>, CompressionStats) {
        let original_size = data.len() * 8;
        let compressed: Vec<f64> = data.iter().map(|&x| self.quantize(x)).collect();
        let compressed_size = (data.len() * self.quantization_bits as usize) / 8;
        let compression_ratio = original_size as f64 / compressed_size as f64;

        let stats = CompressionStats {
            original_size,
            compressed_size,
            compression_ratio,
            lossy: true,
        };

        (compressed, stats)
    }

    fn compress_sampling(&mut self, data: &[f64]) -> (Vec<f64>, CompressionStats) {
        let original_size = data.len() * 8;
        let step = (1.0 / self.sampling_rate) as usize;
        let compressed: Vec<f64> = data.iter().step_by(step).cloned().collect();
        let compressed_size = compressed.len() * 8;
        let compression_ratio = original_size as f64 / compressed_size as f64;

        let stats = CompressionStats {
            original_size,
            compressed_size,
            compression_ratio,
            lossy: true,
        };

        (compressed, stats)
    }

    pub fn compress(&mut self, data: EdgeData) -> (EdgeData, CompressionStats) {
        let mut compressed_values = HashMap::new();
        let mut total_original = 0;
        let mut total_compressed = 0;
        let mut avg_ratio = 0.0;
        let mut count = 0;

        for (key, value) in &data.values {
            let values = vec![*value];
            let (compressed, stats) = match self.method {
                CompressionMethod::Lossless => self.compress_lossless(&values),
                CompressionMethod::Quantization => self.compress_quantization(&values),
                CompressionMethod::Sampling => self.compress_sampling(&values),
            };

            if !compressed.is_empty() {
                compressed_values.insert(key.clone(), compressed[0]);
                total_original += stats.original_size;
                total_compressed += stats.compressed_size;
                avg_ratio += stats.compression_ratio;
                count += 1;
            }
        }

        let avg_ratio = if count > 0 {
            avg_ratio / count as f64
        } else {
            1.0
        };

        let mut result = data.clone();
        result.values = compressed_values;

        let stats = CompressionStats {
            original_size: total_original,
            compressed_size: total_compressed,
            compression_ratio: avg_ratio,
            lossy: self.method != CompressionMethod::Lossless,
        };

        self.stats.push(stats.clone());

        (result, stats)
    }

    pub fn get_stats(&self) -> &[CompressionStats] {
        &self.stats
    }

    pub fn get_average_ratio(&self) -> f64 {
        if self.stats.is_empty() {
            return 1.0;
        }
        self.stats.iter().map(|s| s.compression_ratio).sum::<f64>() / self.stats.len() as f64
    }
}

#[async_trait]
impl DataFilter for DataCompressor {
    fn name(&self) -> &str {
        "DataCompressor"
    }

    async fn filter(&mut self, data: EdgeData) -> crate::utils::Result<Vec<EdgeData>> {
        let (compressed, _) = self.compress(data);
        Ok(vec![compressed])
    }

    async fn batch_filter(&mut self, data: Vec<EdgeData>) -> crate::utils::Result<Vec<EdgeData>> {
        let mut results = Vec::with_capacity(data.len());
        for item in data {
            let (compressed, _) = self.compress(item);
            results.push(compressed);
        }
        Ok(results)
    }
}
