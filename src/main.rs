use aetheris::agent::config::HotReloadManager;
use aetheris::agent::{
    Agent, AgentConfig as AgentRuntimeConfig, AgentRegistry, AgentType, BaseAgent, IndustrialAgent,
};
use aetheris::api::skill_marketplace::SkillMarketplaceState;
use aetheris::api::{AppStateBuilder, auth::AuthManager, create_router, websocket::WebSocketManager};
use aetheris::cli::config::{
    AetherisCli, AetherisCommand, AgentSubcommand, ConfigManager, ConfigSubcommand, SoulSubcommand,
};
use aetheris::config::AppConfig;
use aetheris::core::{CommanderCore, Task, TaskExecutor};
use aetheris::memory::ShortTermMemory;
use aetheris::observability::{
    OpenTelemetryConfig, OpenTelemetryManager, OtlpConfig, PrometheusConfig, init_tracing,
    telemetry::Telemetry,
};
use aetheris::protocol::industrial::{
    IndustrialProtocolManager,
    MockProtocolFactory, ModbusProtocolFactory, OpcUaProtocolFactory,
};
use aetheris::security::SecurityManager;
use aetheris::security::rate_limit::RateLimiter;
use aetheris::skill::{AgentSkillsRegistry, SkillRegistry};
use aetheris::storage::timeseries::{
    InMemoryTimeSeriesFactory, InfluxDBFactory, TimeSeriesBackendType,
    TimeSeriesManager,
};
use aetheris::streaming::{InMemoryStateBackend, StreamConfig, StreamingRuntime};
use aetheris::utils::Result;
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn, error};

struct SimpleExecutor;

#[async_trait::async_trait]
impl TaskExecutor for SimpleExecutor {
    async fn execute(&self, mut task: Task) -> Result<Task> {
        info!("Executing task: {}", task.description);
        task.status = aetheris::core::TaskStatus::Completed;
        task.result = Some("Task executed successfully".to_string());
        Ok(task)
    }

    fn can_execute(&self, _task: &Task) -> bool {
        true
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 32)]
async fn main() -> Result<()> {
    let cli = AetherisCli::parse();
    let manager = ConfigManager::new_default()?;

    if let Some(command) = &cli.command {
        match command {
            AetherisCommand::Onboard => {
                manager.run_onboard().await?;
                return Ok(());
            }
            AetherisCommand::Configure(args) => {
                manager.configure(args.clone())?;
                return Ok(());
            }
            AetherisCommand::Config { subcommand } => {
                match subcommand {
                    ConfigSubcommand::Get(args) => {
                        manager.config_get(args.clone())?;
                    }
                    ConfigSubcommand::Set(args) => {
                        manager.config_set(args.clone())?;
                    }
                    ConfigSubcommand::Unset(args) => {
                        manager.config_unset(args.clone())?;
                    }
                    ConfigSubcommand::List => {
                        manager.config_list()?;
                    }
                    ConfigSubcommand::Import(args) => {
                        manager.config_import(args.clone())?;
                    }
                    ConfigSubcommand::Export(args) => {
                        manager.config_export(args.clone())?;
                    }
                }
                return Ok(());
            }
            AetherisCommand::Soul { subcommand } => {
                match subcommand {
                    SoulSubcommand::Switch(args) => {
                        manager.soul_switch(args.clone())?;
                    }
                    SoulSubcommand::Current(args) => {
                        manager.soul_current(args.clone())?;
                    }
                    SoulSubcommand::List(args) => {
                        manager.soul_list(args.clone())?;
                    }
                    SoulSubcommand::Create(args) => {
                        manager.soul_create(args.clone())?;
                    }
                    SoulSubcommand::Edit(args) => {
                        manager.soul_edit(args.clone())?;
                    }
                    SoulSubcommand::Import(args) => {
                        manager.soul_import(args.clone())?;
                    }
                    SoulSubcommand::Export(args) => {
                        manager.soul_export(args.clone())?;
                    }
                    SoulSubcommand::Optimize(args) => {
                        manager.soul_optimize(args.clone())?;
                    }
                    SoulSubcommand::History(args) => {
                        manager.soul_history(args.clone())?;
                    }
                    SoulSubcommand::Rate(args) => {
                        manager.soul_rate(args.clone())?;
                    }
                    SoulSubcommand::Publish(args) => {
                        manager.soul_publish(args.clone())?;
                    }
                }
                return Ok(());
            }
            AetherisCommand::Doctor => {
                manager.doctor()?;
                return Ok(());
            }
            AetherisCommand::SecurityAudit(args) => {
                manager.security_audit(args.clone())?;
                return Ok(());
            }
            AetherisCommand::Agent { subcommand } => {
                match subcommand {
                    AgentSubcommand::List(args) => {
                        manager.agent_list(args.clone())?;
                    }
                    AgentSubcommand::Create(args) => {
                        manager.agent_create(args.clone())?;
                    }
                    AgentSubcommand::Template(args) => {
                        manager.agent_template(args.clone())?;
                    }
                    AgentSubcommand::Show(args) => {
                        manager.agent_show(args.clone())?;
                    }
                    AgentSubcommand::Validate(args) => {
                        manager.agent_validate(args.clone())?;
                    }
                    AgentSubcommand::Templates(args) => {
                        manager.agent_templates(args.clone())?;
                    }
                    AgentSubcommand::Export(args) => {
                        manager.agent_export(args.clone())?;
                    }
                }
                return Ok(());
            }
        }
    }

    run_server().await
}

async fn run_server() -> Result<()> {
    let config = AppConfig::load()?;
    config.validate()?;

    let otel_config = OpenTelemetryConfig {
        enabled: config.telemetry.otel_enabled,
        service_name: config.telemetry.otel_service_name.clone(),
        service_version: env!("CARGO_PKG_VERSION").to_string(),
        otlp: Some(OtlpConfig {
            enabled: config.telemetry.otel_enabled,
            endpoint: config.telemetry.otel_otlp_endpoint.clone(),
            protocol: aetheris::observability::OtlpProtocol::Grpc,
            timeout_ms: 10000,
        }),
        prometheus: Some(PrometheusConfig {
            enabled: config.telemetry.prometheus_enabled,
            port: config.telemetry.prometheus_port,
            path: config.telemetry.prometheus_path.clone(),
        }),
        sampling_ratio: 1.0,
    };
    init_tracing()?;
    let mut otel_manager = OpenTelemetryManager::new(otel_config);
    otel_manager.init()?;
    let _otel_manager = Some(otel_manager);

    info!("=============================================");
    info!("    Aetheris 复杂任务执行引擎 - 启动中...");
    info!("=============================================");

    let mut commander = CommanderCore::new();
    commander.register_executor(<dyn TaskExecutor>::from_box(SimpleExecutor));

    let security = SecurityManager::new();
    let agents = AgentRegistry::new();

    for agent_config in &config.agents {
        let agent_type = match agent_config.agent_type.as_str() {
            "code" => AgentType::Code,
            "data" => AgentType::Data,
            "ops" => AgentType::Ops,
            "office" => AgentType::Office,
            "industrial" => AgentType::Industrial,
            "compliance" => AgentType::Compliance,
            _ => AgentType::Generic,
        };
        let agent_config = AgentRuntimeConfig::new(
            agent_config.id.clone(),
            agent_config.name.clone(),
            agent_type,
        );
        let agent = BaseAgent::new_arc(agent_config);
        agents.register_agent(agent)?;
    }

    let memory = ShortTermMemory::new();
    let telemetry = Telemetry::new();
    let auth = AuthManager::new(
        config.auth.jwt_secret_key.as_bytes(),
        Some(config.auth.jwt_expiration_hours),
        Some(config.auth.jwt_issuer.clone()),
        config.auth.initial_admin_username.clone(),
        config.auth.initial_admin_password.clone(),
    );
    let ws_manager = WebSocketManager::new();
    let rate_limiter = RateLimiter::new();

    let skill_registry = SkillRegistry::new();
    let agent_skills_registry = AgentSkillsRegistry::new(PathBuf::from("./data/agent-skills"))?;
    /*
    let clawhub_importer = ClawHubImporter::new();
    let skill_marketplace = SkillMarketplaceState::new(
        std::sync::Arc::new(skill_registry.clone()),
        std::sync::Arc::new(clawhub_importer?),
        PathBuf::from("./data/skill-marketplace"),
    )?;
    */
    let skill_marketplace: Option<SkillMarketplaceState> = None;

    let industrial_protocol_manager = {
        info!("Initializing industrial protocol manager");
        let mut manager = IndustrialProtocolManager::new();

        info!("Registering OPC UA protocol factory");
        manager.register_factory(Arc::new(OpcUaProtocolFactory));

        info!("Registering Modbus protocol factory");
        manager.register_factory(Arc::new(ModbusProtocolFactory));

        info!("Registering Mock protocol factory (for testing)");
        manager.register_factory(Arc::new(MockProtocolFactory));

        let supported_protocols = manager.supported_protocols();
        info!(
            "Industrial protocol manager initialized with protocols: {:?}",
            supported_protocols
        );

        Some(manager)
    };

    let _hot_reload_manager = {
        info!("Initializing hot reload manager");
        let manager = HotReloadManager::new();
        info!("Hot reload manager initialized");
        Some(Arc::new(manager))
    };

    let timeseries_manager = {
        info!("Initializing time series manager");
        let mut manager = TimeSeriesManager::new();

        info!("Registering in-memory time series factory");
        manager.register_backend(
            TimeSeriesBackendType::InMemory,
            Arc::new(InMemoryTimeSeriesFactory),
        );

        info!("Registering InfluxDB time series factory");
        manager.register_backend(TimeSeriesBackendType::InfluxDB, Arc::new(InfluxDBFactory));

        let supported_backends = manager.supported_backends();
        info!(
            "Time series manager initialized with backends: {:?}",
            supported_backends
        );

        Some(manager)
    };

    let streaming_manager = match (
        industrial_protocol_manager.is_some(),
        timeseries_manager.is_some(),
    ) {
        (true, true) => {
            info!("Streaming runtime prerequisites available, creating streaming manager");
            let stream_config = StreamConfig::default();
            let state_backend = Arc::new(InMemoryStateBackend::new());

            let streaming_runtime = StreamingRuntime::new(stream_config, state_backend);

            info!("Streaming runtime initialized successfully");
            Some(streaming_runtime)
        }
        _ => {
            warn!(
                "Streaming runtime prerequisites (industrial protocol manager or time series manager) not available, skipping streaming manager initialization"
            );
            None
        }
    };

    if let Some(_protocol_manager) = &industrial_protocol_manager {
        info!("Initializing and registering IndustrialAgent");

        let industrial_agent = IndustrialAgent::new(
            Some("industrial-agent-001".to_string()),
            Some("Industrial Agent".to_string()),
        );

        let agent_arc: Arc<dyn Agent + Send + Sync> =
            Arc::new(industrial_agent);
        if let Err(e) = agents.register_agent(agent_arc) {
            warn!("Failed to register IndustrialAgent: {}", e);
        } else {
            info!("IndustrialAgent registered successfully");
        }
    }

    let app_state = AppStateBuilder::new()
        .commander(commander)
        .security(security)
        .agents(agents)
        .memory(memory)
        .telemetry(telemetry)
        .auth(auth)
        .ws_manager(ws_manager)
        .rate_limiter(rate_limiter)
        .opentelemetry(_otel_manager)
        .skill_registry(skill_registry)
        .agent_skills_registry(agent_skills_registry)
        .skill_marketplace(skill_marketplace)
        .industrial_protocol_manager(industrial_protocol_manager)
        .timeseries_manager(timeseries_manager)
        .streaming_manager(streaming_manager)
        .build();

    let router = create_router(app_state);

    let addr = config.socket_addr();
    info!("API Server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    let server = axum::serve(listener, router.into_make_service());
    
    let graceful_shutdown = async {
        match tokio::signal::ctrl_c().await {
            Ok(_) => {
                info!("Received shutdown signal, initiating graceful shutdown...");
            }
            Err(e) => {
                warn!("Failed to listen for shutdown signal: {}", e);
            }
        }
    };
    
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                error!("Server error: {}", e);
            }
        }
        _ = graceful_shutdown => {
            info!("Graceful shutdown initiated");
        }
    }

    info!("Server shutdown complete");
    Ok(())
}
