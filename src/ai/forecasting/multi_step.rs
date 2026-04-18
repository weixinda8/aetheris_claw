use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MultiStepStrategy {
    Direct,
    Recursive,
    DirRec,
}

pub struct DirectMultiStepStrategy {
    horizon: usize,
}

impl DirectMultiStepStrategy {
    pub fn new(horizon: usize) -> Self {
        Self { horizon }
    }
}

pub struct RecursiveMultiStepStrategy {
    max_horizon: usize,
}

impl RecursiveMultiStepStrategy {
    pub fn new(max_horizon: usize) -> Self {
        Self { max_horizon }
    }
}

pub struct DirRecMultiStepStrategy {
    horizon: usize,
    direct_steps: usize,
}

impl DirRecMultiStepStrategy {
    pub fn new(horizon: usize, direct_steps: usize) -> Self {
        Self {
            horizon,
            direct_steps,
        }
    }
}
