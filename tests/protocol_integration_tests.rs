use aetheris::protocol::industrial::*;
use aetheris::utils::Result;
use std::sync::Arc;
use tokio::sync::broadcast;

#[tokio::test]
async fn test_mock_modbus_connection() -> Result<()> {
    let config = IndustrialProtocolConfig {
        protocol_type: IndustrialProtocolType::ModbusTcp,
        endpoint: "127.0.0.1".to_string(),
        port: 502,
        timeout_ms: 5000,
        reconnect_interval_ms: 3000,
        max_reconnect_attempts: 3,
        security_config: None,
        extra_config: std::collections::HashMap::new(),
    };

    let mut protocol = MockIndustrialProtocol::new(config);

    assert_eq!(protocol.connection_status(), ConnectionStatus::Disconnected);

    protocol.connect().await?;
    assert_eq!(protocol.connection_status(), ConnectionStatus::Connected);

    protocol.disconnect().await?;
    assert_eq!(protocol.connection_status(), ConnectionStatus::Disconnected);

    protocol.reconnect().await?;
    assert_eq!(protocol.connection_status(), ConnectionStatus::Connected);

    Ok(())
}

#[tokio::test]
async fn test_mock_opcua_connection() -> Result<()> {
    let config = IndustrialProtocolConfig {
        protocol_type: IndustrialProtocolType::OpcUa,
        endpoint: "127.0.0.1".to_string(),
        port: 4840,
        timeout_ms: 5000,
        reconnect_interval_ms: 3000,
        max_reconnect_attempts: 3,
        security_config: None,
        extra_config: std::collections::HashMap::new(),
    };

    let mut protocol = MockIndustrialProtocol::new(config);

    protocol.connect().await?;
    assert_eq!(protocol.connection_status(), ConnectionStatus::Connected);

    Ok(())
}

#[tokio::test]
async fn test_mock_read_write_tags() -> Result<()> {
    let config = IndustrialProtocolConfig::default();
    let mut protocol = MockIndustrialProtocol::new(config);
    protocol.connect().await?;

    let temp_point = protocol.read_tag("Temperature").await?;
    assert_eq!(temp_point.tag_name, "Temperature");
    assert!(matches!(temp_point.value, DataValue::Float64(_)));
    assert_eq!(temp_point.quality, DataQuality::Good);

    let tags = vec![
        "Pressure".to_string(),
        "Speed".to_string(),
        "Status".to_string(),
    ];
    let points = protocol.read_tags(&tags).await?;
    assert_eq!(points.len(), 3);

    let write_request = WriteRequest {
        tag_name: "Temperature".to_string(),
        value: DataValue::Float64(25.5),
    };
    let write_result = protocol.write_tag(write_request).await?;
    assert_eq!(write_result.tag_name, "Temperature");
    assert!(write_result.success);

    let write_requests = vec![
        WriteRequest {
            tag_name: "Pressure".to_string(),
            value: DataValue::Float64(120.0),
        },
        WriteRequest {
            tag_name: "Speed".to_string(),
            value: DataValue::Int32(1500),
        },
    ];
    let write_results = protocol.write_tags(&write_requests).await?;
    assert_eq!(write_results.len(), 2);
    for result in write_results {
        assert!(result.success);
    }

    Ok(())
}

#[tokio::test]
async fn test_mock_subscription() -> Result<()> {
    let config = IndustrialProtocolConfig::default();
    let mut protocol = MockIndustrialProtocol::new(config);
    protocol.connect().await?;

    let subscription_config = SubscriptionConfig {
        tag_names: vec!["Temperature".to_string(), "Pressure".to_string()],
        sampling_interval_ms: 100,
        queue_size: 100,
        discard_oldest: true,
    };

    let mut receiver = protocol.subscribe(subscription_config).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let mut received_count = 0;
    for _ in 0..5 {
        match receiver.try_recv() {
            Ok(point) => {
                assert!(point.tag_name == "Temperature" || point.tag_name == "Pressure");
                received_count += 1;
            }
            Err(_) => break,
        }
    }

    protocol.unsubscribe().await?;

    Ok(())
}

#[tokio::test]
async fn test_mock_browse_nodes() -> Result<()> {
    let config = IndustrialProtocolConfig::default();
    let protocol = MockIndustrialProtocol::new(config);

    let nodes = protocol.browse_nodes(None).await?;
    assert_eq!(nodes.len(), 1);

    let root = &nodes[0];
    assert_eq!(root.node_id, "mock_root");
    assert_eq!(root.node_name, "Mock Root");
    assert_eq!(root.node_class, NodeClass::Object);
    assert_eq!(root.children.len(), 2);

    let temperature = &root.children[0];
    assert_eq!(temperature.node_id, "temperature");
    assert_eq!(temperature.node_name, "Temperature");
    assert_eq!(temperature.node_class, NodeClass::Variable);

    let pressure = &root.children[1];
    assert_eq!(pressure.node_id, "pressure");
    assert_eq!(pressure.node_name, "Pressure");

    Ok(())
}

#[test]
fn test_protocol_manager() -> Result<()> {
    let mut manager = IndustrialProtocolManager::new();
    let factory = Arc::new(MockProtocolFactory);
    manager.register_factory(factory);

    let supported = manager.supported_protocols();
    assert_eq!(supported.len(), 2);
    assert!(supported.contains(&IndustrialProtocolType::OpcUa));
    assert!(supported.contains(&IndustrialProtocolType::ModbusTcp));

    let config = IndustrialProtocolConfig {
        protocol_type: IndustrialProtocolType::OpcUa,
        ..Default::default()
    };
    let protocol = manager.create_protocol(config)?;
    assert_eq!(
        protocol.config().protocol_type,
        IndustrialProtocolType::OpcUa
    );

    Ok(())
}
