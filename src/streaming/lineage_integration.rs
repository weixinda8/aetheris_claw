use crate::data_governance::lineage::{LineageCollector, LineageStore};
use crate::data_governance::{LineageEdgeType, LineageNodeId, LineageNodeType};
use crate::streaming::state::KeyValueState;
use crate::streaming::traits::*;
use crate::streaming::types::*;
use crate::utils::Result;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

pub struct LineageTrackedSource<T> {
    inner: Box<dyn StreamSource<T> + Send + Sync>,
    collector: Arc<RwLock<LineageCollector>>,
    source_node_id: Option<LineageNodeId>,
    source_name: String,
}

impl<T> LineageTrackedSource<T> {
    pub fn new(
        inner: Box<dyn StreamSource<T> + Send + Sync>,
        collector: Arc<RwLock<LineageCollector>>,
        source_name: String,
    ) -> Self {
        let mut result = Self {
            inner,
            collector,
            source_node_id: None,
            source_name,
        };

        result.register_source();
        result
    }

    fn register_source(&mut self) {
        let node_id = self.collector.write().collect_data_source(
            self.source_name.clone(),
            "stream_source".to_string(),
            HashMap::new(),
        );
        self.source_node_id = Some(node_id);
    }

    pub fn get_source_node_id(&self) -> Option<LineageNodeId> {
        self.source_node_id.clone()
    }
}

#[async_trait]
impl<T: Clone + Send + Sync + 'static> StreamSource<T> for LineageTrackedSource<T> {
    async fn open(&mut self) -> Result<()> {
        self.inner.open().await
    }

    async fn fetch_next(&mut self) -> Result<Option<StreamEvent<T>>> {
        let result = self.inner.fetch_next().await?;

        if let Some(mut event) = result {
            if let Some(source_id) = &self.source_node_id {
                event
                    .metadata
                    .insert("lineage_source_id".to_string(), source_id.0.clone());
            }
            Ok(Some(event))
        } else {
            Ok(None)
        }
    }

    async fn close(&mut self) -> Result<()> {
        self.inner.close().await
    }
}

pub struct LineageTrackedOperator<In, Out> {
    inner: Box<dyn StreamOperator<In, Out> + Send + Sync>,
    collector: Arc<RwLock<LineageCollector>>,
    transform_node_id: Option<LineageNodeId>,
    operator_name: String,
}

impl<In, Out> LineageTrackedOperator<In, Out> {
    pub fn new(
        inner: Box<dyn StreamOperator<In, Out> + Send + Sync>,
        collector: Arc<RwLock<LineageCollector>>,
        operator_name: String,
    ) -> Self {
        let mut result = Self {
            inner,
            collector,
            transform_node_id: None,
            operator_name,
        };

        result.register_transform();
        result
    }

    fn register_transform(&mut self) {
        let node_id = self.collector.write().collect_custom_node(
            LineageNodeType::DataTransform,
            self.operator_name.clone(),
            HashMap::new(),
        );
        self.transform_node_id = Some(node_id);
    }

    pub fn get_transform_node_id(&self) -> Option<LineageNodeId> {
        self.transform_node_id.clone()
    }
}

#[async_trait]
impl<In: Clone + Send + Sync + 'static, Out: Clone + Send + Sync + 'static> StreamOperator<In, Out>
    for LineageTrackedOperator<In, Out>
{
    async fn process(
        &mut self,
        mut event: StreamEvent<In>,
        state: &mut KeyValueState<String, String>,
    ) -> Result<StreamEvent<Out>> {
        let source_node_ids: Vec<LineageNodeId> = event
            .metadata
            .get("lineage_source_id")
            .map(|id| vec![LineageNodeId::from_string(id.clone())])
            .unwrap_or_default();

        if let Some(transform_id) = &self.transform_node_id {
            for source_id in source_node_ids {
                self.collector.write().collect_edge(
                    LineageEdgeType::TransformsTo,
                    source_id,
                    transform_id.clone(),
                    None,
                );
            }

            event
                .metadata
                .insert("lineage_source_id".to_string(), transform_id.0.clone());
        }

        let result = self.inner.process(event, state).await?;

        Ok(result)
    }
}

pub struct LineageTrackedSink<T> {
    inner: Box<dyn StreamSink<T> + Send + Sync>,
    collector: Arc<RwLock<LineageCollector>>,
    sink_node_id: Option<LineageNodeId>,
    sink_name: String,
}

impl<T> LineageTrackedSink<T> {
    pub fn new(
        inner: Box<dyn StreamSink<T> + Send + Sync>,
        collector: Arc<RwLock<LineageCollector>>,
        sink_name: String,
    ) -> Self {
        let mut result = Self {
            inner,
            collector,
            sink_node_id: None,
            sink_name,
        };

        result.register_sink();
        result
    }

    fn register_sink(&mut self) {
        let node_id = self.collector.write().collect_data_sink(
            self.sink_name.clone(),
            "stream_sink".to_string(),
            Vec::new(),
            HashMap::new(),
        );
        self.sink_node_id = Some(node_id);
    }

    pub fn get_sink_node_id(&self) -> Option<LineageNodeId> {
        self.sink_node_id.clone()
    }
}

#[async_trait]
impl<T: Clone + Send + Sync + 'static> StreamSink<T> for LineageTrackedSink<T> {
    async fn open(&mut self) -> Result<()> {
        self.inner.open().await
    }

    async fn write(&mut self, event: StreamEvent<T>) -> Result<()> {
        if let Some(sink_id) = &self.sink_node_id {
            if let Some(source_id_str) = event.metadata.get("lineage_source_id") {
                let source_id = LineageNodeId::from_string(source_id_str.clone());
                self.collector.write().collect_edge(
                    LineageEdgeType::WritesTo,
                    source_id,
                    sink_id.clone(),
                    None,
                );
            }
        }

        self.inner.write(event).await
    }

    async fn write_batch(&mut self, events: Vec<StreamEvent<T>>) -> Result<()> {
        for event in events {
            self.write(event).await?;
        }
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        let _ = self.collector.write().flush();
        self.inner.flush().await
    }

    async fn close(&mut self) -> Result<()> {
        let _ = self.collector.write().flush();
        self.inner.close().await
    }
}

pub struct StreamingLineageIntegration {
    store: Arc<LineageStore>,
    collector: Arc<RwLock<LineageCollector>>,
}

impl StreamingLineageIntegration {
    pub fn new(store: Arc<LineageStore>) -> Self {
        let collector = Arc::new(RwLock::new(LineageCollector::with_store(store.clone())));
        Self { store, collector }
    }

    pub fn wrap_source<T: Clone + Send + Sync + 'static>(
        &self,
        source: Box<dyn StreamSource<T> + Send + Sync>,
        name: String,
    ) -> LineageTrackedSource<T> {
        LineageTrackedSource::new(source, self.collector.clone(), name)
    }

    pub fn wrap_operator<In: Clone + Send + Sync + 'static, Out: Clone + Send + Sync + 'static>(
        &self,
        operator: Box<dyn StreamOperator<In, Out> + Send + Sync>,
        name: String,
    ) -> LineageTrackedOperator<In, Out> {
        LineageTrackedOperator::new(operator, self.collector.clone(), name)
    }

    pub fn wrap_sink<T: Clone + Send + Sync + 'static>(
        &self,
        sink: Box<dyn StreamSink<T> + Send + Sync>,
        name: String,
    ) -> LineageTrackedSink<T> {
        LineageTrackedSink::new(sink, self.collector.clone(), name)
    }

    pub fn get_collector(&self) -> Arc<RwLock<LineageCollector>> {
        self.collector.clone()
    }

    pub fn get_store(&self) -> Arc<LineageStore> {
        self.store.clone()
    }

    pub fn flush(&self) -> Result<()> {
        self.collector.write().flush()
    }
}
