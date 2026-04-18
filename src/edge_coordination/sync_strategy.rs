use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum SyncStrategy {
    #[default]
    RealTime,
    Batched { interval_seconds: u64 },
    OnDemand,
    EventDriven,
}

