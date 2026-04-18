use crate::cluster::traits::*;
use crate::cluster::types::*;
use crate::utils::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

pub struct ClusterManager {
    config: ClusterConfig,
    load_balancing_config: LoadBalancingConfig,
    role: Arc<RwLock<NodeRole>>,
    current_term: Arc<AtomicU64>,
    leader_id: Arc<RwLock<Option<String>>>,
    peers: Arc<DashMap<String, NodeInfo>>,
    start_time: chrono::DateTime<chrono::Utc>,
    total_heartbeats_sent: Arc<AtomicU64>,
    total_heartbeats_received: Arc<AtomicU64>,
    total_leader_changes: Arc<AtomicU64>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    join_set: Option<JoinSet<()>>,
}

impl ClusterManager {
    pub fn new(config: ClusterConfig, load_balancing_config: LoadBalancingConfig) -> Self {
        Self {
            config,
            load_balancing_config,
            role: Arc::new(RwLock::new(NodeRole::Follower)),
            current_term: Arc::new(AtomicU64::new(0)),
            leader_id: Arc::new(RwLock::new(None)),
            peers: Arc::new(DashMap::new()),
            start_time: chrono::Utc::now(),
            total_heartbeats_sent: Arc::new(AtomicU64::new(0)),
            total_heartbeats_received: Arc::new(AtomicU64::new(0)),
            total_leader_changes: Arc::new(AtomicU64::new(0)),
            shutdown_tx: None,
            join_set: None,
        }
    }

    pub fn load_balancer(&self) -> SimpleLoadBalancer {
        SimpleLoadBalancer::new(self.load_balancing_config.clone())
    }

    async fn send_heartbeat(&self) {
        self.total_heartbeats_sent.fetch_add(1, Ordering::SeqCst);
    }

    async fn receive_heartbeat(&self, from_node_id: &str) {
        self.total_heartbeats_received
            .fetch_add(1, Ordering::SeqCst);

        if let Some(mut peer) = self.peers.get_mut(from_node_id) {
            peer.last_heartbeat = chrono::Utc::now();
            peer.is_alive = true;
        }
    }

    async fn become_leader(&self) {
        let mut role = self.role.write().await;
        let old_role = *role;
        *role = NodeRole::Leader;

        if old_role != NodeRole::Leader {
            *self.leader_id.write().await = Some(self.config.node_id.clone());
            self.total_leader_changes.fetch_add(1, Ordering::SeqCst);
            log::info!(
                "Node {} became leader for term {}",
                self.config.node_id,
                self.current_term.load(Ordering::SeqCst)
            );
        }
    }

    async fn become_follower(&self, new_leader_id: Option<String>) {
        let mut role = self.role.write().await;
        let old_role = *role;
        *role = NodeRole::Follower;

        let mut leader_id = self.leader_id.write().await;
        if *leader_id != new_leader_id {
            *leader_id = new_leader_id.clone();
            if old_role == NodeRole::Leader {
                self.total_leader_changes.fetch_add(1, Ordering::SeqCst);
            }
            log::info!(
                "Node {} became follower, leader is {:?}",
                self.config.node_id,
                new_leader_id
            );
        }
    }
}

#[async_trait]
impl ClusterNode for ClusterManager {
    fn node_id(&self) -> &str {
        &self.config.node_id
    }

    fn config(&self) -> &ClusterConfig {
        &self.config
    }

    async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        let mut join_set = JoinSet::new();

        self.shutdown_tx = Some(shutdown_tx);
        *self.role.write().await = NodeRole::Follower;
        self.start_time = chrono::Utc::now();

        for (i, addr) in self.config.peer_addresses.iter().enumerate() {
            let peer_id = format!("peer_{}", i);
            self.peers.insert(
                peer_id.clone(),
                NodeInfo {
                    node_id: peer_id,
                    address: *addr,
                    role: NodeRole::Follower,
                    is_alive: true,
                    last_heartbeat: chrono::Utc::now(),
                    metadata: std::collections::HashMap::new(),
                },
            );
        }

        let role = self.role.clone();
        let _current_term = self.current_term.clone();
        let _leader_id = self.leader_id.clone();
        let config = self.config.clone();
        let peers = self.peers.clone();
        let total_heartbeats_sent = self.total_heartbeats_sent.clone();

        join_set.spawn(async move {
            let mut heartbeat_interval = tokio::time::interval(config.heartbeat_interval);

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    _ = heartbeat_interval.tick() => {
                        let current_role = *role.blocking_read();

                        if current_role == NodeRole::Leader {
                            total_heartbeats_sent.fetch_add(1, Ordering::SeqCst);

                            for mut peer in peers.iter_mut() {
                                peer.last_heartbeat = chrono::Utc::now();
                            }
                        }
                    }
                }
            }
        });

        self.join_set = Some(join_set);

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            std::mem::drop(tx.send(()));
        }

        if let Some(mut join_set) = self.join_set.take() {
            while let Some(result) = join_set.join_next().await {
                if let Err(e) = result {
                    log::error!("Cluster task error: {}", e);
                }
            }
        }

        *self.role.write().await = NodeRole::Follower;

        Ok(())
    }

    fn role(&self) -> NodeRole {
        *self.role.blocking_read()
    }

    fn is_leader(&self) -> bool {
        *self.role.blocking_read() == NodeRole::Leader
    }

    fn leader_id(&self) -> Option<String> {
        self.leader_id.blocking_read().clone()
    }

    fn peers(&self) -> Vec<NodeInfo> {
        self.peers.iter().map(|p| p.value().clone()).collect()
    }

    fn metrics(&self) -> ClusterMetrics {
        let uptime = (chrono::Utc::now() - self.start_time)
            .to_std()
            .unwrap_or_default();

        ClusterMetrics {
            node_id: self.config.node_id.clone(),
            role: self.role(),
            current_term: self.current_term.load(Ordering::SeqCst),
            leader_id: self.leader_id(),
            cluster_size: self.peers.len() + 1,
            healthy_nodes: self.peers.iter().filter(|p| p.is_alive).count() + 1,
            uptime,
            total_heartbeats_sent: self.total_heartbeats_sent.load(Ordering::SeqCst),
            total_heartbeats_received: self.total_heartbeats_received.load(Ordering::SeqCst),
            total_leader_changes: self.total_leader_changes.load(Ordering::SeqCst),
            log_entries_count: 0,
            last_log_index: 0,
        }
    }
}
