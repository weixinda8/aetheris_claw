use aetheris_skill_hub::api::{AppState, create_router};
use aetheris_skill_hub::config::AppConfig;
use aetheris_skill_hub::observability::{init_metrics, init_tracing, shutdown_metrics, shutdown_tracing};
use aetheris_skill_hub::utils::Result;
use clap::Parser;
use sqlx::PgPool;
use qdrant_client::Qdrant;
use tracing::info;

#[derive(Debug, Parser)]
#[clap(name = "aetheris-skill-hub")]
#[clap(about = "Aetheris Skill Hub - 独立的技能发现、评分和分发服务", long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Parser)]
enum Command {
    Migrate,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = AppConfig::load()?;
    config.validate()?;

    init_tracing(&config)?;
    init_metrics(&config)?;

    let result = if let Some(command) = &cli.command {
        match command {
            Command::Migrate => {
                run_migrations(&config).await?;
                Ok(())
            }
        }
    } else {
        run_server(config).await
    };

    shutdown_metrics()?;
    shutdown_tracing()?;

    result
}

async fn run_migrations(config: &AppConfig) -> Result<()> {
    info!("=============================================");
    info!("    Aetheris Skill Hub - 数据库迁移中...");
    info!("=============================================");

    let pool = PgPool::connect(&config.database.url).await?;
    sqlx::migrate!("./src/migrations").run(&pool).await?;

    info!("数据库迁移完成！");
    Ok(())
}

async fn run_server(config: AppConfig) -> Result<()> {
    info!("=============================================");
    info!("    Aetheris Skill Hub - 启动中...");
    info!("=============================================");

    let db_pool = PgPool::connect(&config.database.url).await?;
    info!("PostgreSQL 连接成功");

    let qdrant_client = Qdrant::from_url(&config.qdrant.url).build()?;
    info!("Qdrant 连接成功");

    let app_state = AppState::new(db_pool, qdrant_client, config.clone());
    let router = create_router(app_state);

    let addr = config.socket_addr();
    info!("API Server starting on http://{}", addr);
    
    if config.telemetry.prometheus_enabled {
        info!("Prometheus metrics available at http://{}:{}{}", 
            config.server.host, 
            config.telemetry.prometheus_port, 
            config.telemetry.prometheus_path
        );
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router.into_make_service()).await?;

    Ok(())
}
