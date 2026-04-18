use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeRole {
    Leader,
    Follower,
    Candidate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub address: SocketAddr,
    pub role: NodeRole,
    pub is_alive: bool,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub node_id: String,
    pub listen_address: SocketAddr,
    pub peer_addresses: Vec<SocketAddr>,
    pub election_timeout_min: Duration,
    pub election_timeout_max: Duration,
    pub heartbeat_interval: Duration,
    pub max_failover_time: Duration,
    pub enable_tls: bool,
    pub ca_cert_path: Option<String>,
    pub node_cert_path: Option<String>,
    pub node_key_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftLogEntry {
    pub index: u64,
    pub term: u64,
    pub command_type: CommandType,
    pub data: Vec<u8>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandType {
    NoOp,
    Set,
    Delete,
    ConfigChange,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftState {
    pub current_term: u64,
    pub voted_for: Option<String>,
    pub log: Vec<RaftLogEntry>,
    pub commit_index: u64,
    pub last_applied: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMetrics {
    pub node_id: String,
    pub role: NodeRole,
    pub current_term: u64,
    pub leader_id: Option<String>,
    pub cluster_size: usize,
    pub healthy_nodes: usize,
    pub uptime: Duration,
    pub total_heartbeats_sent: u64,
    pub total_heartbeats_received: u64,
    pub total_leader_changes: u64,
    pub log_entries_count: u64,
    pub last_log_index: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    pub strategy: LoadBalancingStrategy,
    pub min_tasks_per_node: usize,
    pub max_tasks_per_node: usize,
    pub rebalance_interval: Duration,
    pub task_weight_factor: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastLoaded,
    WeightedRoundRobin,
    ConsistentHashing,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            listen_address: "127.0.0.1:8080".parse().unwrap(),
            peer_addresses: Vec::new(),
            election_timeout_min: Duration::from_millis(150),
            election_timeout_max: Duration::from_millis(300),
            heartbeat_interval: Duration::from_millis(50),
            max_failover_time: Duration::from_secs(5),
            enable_tls: false,
            ca_cert_path: None,
            node_cert_path: None,
            node_key_path: None,
        }
    }
}

impl Default for LoadBalancingConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategy::LeastLoaded,
            min_tasks_per_node: 1,
            max_tasks_per_node: 100,
            rebalance_interval: Duration::from_secs(10),
            task_weight_factor: 1.0,
        }
    }
}
