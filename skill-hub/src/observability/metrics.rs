use crate::config::AppConfig;
use crate::utils::{Result, SkillHubError};
use opentelemetry::{
    global,
    metrics::{Counter, Histogram, Meter, UpDownCounter},
    Context, KeyValue,
};
use opentelemetry_sdk::{
    metrics::{
        reader::MetricReader,
        SdkMeterProvider,
    },
    resource::{EnvResourceDetector, SdkProvidedResourceDetector},
    Resource,
};
use opentelemetry_prometheus::PrometheusExporter;
use prometheus::{Encoder, TextEncoder};
use std::sync::OnceLock;
use std::time::Duration;
use tracing::info;

static METRICS: OnceLock<Metrics> = OnceLock::new();

pub struct Metrics {
    pub api_requests_total: Counter<u64>,
    pub api_requests_duration: Histogram<f64>,
    pub api_errors_total: Counter<u64>,
    pub skills_total: UpDownCounter<i64>,
    pub skills_published: UpDownCounter<i64>,
    pub skills_downloaded_total: Counter<u64>,
    pub skills_rated_total: Counter<u64>,
    pub skills_executed_total: Counter<u64>,
    pub skills_execution_errors: Counter<u64>,
    pub database_queries_total: Counter<u64>,
    pub database_query_duration: Histogram<f64>,
    pub database_errors_total: Counter<u64>,
    pub audit_actions_total: Counter<u64>,
}

impl Metrics {
    fn new(meter: Meter) -> Self {
        let api_requests_total = meter
            .u64_counter("skill_hub.api.requests_total")
            .with_description("Total number of API requests")
            .init();

        let api_requests_duration = meter
            .f64_histogram("skill_hub.api.request_duration_seconds")
            .with_description("API request duration in seconds")
            .with_unit("s")
            .init();

        let api_errors_total = meter
            .u64_counter("skill_hub.api.errors_total")
            .with_description("Total number of API errors")
            .init();

        let skills_total = meter
            .i64_up_down_counter("skill_hub.skills.total")
            .with_description("Total number of skills in the system")
            .init();

        let skills_published = meter
            .i64_up_down_counter("skill_hub.skills.published")
            .with_description("Number of published skills")
            .init();

        let skills_downloaded_total = meter
            .u64_counter("skill_hub.skills.downloaded_total")
            .with_description("Total number of skill downloads")
            .init();

        let skills_rated_total = meter
            .u64_counter("skill_hub.skills.rated_total")
            .with_description("Total number of skill ratings")
            .init();

        let skills_executed_total = meter
            .u64_counter("skill_hub.skills.executed_total")
            .with_description("Total number of skill executions")
            .init();

        let skills_execution_errors = meter
            .u64_counter("skill_hub.skills.execution_errors_total")
            .with_description("Total number of skill execution errors")
            .init();

        let database_queries_total = meter
            .u64_counter("skill_hub.database.queries_total")
            .with_description("Total number of database queries")
            .init();

        let database_query_duration = meter
            .f64_histogram("skill_hub.database.query_duration_seconds")
            .with_description("Database query duration in seconds")
            .with_unit("s")
            .init();

        let database_errors_total = meter
            .u64_counter("skill_hub.database.errors_total")
            .with_description("Total number of database errors")
            .init();

        let audit_actions_total = meter
            .u64_counter("skill_hub.audit.actions_total")
            .with_description("Total number of audit actions")
            .init();

        Self {
            api_requests_total,
            api_requests_duration,
            api_errors_total,
            skills_total,
            skills_published,
            skills_downloaded_total,
            skills_rated_total,
            skills_executed_total,
            skills_execution_errors,
            database_queries_total,
            database_query_duration,
            database_errors_total,
            audit_actions_total,
        }
    }

    pub fn record_api_request(&self, endpoint: &str, method: &str, status: u16, duration_sec: f64) {
        let cx = Context::current();
        let labels = [
            KeyValue::new("endpoint", endpoint.to_string()),
            KeyValue::new("method", method.to_string()),
            KeyValue::new("status", status.to_string()),
        ];
        
        self.api_requests_total.add(1, &labels);
        self.api_requests_duration.record(duration_sec, &labels);
        
        if status >= 400 {
            self.api_errors_total.add(1, &labels);
        }
    }

    pub fn record_skill_download(&self, skill_id: &str) {
        let cx = Context::current();
        let labels = [KeyValue::new("skill_id", skill_id.to_string())];
        self.skills_downloaded_total.add(1, &labels);
    }

    pub fn record_skill_rating(&self, skill_id: &str, rating: u32) {
        let cx = Context::current();
        let labels = [
            KeyValue::new("skill_id", skill_id.to_string()),
            KeyValue::new("rating", rating.to_string()),
        ];
        self.skills_rated_total.add(1, &labels);
    }

    pub fn record_skill_execution(&self, skill_id: &str, success: bool, duration_ms: u64) {
        let cx = Context::current();
        let labels = [
            KeyValue::new("skill_id", skill_id.to_string()),
            KeyValue::new("success", success.to_string()),
        ];
        self.skills_executed_total.add(1, &labels);
        
        if !success {
            self.skills_execution_errors.add(1, &labels);
        }
    }

    pub fn record_database_query(&self, query_type: &str, duration_sec: f64, success: bool) {
        let cx = Context::current();
        let labels = [
            KeyValue::new("query_type", query_type.to_string()),
            KeyValue::new("success", success.to_string()),
        ];
        self.database_queries_total.add(1, &labels);
        self.database_query_duration.record(duration_sec, &labels);
        
        if !success {
            self.database_errors_total.add(1, &labels);
        }
    }

    pub fn record_audit_action(&self, action: &str, skill_id: Option<&str>) {
        let cx = Context::current();
        let mut labels = vec![KeyValue::new("action", action.to_string())];
        
        if let Some(skill_id) = skill_id {
            labels.push(KeyValue::new("skill_id", skill_id.to_string()));
        }
        
        self.audit_actions_total.add(1, &labels);
    }

    pub fn set_skills_total(&self, count: i64) {
        let cx = Context::current();
        self.skills_total.add(count - self.skills_total.load(&cx), &[]);
    }

    pub fn set_skills_published(&self, count: i64) {
        let cx = Context::current();
        self.skills_published.add(count - self.skills_published.load(&cx), &[]);
    }
}

static METER_PROVIDER: OnceLock<SdkMeterProvider> = OnceLock::new();

pub fn init_metrics(config: &AppConfig) -> Result<()> {
    if !config.telemetry.prometheus_enabled {
        info!("Prometheus metrics disabled");
        return Ok(());
    }

    let resource = Resource::from_detectors(
        Duration::from_secs(0),
        vec![
            Box::new(SdkProvidedResourceDetector),
            Box::new(EnvResourceDetector::new()),
        ],
    )
    .merge(&Resource::new(vec![
        KeyValue::new("service.name", config.telemetry.otel_service_name.clone()),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
    ]));

    let exporter = PrometheusExporter::default();
    let reader: Box<dyn MetricReader> = Box::new(exporter.clone());

    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader)
        .build();

    global::set_meter_provider(meter_provider.clone());
    
    let meter = global::meter_provider().meter(config.telemetry.otel_service_name.clone());
    let metrics = Metrics::new(meter);
    
    METRICS.set(metrics).map_err(|_| {
        SkillHubError::Internal("Metrics already initialized".to_string())
    })?;
    
    METER_PROVIDER.set(meter_provider).map_err(|_| {
        SkillHubError::Internal("Meter provider already initialized".to_string())
    })?;

    info!("Prometheus metrics initialized on port {}", config.telemetry.prometheus_port);
    Ok(())
}

pub fn get_metrics() -> Option<&'static Metrics> {
    METRICS.get()
}

pub fn gather_metrics() -> Result<String> {
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    
    let metric_families = prometheus::gather();
    encoder.encode(&metric_families, &mut buffer)
        .map_err(|e| SkillHubError::Internal(format!("Failed to encode metrics: {}", e)))?;
    
    String::from_utf8(buffer)
        .map_err(|e| SkillHubError::Internal(format!("Failed to convert metrics to string: {}", e)))
}

pub fn shutdown_metrics() -> Result<()> {
    if let Some(meter_provider) = METER_PROVIDER.take() {
        meter_provider.shutdown()
            .map_err(|e| SkillHubError::Internal(format!("Failed to shutdown metrics: {}", e)))?;
    }
    Ok(())
}
