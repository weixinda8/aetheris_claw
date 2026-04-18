use crate::config::AppConfig;
use crate::utils::{Result, SkillHubError};
use opentelemetry::{
    global,
    trace::TracerProvider,
    Context, KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    resource::{EnvResourceDetector, SdkProvidedResourceDetector},
    trace::{Config, RandomIdGenerator, Sampler},
    Resource,
};
use std::sync::OnceLock;
use std::time::Duration;
use tracing::info;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

static TRACER_PROVIDER: OnceLock<opentelemetry_sdk::trace::TracerProvider> = OnceLock::new();

pub fn init_tracing(config: &AppConfig) -> Result<()> {
    if !config.telemetry.otel_enabled {
        info!("OpenTelemetry tracing disabled");
        return init_basic_tracing();
    }

    global::set_text_map_propagator(TraceContextPropagator::new());

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

    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&config.telemetry.otel_otlp_endpoint)
                .with_timeout(Duration::from_secs(10)),
        )
        .with_trace_config(
            Config::default()
                .with_sampler(Sampler::TraceIdRatioBased(1.0))
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .install_simple()
        .map_err(|e| SkillHubError::Internal(format!("Failed to init OTLP tracing: {}", e)))?;

    let tracer = tracer_provider.tracer(config.telemetry.otel_service_name.clone());
    
    TRACER_PROVIDER.set(tracer_provider).map_err(|_| {
        SkillHubError::Internal("Tracer provider already initialized".to_string())
    })?;

    global::set_tracer_provider(TRACER_PROVIDER.get().unwrap().clone());

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,aetheris_skill_hub=debug,tower_http=debug".into());

    let otel_layer = OpenTelemetryLayer::new(tracer);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(otel_layer)
        .try_init()
        .map_err(|e| SkillHubError::Internal(format!("Failed to init tracing subscriber: {}", e)))?;

    info!("OpenTelemetry tracing initialized with endpoint: {}", config.telemetry.otel_otlp_endpoint);
    Ok(())
}

fn init_basic_tracing() -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,aetheris_skill_hub=debug,tower_http=debug".into());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .map_err(|e| SkillHubError::Internal(format!("Failed to init basic tracing: {}", e)))?;

    info!("Basic tracing initialized");
    Ok(())
}

pub fn shutdown_tracing() -> Result<()> {
    if let Some(tracer_provider) = TRACER_PROVIDER.take() {
        tracer_provider.shutdown()
            .map_err(|e| SkillHubError::Internal(format!("Failed to shutdown tracing: {}", e)))?;
    }
    Ok(())
}

pub fn with_span_context<F, R>(f: F) -> R
where
    F: FnOnce(Context) -> R,
{
    let cx = Context::current();
    f(cx)
}
