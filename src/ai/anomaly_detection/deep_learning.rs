use super::{Anomaly, AnomalyDetectionMethod, AnomalyDetector};
use rand::Rng;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AutoencoderDetector {
    input_dim: usize,
    hidden_dim: usize,
    encoder_weights: Vec<Vec<f64>>,
    encoder_bias: Vec<f64>,
    decoder_weights: Vec<Vec<f64>>,
    decoder_bias: Vec<f64>,
    fitted: bool,
    reconstruction_errors: Vec<f64>,
    learning_rate: f64,
}

impl AutoencoderDetector {
    pub fn new(input_dim: usize, hidden_dim: usize) -> Self {
        Self {
            input_dim,
            hidden_dim,
            encoder_weights: Vec::new(),
            encoder_bias: Vec::new(),
            decoder_weights: Vec::new(),
            decoder_bias: Vec::new(),
            fitted: false,
            reconstruction_errors: Vec::new(),
            learning_rate: 0.001,
        }
    }

    fn initialize_weights(&mut self) {
        let mut rng = rand::thread_rng();

        self.encoder_weights = (0..self.hidden_dim)
            .map(|_| {
                (0..self.input_dim)
                    .map(|_| rng.gen_range(-0.1..0.1))
                    .collect()
            })
            .collect();

        self.encoder_bias = (0..self.hidden_dim)
            .map(|_| rng.gen_range(-0.1..0.1))
            .collect();

        self.decoder_weights = (0..self.input_dim)
            .map(|_| {
                (0..self.hidden_dim)
                    .map(|_| rng.gen_range(-0.1..0.1))
                    .collect()
            })
            .collect();

        self.decoder_bias = (0..self.input_dim)
            .map(|_| rng.gen_range(-0.1..0.1))
            .collect();
    }

    fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }

    fn sigmoid_derivative(x: f64) -> f64 {
        x * (1.0 - x)
    }

    fn relu(x: f64) -> f64 {
        x.max(0.0)
    }

    fn relu_derivative(x: f64) -> f64 {
        if x > 0.0 { 1.0 } else { 0.0 }
    }

    fn encode(&self, input: &[f64]) -> Vec<f64> {
        let mut hidden = vec![0.0; self.hidden_dim];

        for (i, hidden_val) in hidden.iter_mut().enumerate() {
            let mut sum = self.encoder_bias[i];
            for (j, &input_val) in input.iter().enumerate() {
                sum += self.encoder_weights[i][j] * input_val;
            }
            *hidden_val = Self::relu(sum);
        }

        hidden
    }

    fn decode(&self, hidden: &[f64]) -> Vec<f64> {
        let mut output = vec![0.0; self.input_dim];

        for (i, output_val) in output.iter_mut().enumerate() {
            let mut sum = self.decoder_bias[i];
            for (j, &hidden_val) in hidden.iter().enumerate() {
                sum += self.decoder_weights[i][j] * hidden_val;
            }
            *output_val = sum;
        }

        output
    }

    fn forward(&self, input: &[f64]) -> Vec<f64> {
        let hidden = self.encode(input);
        self.decode(&hidden)
    }

    fn compute_reconstruction_error(&self, input: &[f64], output: &[f64]) -> f64 {
        input
            .iter()
            .zip(output.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    fn train_step(&mut self, input: &[f64]) {
        let hidden = self.encode(input);
        let output = self.decode(&hidden);

        let mut output_errors = vec![0.0; self.input_dim];
        for (i, output_error) in output_errors.iter_mut().enumerate() {
            *output_error = output[i] - input[i];
        }

        let mut hidden_errors = vec![0.0; self.hidden_dim];
        for (i, hidden_error) in hidden_errors.iter_mut().enumerate() {
            let mut sum = 0.0;
            for (j, &output_error) in output_errors.iter().enumerate() {
                sum += output_error * self.decoder_weights[j][i];
            }
            *hidden_error = sum * Self::relu_derivative(hidden[i]);
        }

        for (i, (&output_error, decoder_bias)) in output_errors.iter().zip(self.decoder_bias.iter_mut()).enumerate() {
            *decoder_bias -= self.learning_rate * output_error;
            for (j, (&hidden_val, decoder_weight)) in hidden.iter().zip(self.decoder_weights[i].iter_mut()).enumerate() {
                *decoder_weight -= self.learning_rate * output_error * hidden_val;
            }
        }

        for (i, (&hidden_error, encoder_bias)) in hidden_errors.iter().zip(self.encoder_bias.iter_mut()).enumerate() {
            *encoder_bias -= self.learning_rate * hidden_error;
            for (j, (&input_val, encoder_weight)) in input.iter().zip(self.encoder_weights[i].iter_mut()).enumerate() {
                *encoder_weight -= self.learning_rate * hidden_error * input_val;
            }
        }
    }
}

impl Default for AutoencoderDetector {
    fn default() -> Self {
        Self::new(10, 5)
    }
}

#[async_trait::async_trait]
impl AnomalyDetector for AutoencoderDetector {
    fn name(&self) -> &str {
        "Autoencoder Detector"
    }

    fn method(&self) -> AnomalyDetectionMethod {
        AnomalyDetectionMethod::Autoencoder
    }

    async fn detect(&mut self, features: &HashMap<String, f64>) -> crate::utils::Result<Anomaly> {
        let input: Vec<f64> = features.values().cloned().collect();

        if input.len() != self.input_dim {
            return Ok(Anomaly::new(0.0, false, features.clone(), self.method()));
        }

        let output = self.forward(&input);
        let error = self.compute_reconstruction_error(&input, &output);

        let mean_error = if !self.reconstruction_errors.is_empty() {
            self.reconstruction_errors.iter().sum::<f64>() / self.reconstruction_errors.len() as f64
        } else {
            0.0
        };

        let std_error = if self.reconstruction_errors.len() > 1 {
            let variance = self
                .reconstruction_errors
                .iter()
                .map(|&e| (e - mean_error).powi(2))
                .sum::<f64>()
                / (self.reconstruction_errors.len() - 1) as f64;
            variance.sqrt()
        } else {
            0.0
        };

        let score = if std_error > 0.0 {
            (error - mean_error) / std_error
        } else {
            0.0
        };

        let is_anomaly = score > 3.0;

        self.train_step(&input);
        self.reconstruction_errors.push(error);

        if self.reconstruction_errors.len() > 1000 {
            self.reconstruction_errors.remove(0);
        }

        Ok(Anomaly::new(
            score.max(0.0),
            is_anomaly,
            features.clone(),
            self.method(),
        ))
    }

    async fn fit(&mut self, data: &[HashMap<String, f64>]) -> crate::utils::Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        let sample_dim = data[0].len();
        self.input_dim = sample_dim;

        self.initialize_weights();
        self.reconstruction_errors.clear();

        for _ in 0..100 {
            for features in data {
                let input: Vec<f64> = features.values().cloned().collect();
                if input.len() == self.input_dim {
                    self.train_step(&input);
                    let output = self.forward(&input);
                    let error = self.compute_reconstruction_error(&input, &output);
                    self.reconstruction_errors.push(error);
                }
            }
        }

        self.fitted = true;
        Ok(())
    }

    fn is_fitted(&self) -> bool {
        self.fitted
    }
}
