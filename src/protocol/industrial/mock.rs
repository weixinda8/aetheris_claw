use crate::protocol::industrial::traits::*;
use crate::protocol::industrial::types::*;
use crate::utils::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{RwLock, broadcast};

pub struct MockIndustrialProtocol {
    config: IndustrialProtocolConfig,
    status: AtomicBool,
    sender: Option<broadcast::Sender<DataPoint>>,
}

impl MockIndustrialProtocol {
    pub fn new(config: IndustrialProtocolConfig) -> Self {
        Self {
            config,
            status: AtomicBool::new(false),
            sender: None,
        }
    }

    fn generate_mock_data_point(tag_name: &str) -> DataPoint {
        let timestamp = chrono::Utc::now();
        let value = match tag_name {
            "Temperature" => DataValue::Float64(
                20.0 + (timestamp.timestamp_subsec_nanos() as f64 / 1_000_000_000.0) * 10.0,
            ),
            "Pressure" => DataValue::Float64(
                100.0 + (timestamp.timestamp_subsec_nanos() as f64 / 1_000_000_000.0) * 50.0,
            ),
            "Speed" => DataValue::Int32(1000 + (timestamp.timestamp_subsec_nanos() as i32 % 500)),
            "Status" => DataValue::Boolean(timestamp.timestamp_subsec_nanos().is_multiple_of(2)),
            _ => DataValue::String(format!("Mock value for {}", tag_name)),
        };

        DataPoint {
            tag_name: tag_name.to_string(),
            timestamp,
            value,
            quality: DataQuality::Good,
        }
    }
}

#[async_trait]
impl IndustrialProtocol for MockIndustrialProtocol {
    async fn connect(&mut self) -> Result<()> {
        self.status.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.status.store(false, Ordering::SeqCst);
        self.sender = None;
        Ok(())
    }

    async fn reconnect(&mut self) -> Result<()> {
        self.disconnect().await?;
        self.connect().await?;
        Ok(())
    }

    fn connection_status(&self) -> ConnectionStatus {
        if self.status.load(Ordering::SeqCst) {
            ConnectionStatus::Connected
        } else {
            ConnectionStatus::Disconnected
        }
    }

    fn config(&self) -> &IndustrialProtocolConfig {
        &self.config
    }

    async fn read_tag(&self, tag_name: &str) -> Result<DataPoint> {
        Ok(Self::generate_mock_data_point(tag_name))
    }

    async fn read_tags(&self, tag_names: &[String]) -> Result<Vec<DataPoint>> {
        let mut points = Vec::with_capacity(tag_names.len());
        for tag_name in tag_names {
            points.push(Self::generate_mock_data_point(tag_name));
        }
        Ok(points)
    }

    async fn write_tag(&self, _request: WriteRequest) -> Result<WriteResult> {
        Ok(WriteResult {
            tag_name: _request.tag_name,
            success: true,
            error_message: None,
        })
    }

    async fn write_tags(&self, requests: &[WriteRequest]) -> Result<Vec<WriteResult>> {
        Ok(requests
            .iter()
            .map(|req| WriteResult {
                tag_name: req.tag_name.clone(),
                success: true,
                error_message: None,
            })
            .collect())
    }

    async fn subscribe(
        &mut self,
        config: SubscriptionConfig,
    ) -> Result<broadcast::Receiver<DataPoint>> {
        let (tx, rx) = broadcast::channel(config.queue_size);
        self.sender = Some(tx.clone());

        let tag_names = config.tag_names.clone();
        let sampling_interval =
            std::time::Duration::from_millis(config.sampling_interval_ms as u64);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(sampling_interval);
            loop {
                interval.tick().await;
                for tag_name in &tag_names {
                    let point = Self::generate_mock_data_point(tag_name);
                    if tx.send(point).is_err() {
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn unsubscribe(&mut self) -> Result<()> {
        self.sender = None;
        Ok(())
    }

    async fn browse_nodes(&self, _root_path: Option<&str>) -> Result<Vec<NodeInfo>> {
        Ok(vec![NodeInfo {
            node_id: "mock_root".to_string(),
            node_name: "Mock Root".to_string(),
            node_class: NodeClass::Object,
            data_type: None,
            description: Some("Mock device root node".to_string()),
            children: vec![
                NodeInfo {
                    node_id: "temperature".to_string(),
                    node_name: "Temperature".to_string(),
                    node_class: NodeClass::Variable,
                    data_type: Some("Double".to_string()),
                    description: Some("Temperature sensor".to_string()),
                    children: vec![],
                },
                NodeInfo {
                    node_id: "pressure".to_string(),
                    node_name: "Pressure".to_string(),
                    node_class: NodeClass::Variable,
                    data_type: Some("Double".to_string()),
                    description: Some("Pressure sensor".to_string()),
                    children: vec![],
                },
            ],
        }])
    }
}

pub struct MockProtocolFactory;

impl IndustrialProtocolFactory for MockProtocolFactory {
    fn create(&self, config: IndustrialProtocolConfig) -> Arc<RwLock<dyn IndustrialProtocol>> {
        Arc::new(RwLock::new(MockIndustrialProtocol::new(config)))
    }

    fn supported_protocols(&self) -> Vec<IndustrialProtocolType> {
        vec![
            IndustrialProtocolType::OpcUa,
            IndustrialProtocolType::ModbusTcp,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_protocol_connect_disconnect() {
        let config = IndustrialProtocolConfig::default();
        let mut protocol = MockIndustrialProtocol::new(config);
        
        assert_eq!(protocol.connection_status(), ConnectionStatus::Disconnected);
        
        protocol.connect().await.unwrap();
        assert_eq!(protocol.connection_status(), ConnectionStatus::Connected);
        
        protocol.disconnect().await.unwrap();
        assert_eq!(protocol.connection_status(), ConnectionStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_mock_protocol_reconnect() {
        let config = IndustrialProtocolConfig::default();
        let mut protocol = MockIndustrialProtocol::new(config);
        
        protocol.connect().await.unwrap();
        assert_eq!(protocol.connection_status(), ConnectionStatus::Connected);
        
        protocol.reconnect().await.unwrap();
        assert_eq!(protocol.connection_status(), ConnectionStatus::Connected);
    }

    #[tokio::test]
    async fn test_mock_protocol_read_tag() {
        let config = IndustrialProtocolConfig::default();
        let mut protocol = MockIndustrialProtocol::new(config);
        protocol.connect().await.unwrap();
        
        let point = protocol.read_tag("Temperature").await.unwrap();
        
        assert_eq!(point.tag_name, "Temperature");
        assert_eq!(point.quality, DataQuality::Good);
        assert!(matches!(point.value, DataValue::Float64(_)));
    }

    #[tokio::test]
    async fn test_mock_protocol_read_tags() {
        let config = IndustrialProtocolConfig::default();
        let mut protocol = MockIndustrialProtocol::new(config);
        protocol.connect().await.unwrap();
        
        let tag_names = vec!["Temperature".to_string(), "Pressure".to_string()];
        let points = protocol.read_tags(&tag_names).await.unwrap();
        
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].tag_name, "Temperature");
        assert_eq!(points[1].tag_name, "Pressure");
    }

    #[tokio::test]
    async fn test_mock_protocol_write_tag() {
        let config = IndustrialProtocolConfig::default();
        let mut protocol = MockIndustrialProtocol::new(config);
        protocol.connect().await.unwrap();
        
        let request = WriteRequest {
            tag_name: "SetPoint".to_string(),
            value: DataValue::Int32(100),
        };
        
        let result = protocol.write_tag(request).await.unwrap();
        
        assert_eq!(result.tag_name, "SetPoint");
        assert!(result.success);
        assert!(result.error_message.is_none());
    }

    #[tokio::test]
    async fn test_mock_protocol_write_tags() {
        let config = IndustrialProtocolConfig::default();
        let mut protocol = MockIndustrialProtocol::new(config);
        protocol.connect().await.unwrap();
        
        let requests = vec![
            WriteRequest {
                tag_name: "SetPoint1".to_string(),
                value: DataValue::Int32(100),
            },
            WriteRequest {
                tag_name: "SetPoint2".to_string(),
                value: DataValue::Int32(200),
            },
        ];
        
        let results = protocol.write_tags(&requests).await.unwrap();
        
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].tag_name, "SetPoint1");
        assert!(results[0].success);
        assert_eq!(results[1].tag_name, "SetPoint2");
        assert!(results[1].success);
    }

    #[tokio::test]
    async fn test_mock_protocol_subscribe_unsubscribe() {
        let config = IndustrialProtocolConfig::default();
        let mut protocol = MockIndustrialProtocol::new(config);
        protocol.connect().await.unwrap();
        
        let sub_config = SubscriptionConfig {
            tag_names: vec!["Temperature".to_string()],
            sampling_interval_ms: 100,
            queue_size: 100,
            discard_oldest: true,
        };
        
        let mut receiver = protocol.subscribe(sub_config).await.unwrap();
        
        let result = tokio::time::timeout(std::time::Duration::from_millis(500), receiver.recv()).await;
        assert!(result.is_ok());
        
        protocol.unsubscribe().await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_protocol_browse_nodes() {
        let config = IndustrialProtocolConfig::default();
        let mut protocol = MockIndustrialProtocol::new(config);
        protocol.connect().await.unwrap();
        
        let nodes = protocol.browse_nodes(None).await.unwrap();
        
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_id, "mock_root");
        assert_eq!(nodes[0].node_name, "Mock Root");
        assert_eq!(nodes[0].node_class, NodeClass::Object);
        assert_eq!(nodes[0].children.len(), 2);
    }

    #[test]
    fn test_mock_protocol_factory() {
        let factory = MockProtocolFactory;
        
        let supported = factory.supported_protocols();
        assert_eq!(supported.len(), 2);
        assert!(supported.contains(&IndustrialProtocolType::OpcUa));
        assert!(supported.contains(&IndustrialProtocolType::ModbusTcp));
        
        let config = IndustrialProtocolConfig::default();
        let protocol = factory.create(config);
        assert!(!protocol.is_closed());
    }
}
