use aetheris::protocol::industrial::{
    DataPoint, IndustrialProtocolConfig, IndustrialProtocolManager, IndustrialProtocolType,
    MockProtocolFactory, SubscriptionConfig,
};
use aetheris::storage::timeseries::{
    InMemoryTimeSeriesFactory, TimeSeriesBackendType, TimeSeriesConfig, TimeSeriesManager,
};
use aetheris::streaming::{
    InMemoryStateBackend, OpcUaStreamSource, SimplePipeline, StreamConfig, StreamingRuntime,
    TimeSeriesSink,
};
use aetheris::utils::Result;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("=============================================");
    info!("  Aetheris 工业数据管道示例");
    info!("=============================================");

    info!("1. 初始化工业协议管理器");
    let mut protocol_manager = IndustrialProtocolManager::new();
    protocol_manager.register_factory(Arc::new(MockProtocolFactory));

    info!("2. 创建模拟工业协议连接");
    let mock_config = IndustrialProtocolConfig {
        protocol_type: IndustrialProtocolType::OpcUa,
        endpoint: "127.0.0.1".to_string(),
        port: 4840,
        timeout_ms: 5000,
        reconnect_interval_ms: 3000,
        max_reconnect_attempts: 10,
        security_config: None,
        extra_config: std::collections::HashMap::new(),
    };
    let mock_protocol = protocol_manager.create_protocol(mock_config)?;

    info!("3. 初始化时序数据库管理器");
    let mut ts_manager = TimeSeriesManager::new();
    ts_manager.register_backend(
        TimeSeriesBackendType::InMemory,
        Arc::new(InMemoryTimeSeriesFactory),
    );

    info!("4. 创建数据源 (Source)");
    let sub_config = SubscriptionConfig {
        tag_names: vec![
            "Temperature".to_string(),
            "Pressure".to_string(),
            "Speed".to_string(),
        ],
        sampling_interval_ms: 1000,
        queue_size: 1000,
        discard_oldest: true,
    };
    let source = OpcUaStreamSource::new(mock_protocol.clone(), sub_config);

    info!("5. 创建数据接收器 (Sink)");
    let ts_config = TimeSeriesConfig {
        backend_type: TimeSeriesBackendType::InMemory,
        endpoint: "127.0.0.1".to_string(),
        port: 8086,
        database: "industrial_metrics".to_string(),
        username: None,
        password: None,
        token: None,
        batch_size: 1000,
        max_retries: 3,
        retry_interval: std::time::Duration::from_millis(100),
        retention_policies: vec![],
        downsampling_rules: vec![],
    };
    let ts_database = ts_manager.create_database(ts_config)?;
    let sink = TimeSeriesSink::new(Arc::from(ts_database), "industrial_data".to_string(), 100);

    info!("6. 构建数据处理管道");
    let mut pipeline = SimplePipeline::new(
        "example_industrial_pipeline".to_string(),
        Arc::new(source),
        Arc::new(sink),
    );

    pipeline = pipeline.add_filter(|data_point: &DataPoint| match &data_point.value {
        aetheris::protocol::industrial::types::DataValue::Float64(v) => *v > 0.0,
        aetheris::protocol::industrial::types::DataValue::Int32(v) => *v > 0,
        _ => true,
    });

    info!("7. 初始化流处理运行时");
    let stream_config = StreamConfig::default();
    let state_backend = Arc::new(InMemoryStateBackend::new());
    let mut streaming_runtime = StreamingRuntime::new(stream_config, state_backend.clone());

    info!("8. 注册并启动管道");
    streaming_runtime.register_pipeline(pipeline).await?;
    streaming_runtime
        .start_stream("example_industrial_pipeline")
        .await?;

    info!("=============================================");
    info!("  管道已成功启动！");
    info!("  - 数据源: 模拟 OPC UA 设备");
    info!("  - 数据标签: Temperature, Pressure, Speed");
    info!("  - 采样间隔: 1000ms");
    info!("  - 数据存储: 内存时序数据库");
    info!("  - 过滤器: 过滤掉非正值数据");
    info!("=============================================");
    info!("  按 Ctrl+C 停止...");
    info!("=============================================");

    tokio::signal::ctrl_c().await?;
    info!("收到停止信号，正在关闭管道...");
    streaming_runtime
        .stop_stream("example_industrial_pipeline")
        .await?;
    streaming_runtime.shutdown_all().await?;

    info!("管道已成功关闭！");
    Ok(())
}
