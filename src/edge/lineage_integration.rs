use crate::data_governance::lineage::{LineageCollector, LineageStore};
use crate::data_governance::{LineageEdgeType, LineageNodeId, LineageNodeType};
use crate::edge::*;
use crate::utils::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

pub struct LineageTrackedFilter {
    inner: Box<dyn DataFilter + Send + Sync>,
    collector: Arc<RwLock<LineageCollector>>,
    filter_node_id: Option<LineageNodeId>,
    filter_name: String,
}

impl LineageTrackedFilter {
    pub fn new(
        inner: Box<dyn DataFilter + Send + Sync>,
        collector: Arc<RwLock<LineageCollector>>,
        filter_name: String,
    ) -> Self {
        let mut result = Self {
            inner,
            collector,
            filter_node_id: None,
            filter_name,
        };

        result.register_filter();
        result
    }

    fn register_filter(&mut self) {
        let node_id = self.collector.write().collect_custom_node(
            LineageNodeType::DataTransform,
            self.filter_name.clone(),
            HashMap::new(),
        );
        self.filter_node_id = Some(node_id);
    }

    pub fn get_filter_node_id(&self) -> Option<LineageNodeId> {
        self.filter_node_id.clone()
    }
}

#[async_trait::async_trait]
impl DataFilter for LineageTrackedFilter {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn filter(&mut self, data: EdgeData) -> Result<Vec<EdgeData>> {
        let _source_node_id = LineageNodeId::from_string(data.id.clone());

        let source_node_id = self.collector.write().collect_custom_node(
            LineageNodeType::Custom("EdgeData".to_string()),
            format!("edge_data_{}", data.id),
            HashMap::new(),
        );

        if let Some(filter_node_id) = &self.filter_node_id {
            self.collector.write().collect_edge(
                LineageEdgeType::ReadsFrom,
                filter_node_id.clone(),
                source_node_id.clone(),
                None,
            );
        }

        let result = self.inner.filter(data).await?;

        for output_data in &result {
            if let Some(filter_node_id) = &self.filter_node_id {
                let output_node_id = self.collector.write().collect_custom_node(
                    LineageNodeType::Custom("EdgeData".to_string()),
                    format!("edge_data_{}", output_data.id),
                    HashMap::new(),
                );
                self.collector.write().collect_edge(
                    LineageEdgeType::TransformsTo,
                    filter_node_id.clone(),
                    output_node_id,
                    None,
                );
            }
        }

        Ok(result)
    }

    async fn batch_filter(&mut self, data: Vec<EdgeData>) -> Result<Vec<EdgeData>> {
        let mut results = Vec::new();
        for item in data {
            results.extend(self.filter(item).await?);
        }
        Ok(results)
    }
}

pub struct EdgeLineageIntegration {
    store: Arc<LineageStore>,
    collector: Arc<RwLock<LineageCollector>>,
}

impl EdgeLineageIntegration {
    pub fn new(store: Arc<LineageStore>) -> Self {
        let collector = Arc::new(RwLock::new(LineageCollector::with_store(store.clone())));
        Self { store, collector }
    }

    pub fn wrap_filter(
        &self,
        filter: Box<dyn DataFilter + Send + Sync>,
        name: String,
    ) -> LineageTrackedFilter {
        LineageTrackedFilter::new(filter, self.collector.clone(), name)
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
