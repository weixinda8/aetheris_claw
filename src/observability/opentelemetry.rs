
use crate::utils::Result;
use opentelemetry::{
    Context, KeyValue, global,
    metrics::{Counter, Meter, UpDownCounter},
};
use opentelemetry_sdk::{
    Resource,
    propagation::TraceContextPropagator,
    trace::{SdkTracerProvider, Tracer},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenTelemetryConfig {
    pub enabled: bool,
    pub service_name: String,
    pub service_version: String,
    pub otlp: Option<OtlpConfig>,
    pub prometheus: Option<PrometheusConfig>,
    pub sampling_ratio: f64,
}

impl Default for OpenTelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            service_name: "aetheris".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            otlp: Some(OtlpConfig::default()),
            prometheus: Some(PrometheusConfig::default()),
            sampling_ratio: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub protocol: OtlpProtocol,
    pub timeout_ms: u64,
}

impl Default for OtlpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "http://localhost:4317".to_string(),
            protocol: OtlpProtocol::Grpc,
            timeout_ms: 10000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OtlpProtocol {
    Grpc,
    Http,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusConfig {
    pub enabled: bool,
    pub port: u16,
    pub path: String,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 9090,
            path: "/metrics".to_string(),
        }
    }
}

pub struct Metrics {
    pub task_total: Counter<u64>,
    pub task_completed: Counter<u64>,
    pub task_failed: Counter<u64>,
    pub active_tasks: UpDownCounter<i64>,
    pub tokens_used: Counter<u64>,
    pub api_requests: Counter<u64>,
    pub api_errors: Counter<u64>,
}

impl Metrics {
    fn new(meter: Meter) -> Self {
        let task_total = meter
            .u64_counter("aetheris.task.total")
            .with_description("Total number of tasks")
            .build();

        let task_completed = meter
            .u64_counter("aetheris.task.completed")
            .with_description("Number of completed tasks")
            .build();

        let task_failed = meter
            .u64_counter("aetheris.task.failed")
            .with_description("Number of failed tasks")
            .build();

        let active_tasks = meter
            .i64_up_down_counter("aetheris.task.active")
            .with_description("Number of active tasks")
            .build();

        let tokens_used = meter
            .u64_counter("aetheris.tokens.used")
            .with_description("Total tokens used")
            .build();

        let api_requests = meter
            .u64_counter("aetheris.api.requests")
            .with_description("Total API requests")
            .build();

        let api_errors = meter
            .u64_counter("aetheris.api.errors")
            .with_description("Number of API errors")
            .build();

        Self {
            task_total,
            task_completed,
            task_failed,
            active_tasks,
            tokens_used,
            api_requests,
            api_errors,
        }
    }
}

pub struct OpenTelemetryManager {
    config: OpenTelemetryConfig,
    tracer: Option<Tracer>,
    metrics: Option<Metrics>,
    tracer_provider: Option<SdkTracerProvider>,
    pub meter_provider: Option<opentelemetry_sdk::metrics::SdkMeterProvider>,
}

impl OpenTelemetryManager {
    pub fn new(config: OpenTelemetryConfig) -> Self {
        Self {
            config,
            tracer: None,
            metrics: None,
            tracer_provider: None,
            meter_provider: None,
        }
    }

    pub fn init(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        global::set_text_map_propagator(TraceContextPropagator::new());

        let resource = Resource::builder_empty()
            .with_service_name(self.config.service_name.clone())
            .with_attribute(KeyValue::new("service.version", self.config.service_version.clone()))
            .build();

        self.init_tracing(resource.clone())?;
        self.init_metrics(resource.clone())?;

        Ok(())
    }

    fn init_tracing(&mut self, _resource: Resource) -> Result<()> {
        Ok(())
    }

    fn init_metrics(&mut self, _resource: Resource) -> Result<()> {
        Ok(())
    }

    pub fn get_tracer(&self) -> Option<&Tracer> {
        self.tracer.as_ref()
    }

    pub fn get_metrics(&self) -> Option<&Metrics> {
        self.metrics.as_ref()
    }

    pub fn record_task_start(&self, _task_id: &str, _agent_id: Option<&str>) {
        if let Some(metrics) = &self.metrics {
            let _cx = Context::current();
            metrics.task_total.add(1, &[]);
            metrics.active_tasks.add(1, &[]);
        }
    }

    pub fn record_task_completion(
        &self,
        _task_id: &str,
        success: bool,
        _duration_sec: f64,
        tokens_used: Option<u64>,
    ) {
        if let Some(metrics) = &self.metrics {
            let _cx = Context::current();
            metrics.active_tasks.add(-1, &[]);

            if success {
                metrics.task_completed.add(1, &[]);
            } else {
                metrics.task_failed.add(1, &[]);
            }

            if let Some(tokens) = tokens_used {
                metrics.tokens_used.add(tokens, &[]);
            }
        }
    }

    pub fn record_api_request(&self, _endpoint: &str, _method: &str) {
        if let Some(metrics) = &self.metrics {
            let _cx = Context::current();
            metrics.api_requests.add(1, &[]);
        }
    }

    pub fn record_api_error(&self, _endpoint: &str, _method: &str, _status_code: u16) {
        if let Some(metrics) = &self.metrics {
            let _cx = Context::current();
            metrics.api_errors.add(1, &[]);
        }
    }

    pub fn shutdown(&self) -> Result<()> {
        if let Some(tracer_provider) = &self.tracer_provider {
            let _ = tracer_provider.shutdown();
        }
        if let Some(meter_provider) = &self.meter_provider {
            let _ = meter_provider.shutdown();
        }
        Ok(())
    }
}

impl Drop for OpenTelemetryManager {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_telemetry_config_default() {
        let config = OpenTelemetryConfig::default();
        
        assert!(!config.enabled);
        assert_eq!(config.service_name, "aetheris");
        assert_eq!(config.sampling_ratio, 1.0);
        assert!(config.otlp.is_some());
        assert!(config.prometheus.is_some());
    }

    #[test]
    fn test_otlp_config_default() {
        let config = OtlpConfig::default();
        
        assert!(config.enabled);
        assert_eq!(config.endpoint, "http://localhost:4317");
        assert_eq!(config.protocol, OtlpProtocol::Grpc);
        assert_eq!(config.timeout_ms, 10000);
    }

    #[test]
    fn test_prometheus_config_default() {
        let config = PrometheusConfig::default();
        
        assert!(config.enabled);
        assert_eq!(config.port, 9090);
        assert_eq!(config.path, "/metrics");
    }

    #[test]
    fn test_otlp_protocol_equality() {
        assert_eq!(OtlpProtocol::Grpc, OtlpProtocol::Grpc);
        assert_eq!(OtlpProtocol::Http, OtlpProtocol::Http);
        assert_ne!(OtlpProtocol::Grpc, OtlpProtocol::Http);
    }

    #[test]
    fn test_open_telemetry_manager_new() {
        let config = OpenTelemetryConfig::default();
        let manager = OpenTelemetryManager::new(config);
        
        assert!(manager.get_tracer().is_none());
        assert!(manager.get_metrics().is_none());
    }

    #[test]
    fn test_open_telemetry_manager_init_disabled() {
        let mut config = OpenTelemetryConfig::default();
        config.enabled = false;
        
        let mut manager = OpenTelemetryManager::new(config);
        let result = manager.init();
        
        assert!(result.is_ok());
        assert!(manager.get_tracer().is_none());
        assert!(manager.get_metrics().is_none());
    }

    #[test]
    fn test_open_telemetry_config_serde() {
        let config = OpenTelemetryConfig {
            enabled: true,
            service_name: "test-service".to_string(),
            service_version: "1.0.0".to_string(),
            otlp: Some(OtlpConfig {
                enabled: true,
                endpoint: "http://example.com:4317".to_string(),
                protocol: OtlpProtocol::Http,
                timeout_ms: 5000,
            }),
            prometheus: Some(PrometheusConfig {
                enabled: false,
                port: 8080,
                path: "/custom-metrics".to_string(),
            }),
            sampling_ratio: 0.5,
        };
        
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: OpenTelemetryConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.enabled, deserialized.enabled);
        assert_eq!(config.service_name, deserialized.service_name);
        assert_eq!(config.sampling_ratio, deserialized.sampling_ratio);
    }
}

