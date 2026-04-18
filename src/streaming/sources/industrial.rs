use crate::protocol::industrial::traits::*;
use crate::protocol::industrial::types::*;
use crate::streaming::state::KeyValueState;
use crate::streaming::traits::*;
use crate::streaming::types::*;
use crate::storage::timeseries::types::{TimeSeriesPoint, TimeSeriesValue};
use crate::utils::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

pub struct OpcUaStreamSource {
    protocol: Arc<RwLock<dyn IndustrialProtocol>>,
    subscription_config: SubscriptionConfig,
    receiver: Option<broadcast::Receiver<DataPoint>>,
}

impl OpcUaStreamSource {
    pub fn new(
        protocol: Arc<RwLock<dyn IndustrialProtocol>>,
        subscription_config: SubscriptionConfig,
    ) -> Self {
        Self {
            protocol,
            subscription_config,
            receiver: None,
        }
    }
}

#[async_trait]
impl<T> StreamSource<T> for OpcUaStreamSource
where
    T: From<DataPoint> + Clone + Send + Sync + 'static,
{
    async fn open(&mut self) -> Result<()> {
        let mut guard = self.protocol.write().await;
        self.receiver = Some(guard.subscribe(self.subscription_config.clone()).await?);
        Ok(())
    }

    async fn fetch_next(&mut self) -> Result<Option<StreamEvent<T>>> {
        if let Some(receiver) = &mut self.receiver {
            match receiver.recv().await {
                Ok(data_point) => {
                    let timestamp = data_point.timestamp;
                    let mut event = StreamEvent::new(T::from(data_point));
                    event = event.with_event_time(timestamp);
                    Ok(Some(event))
                }
                Err(broadcast::error::RecvError::Closed) => Ok(None),
                Err(broadcast::error::RecvError::Lagged(_)) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    async fn close(&mut self) -> Result<()> {
        self.protocol.write().await.unsubscribe().await?;
        self.receiver = None;
        Ok(())
    }
}

pub struct ModbusPollingSource {
    protocol: Arc<RwLock<dyn IndustrialProtocol>>,
    tag_names: Vec<String>,
    poll_interval_ms: u64,
    last_fetch: Option<chrono::DateTime<chrono::Utc>>,
}

impl ModbusPollingSource {
    pub fn new(
        protocol: Arc<RwLock<dyn IndustrialProtocol>>,
        tag_names: Vec<String>,
        poll_interval_ms: u64,
    ) -> Self {
        Self {
            protocol,
            tag_names,
            poll_interval_ms,
            last_fetch: None,
        }
    }
}

#[async_trait]
impl<T> StreamSource<T> for ModbusPollingSource
where
    T: From<DataPoint> + Clone + Send + Sync + 'static,
{
    async fn open(&mut self) -> Result<()> {
        self.last_fetch = Some(chrono::Utc::now());
        Ok(())
    }

    async fn fetch_next(&mut self) -> Result<Option<StreamEvent<T>>> {
        let now = chrono::Utc::now();
        let should_poll = if let Some(last) = self.last_fetch {
            (now - last).num_milliseconds() >= self.poll_interval_ms as i64
        } else {
            true
        };

        if should_poll {
            self.last_fetch = Some(now);

            let data_points = self.protocol.read().await.read_tags(&self.tag_names).await?;

            if let Some(data_point) = data_points.into_iter().next() {
                let timestamp = data_point.timestamp;
                let mut event = StreamEvent::new(T::from(data_point));
                event = event.with_event_time(timestamp);
                return Ok(Some(event));
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        Ok(None)
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

pub struct TimeSeriesSink {
    database: Arc<RwLock<dyn crate::storage::timeseries::traits::TimeSeriesDatabase + Send + Sync>>,
    measurement: String,
    batch: Vec<TimeSeriesPoint>,
    batch_size: usize,
}

impl TimeSeriesSink {
    pub fn new(
        database: Arc<RwLock<dyn crate::storage::timeseries::traits::TimeSeriesDatabase + Send + Sync>>,
        measurement: String,
        batch_size: usize,
    ) -> Self {
        Self {
            database,
            measurement,
            batch: Vec::with_capacity(batch_size),
            batch_size,
        }
    }

    fn data_point_to_timeseries(&self, data_point: &DataPoint) -> TimeSeriesPoint {
        let mut point = TimeSeriesPoint::new(self.measurement.clone(), data_point.timestamp);

        point = point.add_tag("tag_name", data_point.tag_name.clone());
        point = point.add_tag("quality", format!("{:?}", data_point.quality));

        match &data_point.value {
            DataValue::Boolean(b) => point.add_field("value", TimeSeriesValue::Boolean(*b)),
            DataValue::Int8(i) => point.add_field("value", TimeSeriesValue::Int64(*i as i64)),
            DataValue::Int16(i) => point.add_field("value", TimeSeriesValue::Int64(*i as i64)),
            DataValue::Int32(i) => point.add_field("value", TimeSeriesValue::Int64(*i as i64)),
            DataValue::Int64(i) => point.add_field("value", TimeSeriesValue::Int64(*i)),
            DataValue::UInt8(u) => point.add_field("value", TimeSeriesValue::UInt64(*u as u64)),
            DataValue::UInt16(u) => point.add_field("value", TimeSeriesValue::UInt64(*u as u64)),
            DataValue::UInt32(u) => point.add_field("value", TimeSeriesValue::UInt64(*u as u64)),
            DataValue::UInt64(u) => point.add_field("value", TimeSeriesValue::UInt64(*u)),
            DataValue::Float32(f) => point.add_field("value", TimeSeriesValue::Float64(*f as f64)),
            DataValue::Float64(f) => point.add_field("value", TimeSeriesValue::Float64(*f)),
            DataValue::String(s) => point.add_field("value", TimeSeriesValue::String(s.clone())),
            DataValue::ByteArray(_) => point,
        }
    }
}

#[async_trait]
impl<T> StreamSink<T> for TimeSeriesSink
where
    T: Into<DataPoint> + Clone + Send + Sync + 'static,
{
    async fn open(&mut self) -> Result<()> {
        Ok(())
    }

    async fn write(&mut self, event: StreamEvent<T>) -> Result<()> {
        let data_point: DataPoint = event.data.into();
        let ts_point = self.data_point_to_timeseries(&data_point);

        self.batch.push(ts_point);

        if self.batch.len() >= self.batch_size {
            <Self as StreamSink<T>>::flush(self).await?;
        }

        Ok(())
    }

    async fn write_batch(&mut self, events: Vec<StreamEvent<T>>) -> Result<()> {
        for event in events {
            let data_point: DataPoint = event.data.into();
            let ts_point = self.data_point_to_timeseries(&data_point);
            self.batch.push(ts_point);

            if self.batch.len() >= self.batch_size {
                <Self as StreamSink<T>>::flush(self).await?;
            }
        }
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        if !self.batch.is_empty() {
            self.database
                .write()
                .await
                .write_points(std::mem::take(&mut self.batch))
                .await?;
            self.batch = Vec::with_capacity(self.batch_size);
        }
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        <Self as StreamSink<T>>::flush(self).await?;
        Ok(())
    }
}

pub struct AlertOperator {
    alert_rules: Vec<AlertRule>,
    state: Arc<dyn StateBackend + Send + Sync>,
}

pub struct AlertRule {
    pub name: String,
    pub condition: Box<dyn Fn(&DataPoint) -> bool + Send + Sync>,
    pub severity: AlertSeverity,
    pub cooldown_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl AlertOperator {
    pub fn new(alert_rules: Vec<AlertRule>, state: Arc<dyn StateBackend + Send + Sync>) -> Self {
        Self { alert_rules, state }
    }
}

#[async_trait]
impl<T> StreamOperator<T, T> for AlertOperator
where
    T: Into<DataPoint> + From<DataPoint> + Clone + Send + Sync + 'static,
{
    async fn process(
        &mut self,
        event: StreamEvent<T>,
        state: &mut KeyValueState<String, String>,
    ) -> Result<StreamEvent<T>> {
        let data_point: DataPoint = event.data.clone().into();

        for rule in &self.alert_rules {
            let last_alert_key = format!("alert_last_{}", rule.name);
            let now = chrono::Utc::now();

            let should_alert = if let Some(last_alert_str) = state.get(&last_alert_key).await? {
                if let Ok(last_alert) = chrono::DateTime::parse_from_rfc3339(&last_alert_str) {
                    let last_alert_utc = last_alert.with_timezone(&chrono::Utc);
                    (now - last_alert_utc).num_milliseconds() >= rule.cooldown_ms as i64
                } else {
                    true
                }
            } else {
                true
            };

            if should_alert && (rule.condition)(&data_point) {
                let now_str = now.to_rfc3339();
                state.put(&last_alert_key, &now_str).await?;
                log::warn!(
                    "[Alert: {:?}] {} - Tag: {}, Value: {:?}",
                    rule.severity,
                    rule.name,
                    data_point.tag_name,
                    data_point.value
                );
            }
        }

        Ok(event)
    }
}
