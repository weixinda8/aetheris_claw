use crate::data_governance::lineage::LineageStore;
use crate::data_governance::{
    LineageEdge, LineageEdgeId, LineageEdgeType, LineageNode, LineageNodeId, LineageNodeType,
};
use dashmap::DashMap;
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

type LineageHookFn = Box<dyn Fn(&LineageNode) + Send + Sync>;

pub struct LineageCollector {
    pending_nodes: Vec<LineageNode>,
    pending_edges: Vec<LineageEdge>,
    hooks: Vec<LineageHookFn>,
    store: Option<Arc<LineageStore>>,
    auto_flush: bool,
    node_cache: DashMap<String, LineageNodeId>,
    context_stack: Mutex<Vec<LineageNodeId>>,
}

impl LineageCollector {
    pub fn new() -> Self {
        Self {
            pending_nodes: Vec::new(),
            pending_edges: Vec::new(),
            hooks: Vec::new(),
            store: None,
            auto_flush: false,
            node_cache: DashMap::new(),
            context_stack: Mutex::new(Vec::new()),
        }
    }

    pub fn with_store(store: Arc<LineageStore>) -> Self {
        Self {
            pending_nodes: Vec::new(),
            pending_edges: Vec::new(),
            hooks: Vec::new(),
            store: Some(store),
            auto_flush: false,
            node_cache: DashMap::new(),
            context_stack: Mutex::new(Vec::new()),
        }
    }

    pub fn set_store(&mut self, store: Arc<LineageStore>) {
        self.store = Some(store);
    }

    pub fn set_auto_flush(&mut self, auto_flush: bool) {
        self.auto_flush = auto_flush;
    }

    pub fn register_hook<F>(&mut self, hook: F)
    where
        F: Fn(&LineageNode) + Send + Sync + 'static,
    {
        self.hooks.push(Box::new(hook));
    }

    pub fn push_context(&self, node_id: LineageNodeId) {
        self.context_stack.lock().push(node_id);
    }

    pub fn pop_context(&self) -> Option<LineageNodeId> {
        self.context_stack.lock().pop()
    }

    pub fn current_context(&self) -> Option<LineageNodeId> {
        self.context_stack.lock().last().cloned()
    }

    pub fn flush(&mut self) -> crate::utils::Result<()> {
        if let Some(store) = &self.store {
            for node in self.pending_nodes.drain(..) {
                store.store_node(node)?;
            }
            for edge in self.pending_edges.drain(..) {
                store.store_edge(edge)?;
            }
        }
        Ok(())
    }

    pub fn collect_data_source(
        &mut self,
        name: String,
        source_type: String,
        metadata: HashMap<String, Value>,
    ) -> LineageNodeId {
        let cache_key = format!("source:{}", name);

        if let Some(cached_id) = self.node_cache.get(&cache_key) {
            return cached_id.clone();
        }

        let mut node = LineageNode::new(LineageNodeType::DataSource, name);
        node.metadata = metadata;
        node.metadata
            .insert("source_type".to_string(), Value::String(source_type));

        for hook in &self.hooks {
            hook(&node);
        }

        let node_id = node.id.clone();
        self.pending_nodes.push(node);
        self.node_cache.insert(cache_key, node_id.clone());

        if self.auto_flush {
            let _ = self.flush();
        }

        node_id
    }

    pub fn collect_transform(
        &mut self,
        name: String,
        transform_type: String,
        source_nodes: Vec<LineageNodeId>,
        metadata: HashMap<String, Value>,
    ) -> LineageNodeId {
        let cache_key = format!("transform:{}", name);

        if let Some(cached_id) = self.node_cache.get(&cache_key) {
            return cached_id.clone();
        }

        let mut node = LineageNode::new(LineageNodeType::DataTransform, name);
        node.metadata = metadata;
        node.metadata
            .insert("transform_type".to_string(), Value::String(transform_type));

        for hook in &self.hooks {
            hook(&node);
        }

        let node_id = node.id.clone();
        self.pending_nodes.push(node);
        self.node_cache.insert(cache_key, node_id.clone());

        for source_id in source_nodes {
            let edge = LineageEdge::new(LineageEdgeType::TransformsTo, source_id, node_id.clone());
            self.pending_edges.push(edge);
        }

        if let Some(context_id) = self.current_context() {
            let edge = LineageEdge::new(LineageEdgeType::DependsOn, node_id.clone(), context_id);
            self.pending_edges.push(edge);
        }

        if self.auto_flush {
            let _ = self.flush();
        }

        node_id
    }

    pub fn collect_data_sink(
        &mut self,
        name: String,
        sink_type: String,
        source_nodes: Vec<LineageNodeId>,
        metadata: HashMap<String, Value>,
    ) -> LineageNodeId {
        let cache_key = format!("sink:{}", name);

        if let Some(cached_id) = self.node_cache.get(&cache_key) {
            return cached_id.clone();
        }

        let mut node = LineageNode::new(LineageNodeType::DataSink, name);
        node.metadata = metadata;
        node.metadata
            .insert("sink_type".to_string(), Value::String(sink_type));

        for hook in &self.hooks {
            hook(&node);
        }

        let node_id = node.id.clone();
        self.pending_nodes.push(node);
        self.node_cache.insert(cache_key, node_id.clone());

        for source_id in source_nodes {
            let edge = LineageEdge::new(LineageEdgeType::WritesTo, source_id, node_id.clone());
            self.pending_edges.push(edge);
        }

        if self.auto_flush {
            let _ = self.flush();
        }

        node_id
    }

    pub fn collect_custom_node(
        &mut self,
        node_type: LineageNodeType,
        name: String,
        metadata: HashMap<String, Value>,
    ) -> LineageNodeId {
        let cache_key = format!("custom:{:?}:{}", node_type, name);

        if let Some(cached_id) = self.node_cache.get(&cache_key) {
            return cached_id.clone();
        }

        let mut node = LineageNode::new(node_type, name);
        node.metadata = metadata;

        for hook in &self.hooks {
            hook(&node);
        }

        let node_id = node.id.clone();
        self.pending_nodes.push(node);
        self.node_cache.insert(cache_key, node_id.clone());

        if self.auto_flush {
            let _ = self.flush();
        }

        node_id
    }

    pub fn collect_edge(
        &mut self,
        edge_type: LineageEdgeType,
        source_id: LineageNodeId,
        target_id: LineageNodeId,
        description: Option<String>,
    ) -> LineageEdgeId {
        let mut edge = LineageEdge::new(edge_type, source_id, target_id);
        if let Some(desc) = description {
            edge = edge.with_description(desc);
        }
        let edge_id = edge.id.clone();
        self.pending_edges.push(edge);

        if self.auto_flush {
            let _ = self.flush();
        }

        edge_id
    }

    pub fn collect_data_read(
        &mut self,
        source_id: LineageNodeId,
        reader_name: String,
        metadata: HashMap<String, Value>,
    ) -> LineageNodeId {
        let reader_id = self.collect_custom_node(
            LineageNodeType::Custom("DataReader".to_string()),
            reader_name,
            metadata,
        );

        self.collect_edge(
            LineageEdgeType::ReadsFrom,
            reader_id.clone(),
            source_id,
            None,
        );

        reader_id
    }

    pub fn collect_data_write(
        &mut self,
        writer_name: String,
        target_id: LineageNodeId,
        source_nodes: Vec<LineageNodeId>,
        metadata: HashMap<String, Value>,
    ) -> LineageNodeId {
        let writer_id = self.collect_custom_node(
            LineageNodeType::Custom("DataWriter".to_string()),
            writer_name,
            metadata,
        );

        for source_id in source_nodes {
            self.collect_edge(
                LineageEdgeType::WritesTo,
                source_id,
                writer_id.clone(),
                None,
            );
        }

        self.collect_edge(
            LineageEdgeType::WritesTo,
            writer_id.clone(),
            target_id,
            None,
        );

        writer_id
    }

    pub fn take_pending_nodes(&mut self) -> Vec<LineageNode> {
        std::mem::take(&mut self.pending_nodes)
    }

    pub fn take_pending_edges(&mut self) -> Vec<LineageEdge> {
        std::mem::take(&mut self.pending_edges)
    }

    pub fn clear(&mut self) {
        self.pending_nodes.clear();
        self.pending_edges.clear();
    }
}

impl Default for LineageCollector {
    fn default() -> Self {
        Self::new()
    }
}
