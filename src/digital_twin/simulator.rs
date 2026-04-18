use crate::digital_twin::{TwinEntity, TwinModel, TwinState};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SimulationMode {
    RealTime,
    Accelerated,
    StepByStep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub mode: SimulationMode,
    pub time_multiplier: f64,
    pub max_simulation_seconds: u64,
    pub random_seed: Option<u64>,
    pub enable_visualization: bool,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            mode: SimulationMode::RealTime,
            time_multiplier: 1.0,
            max_simulation_seconds: 86400,
            random_seed: None,
            enable_visualization: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatIfScenario {
    pub id: String,
    pub name: String,
    pub description: String,
    pub modifications: Vec<EntityModification>,
    pub created_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityModification {
    pub entity_id: String,
    pub property_changes: HashMap<String, serde_json::Value>,
    pub state_change: Option<TwinState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub scenario_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_seconds: f64,
    pub entity_states: Vec<(String, TwinState, HashMap<String, serde_json::Value>)>,
    pub metrics: SimulationMetrics,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationMetrics {
    pub total_entities: usize,
    pub state_changes: u64,
    pub warnings: u64,
    pub alerts: u64,
    pub critical_events: u64,
    pub average_latency_ms: f64,
}

pub struct DigitalTwinSimulator {
    model: Arc<TwinModel>,
    config: SimulationConfig,
    current_scenario: Arc<RwLock<Option<WhatIfScenario>>>,
    simulation_history: Arc<RwLock<Vec<SimulationResult>>>,
    is_running: Arc<RwLock<bool>>,
}

impl DigitalTwinSimulator {
    pub fn new(model: Arc<TwinModel>, config: SimulationConfig) -> Self {
        Self {
            model,
            config,
            current_scenario: Arc::new(RwLock::new(None)),
            simulation_history: Arc::new(RwLock::new(Vec::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub fn create_scenario(&self, name: String, description: String) -> WhatIfScenario {
        WhatIfScenario {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            modifications: Vec::new(),
            created_at: Utc::now(),
            last_run: None,
        }
    }

    pub fn add_modification(
        &self,
        scenario: &mut WhatIfScenario,
        modification: EntityModification,
    ) {
        scenario.modifications.push(modification);
    }

    pub async fn run_simulation(
        &self,
        mut scenario: WhatIfScenario,
    ) -> crate::utils::Result<SimulationResult> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(crate::utils::AetherisError::Validation(
                "Simulation already running".to_string(),
            ));
        }
        *is_running = true;
        *self.current_scenario.write().await = Some(scenario.clone());
        drop(is_running);

        let start_time = Utc::now();
        let mut metrics = SimulationMetrics {
            total_entities: self.model.list_entities().len(),
            state_changes: 0,
            warnings: 0,
            alerts: 0,
            critical_events: 0,
            average_latency_ms: 0.0,
        };

        let original_entities: Vec<TwinEntity> = self.model.list_entities();
        let mut final_states = Vec::new();

        for modification in &scenario.modifications {
            if let Some(mut entity) = self.model.get_entity(&modification.entity_id) {
                if let Some(new_state) = &modification.state_change {
                    entity.state = new_state.clone();
                    metrics.state_changes += 1;
                }
                entity
                    .properties
                    .extend(modification.property_changes.clone());
                self.model.update_entity(
                    &modification.entity_id,
                    modification.property_changes.clone(),
                );
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        for entity in self.model.list_entities() {
            final_states.push((
                entity.id.clone(),
                entity.state.clone(),
                entity.properties.clone(),
            ));
        }

        for entity in original_entities {
            self.model.add_entity(entity);
        }

        let end_time = Utc::now();
        let duration_seconds = (end_time - start_time).num_milliseconds() as f64 / 1000.0;

        scenario.last_run = Some(end_time);

        let result = SimulationResult {
            scenario_id: scenario.id,
            start_time,
            end_time,
            duration_seconds,
            entity_states: final_states,
            metrics,
            success: true,
            error_message: None,
        };

        self.simulation_history.write().await.push(result.clone());
        *self.is_running.write().await = false;
        *self.current_scenario.write().await = None;

        Ok(result)
    }

    pub async fn compare_results(
        &self,
        scenario_id_1: &str,
        scenario_id_2: &str,
    ) -> Option<SimulationComparison> {
        let history = self.simulation_history.read().await;
        let result1 = history.iter().find(|r| r.scenario_id == scenario_id_1)?;
        let result2 = history.iter().find(|r| r.scenario_id == scenario_id_2)?;

        Some(SimulationComparison {
            scenario_id_1: scenario_id_1.to_string(),
            scenario_id_2: scenario_id_2.to_string(),
            state_differences: Self::compare_entity_states(result1, result2),
            metric_differences: Self::compare_metrics(result1, result2),
        })
    }

    fn compare_entity_states(
        result1: &SimulationResult,
        result2: &SimulationResult,
    ) -> Vec<EntityStateDifference> {
        let mut differences = Vec::new();

        for (id1, state1, props1) in &result1.entity_states {
            if let Some((_, state2, props2)) =
                result2.entity_states.iter().find(|(id, _, _)| id == id1)
            {
                if state1 != state2 {
                    differences.push(EntityStateDifference {
                        entity_id: id1.clone(),
                        state_change: Some((state1.clone(), state2.clone())),
                        property_changes: Self::diff_properties(props1, props2),
                    });
                } else {
                    let prop_changes = Self::diff_properties(props1, props2);
                    if !prop_changes.is_empty() {
                        differences.push(EntityStateDifference {
                            entity_id: id1.clone(),
                            state_change: None,
                            property_changes: prop_changes,
                        });
                    }
                }
            }
        }

        differences
    }

    fn diff_properties(
        props1: &HashMap<String, serde_json::Value>,
        props2: &HashMap<String, serde_json::Value>,
    ) -> Vec<PropertyChange> {
        let mut changes = Vec::new();

        for (key, value1) in props1 {
            if let Some(value2) = props2.get(key) {
                if value1 != value2 {
                    changes.push(PropertyChange {
                        property_name: key.clone(),
                        old_value: Some(value1.clone()),
                        new_value: Some(value2.clone()),
                    });
                }
            } else {
                changes.push(PropertyChange {
                    property_name: key.clone(),
                    old_value: Some(value1.clone()),
                    new_value: None,
                });
            }
        }

        for (key, value2) in props2 {
            if !props1.contains_key(key) {
                changes.push(PropertyChange {
                    property_name: key.clone(),
                    old_value: None,
                    new_value: Some(value2.clone()),
                });
            }
        }

        changes
    }

    fn compare_metrics(
        result1: &SimulationResult,
        result2: &SimulationResult,
    ) -> MetricDifferences {
        MetricDifferences {
            state_changes: result2.metrics.state_changes as i64
                - result1.metrics.state_changes as i64,
            warnings: result2.metrics.warnings as i64 - result1.metrics.warnings as i64,
            alerts: result2.metrics.alerts as i64 - result1.metrics.alerts as i64,
            critical_events: result2.metrics.critical_events as i64
                - result1.metrics.critical_events as i64,
            duration_seconds: result2.duration_seconds - result1.duration_seconds,
        }
    }

    pub async fn get_history(&self, limit: Option<usize>) -> Vec<SimulationResult> {
        let history = self.simulation_history.read().await;
        let mut results = history.clone();
        if let Some(lim) = limit {
            results.truncate(lim);
        }
        results
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationComparison {
    pub scenario_id_1: String,
    pub scenario_id_2: String,
    pub state_differences: Vec<EntityStateDifference>,
    pub metric_differences: MetricDifferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityStateDifference {
    pub entity_id: String,
    pub state_change: Option<(TwinState, TwinState)>,
    pub property_changes: Vec<PropertyChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyChange {
    pub property_name: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDifferences {
    pub state_changes: i64,
    pub warnings: i64,
    pub alerts: i64,
    pub critical_events: i64,
    pub duration_seconds: f64,
}

impl Default for DigitalTwinSimulator {
    fn default() -> Self {
        Self::new(Arc::new(TwinModel::default()), SimulationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::digital_twin::{TwinEntity, TwinEntityType, TwinModel, TwinState};
    use chrono::Utc;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn test_simulation_config_default() {
        let config = SimulationConfig::default();
        assert_eq!(config.mode, SimulationMode::RealTime);
        assert_eq!(config.time_multiplier, 1.0);
        assert_eq!(config.max_simulation_seconds, 86400);
        assert!(config.enable_visualization);
    }

    #[test]
    fn test_simulator_new() {
        let model = TwinModel::new();
        let config = SimulationConfig::default();
        let simulator = DigitalTwinSimulator::new(Arc::new(model), config);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let history = rt.block_on(simulator.get_history(None));
        assert!(history.is_empty());
    }

    #[test]
    fn test_create_scenario() {
        let model = TwinModel::new();
        let config = SimulationConfig::default();
        let simulator = DigitalTwinSimulator::new(Arc::new(model), config);

        let scenario =
            simulator.create_scenario("Test Scenario".to_string(), "Test Description".to_string());

        assert_eq!(scenario.name, "Test Scenario");
        assert_eq!(scenario.description, "Test Description");
        assert!(scenario.modifications.is_empty());
    }

    #[test]
    fn test_add_modification() {
        let model = TwinModel::new();
        let config = SimulationConfig::default();
        let simulator = DigitalTwinSimulator::new(Arc::new(model), config);

        let mut scenario =
            simulator.create_scenario("Test Scenario".to_string(), "Test Description".to_string());

        let modification = EntityModification {
            entity_id: "entity-1".to_string(),
            property_changes: HashMap::new(),
            state_change: Some(TwinState::Degraded),
        };

        simulator.add_modification(&mut scenario, modification);
        assert_eq!(scenario.modifications.len(), 1);
    }

    #[tokio::test]
    async fn test_run_simulation() {
        let model = TwinModel::new();
        let config = SimulationConfig::default();
        let simulator = DigitalTwinSimulator::new(Arc::new(model), config);

        let entity = TwinEntity {
            id: "test-entity".to_string(),
            name: "Test Entity".to_string(),
            entity_type: TwinEntityType::Device,
            properties: HashMap::new(),
            state: TwinState::Online,
            parent_id: None,
            children_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        simulator.model.add_entity(entity);

        let mut scenario =
            simulator.create_scenario("Test Scenario".to_string(), "Test Description".to_string());

        let modification = EntityModification {
            entity_id: "test-entity".to_string(),
            property_changes: HashMap::new(),
            state_change: Some(TwinState::Degraded),
        };

        simulator.add_modification(&mut scenario, modification);

        let result = simulator.run_simulation(scenario).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.metrics.state_changes, 1);
    }

    #[tokio::test]
    async fn test_simulation_history() {
        let model = TwinModel::new();
        let config = SimulationConfig::default();
        let simulator = DigitalTwinSimulator::new(Arc::new(model), config);

        let entity = TwinEntity {
            id: "test-entity".to_string(),
            name: "Test Entity".to_string(),
            entity_type: TwinEntityType::Device,
            properties: HashMap::new(),
            state: TwinState::Online,
            parent_id: None,
            children_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        simulator.model.add_entity(entity);

        let scenario1 =
            simulator.create_scenario("Scenario 1".to_string(), "Description 1".to_string());

        let scenario2 =
            simulator.create_scenario("Scenario 2".to_string(), "Description 2".to_string());

        let _ = simulator.run_simulation(scenario1).await;
        let _ = simulator.run_simulation(scenario2).await;

        let history = simulator.get_history(None).await;
        assert_eq!(history.len(), 2);

        let limited_history = simulator.get_history(Some(1)).await;
        assert_eq!(limited_history.len(), 1);
    }
}
