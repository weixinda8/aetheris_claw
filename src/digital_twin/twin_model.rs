use crate::digital_twin::{
    CommandStatus, TwinCommand, TwinEntity, TwinEntityType, TwinStateUpdate,
};
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;

pub struct TwinModel {
    entities: Arc<DashMap<String, TwinEntity>>,
    command_history: Arc<DashMap<String, TwinCommand>>,
    state_history: Arc<DashMap<String, Vec<TwinStateUpdate>>>,
}

impl TwinModel {
    pub fn new() -> Self {
        Self {
            entities: Arc::new(DashMap::new()),
            command_history: Arc::new(DashMap::new()),
            state_history: Arc::new(DashMap::new()),
        }
    }

    pub fn add_entity(&self, mut entity: TwinEntity) {
        entity.created_at = Utc::now();
        entity.updated_at = Utc::now();
        self.entities.insert(entity.id.clone(), entity);
    }

    pub fn get_entity(&self, entity_id: &str) -> Option<TwinEntity> {
        self.entities.get(entity_id).map(|e| e.value().clone())
    }

    pub fn list_entities(&self) -> Vec<TwinEntity> {
        self.entities.iter().map(|e| e.value().clone()).collect()
    }

    pub fn list_entities_by_type(&self, entity_type: TwinEntityType) -> Vec<TwinEntity> {
        self.entities
            .iter()
            .map(|e| e.value().clone())
            .filter(|e| e.entity_type == entity_type)
            .collect()
    }

    pub fn get_children(&self, parent_id: &str) -> Vec<TwinEntity> {
        self.entities
            .iter()
            .map(|e| e.value().clone())
            .filter(|e| e.parent_id.as_ref() == Some(&parent_id.to_string()))
            .collect()
    }

    pub fn update_entity(
        &self,
        entity_id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> bool {
        if let Some(mut entity) = self.entities.get_mut(entity_id) {
            entity.properties.extend(properties);
            entity.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    pub fn update_state(&self, mut update: TwinStateUpdate) -> bool {
        if let Some(mut entity) = self.entities.get_mut(&update.entity_id) {
            entity.state = update.state.clone();
            if let Some(props) = update.properties.take() {
                entity.properties.extend(props);
            }
            entity.updated_at = update.timestamp;

            self.state_history
                .entry(update.entity_id.clone())
                .or_default()
                .push(update);

            true
        } else {
            false
        }
    }

    pub fn get_state_history(&self, entity_id: &str, limit: Option<usize>) -> Vec<TwinStateUpdate> {
        self.state_history
            .get(entity_id)
            .map(|history| {
                let mut history = history.value().clone();
                if let Some(lim) = limit {
                    history.truncate(lim);
                }
                history
            })
            .unwrap_or_default()
    }

    pub fn create_command(&self, mut command: TwinCommand) -> String {
        command.id = uuid::Uuid::new_v4().to_string();
        command.issued_at = Utc::now();
        command.status = CommandStatus::Pending;
        let id = command.id.clone();
        self.command_history.insert(id.clone(), command);
        id
    }

    pub fn update_command_status(&self, command_id: &str, status: CommandStatus) -> bool {
        if let Some(mut command) = self.command_history.get_mut(command_id) {
            command.status = status;
            true
        } else {
            false
        }
    }

    pub fn get_command(&self, command_id: &str) -> Option<TwinCommand> {
        self.command_history
            .get(command_id)
            .map(|c| c.value().clone())
    }

    pub fn list_commands(
        &self,
        entity_id: Option<&str>,
        status: Option<CommandStatus>,
    ) -> Vec<TwinCommand> {
        self.command_history
            .iter()
            .map(|c| c.value().clone())
            .filter(|c| {
                let entity_match = entity_id.is_none_or(|id| c.target_entity_id == id);
                let status_match = status.as_ref().is_none_or(|s| c.status == *s);
                entity_match && status_match
            })
            .collect()
    }

    pub fn remove_entity(&self, entity_id: &str) -> bool {
        self.entities.remove(entity_id).is_some()
    }
}

impl Default for TwinModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::digital_twin::{
        CommandStatus, TwinCommand, TwinEntity, TwinEntityType, TwinState, TwinStateUpdate,
    };
    use chrono::Utc;
    use std::collections::HashMap;

    #[test]
    fn test_twin_model_new() {
        let model = TwinModel::new();
        assert!(model.list_entities().is_empty());
    }

    #[test]
    fn test_twin_model_default() {
        let model = TwinModel::default();
        assert!(model.list_entities().is_empty());
    }

    #[test]
    fn test_add_and_get_entity() {
        let model = TwinModel::new();
        let entity = TwinEntity {
            id: "test-entity-1".to_string(),
            name: "Test Entity".to_string(),
            entity_type: TwinEntityType::Device,
            properties: HashMap::new(),
            state: TwinState::Online,
            parent_id: None,
            children_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        model.add_entity(entity.clone());

        let retrieved = model.get_entity("test-entity-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Entity");
    }

    #[test]
    fn test_list_entities() {
        let model = TwinModel::new();

        let entity1 = TwinEntity {
            id: "entity-1".to_string(),
            name: "Entity 1".to_string(),
            entity_type: TwinEntityType::Sensor,
            properties: HashMap::new(),
            state: TwinState::Online,
            parent_id: None,
            children_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let entity2 = TwinEntity {
            id: "entity-2".to_string(),
            name: "Entity 2".to_string(),
            entity_type: TwinEntityType::Actuator,
            properties: HashMap::new(),
            state: TwinState::Offline,
            parent_id: None,
            children_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        model.add_entity(entity1);
        model.add_entity(entity2);

        let entities = model.list_entities();
        assert_eq!(entities.len(), 2);
    }

    #[test]
    fn test_list_entities_by_type() {
        let model = TwinModel::new();

        let sensor_entity = TwinEntity {
            id: "sensor-1".to_string(),
            name: "Temperature Sensor".to_string(),
            entity_type: TwinEntityType::Sensor,
            properties: HashMap::new(),
            state: TwinState::Online,
            parent_id: None,
            children_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let device_entity = TwinEntity {
            id: "device-1".to_string(),
            name: "Motor Device".to_string(),
            entity_type: TwinEntityType::Device,
            properties: HashMap::new(),
            state: TwinState::Online,
            parent_id: None,
            children_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        model.add_entity(sensor_entity);
        model.add_entity(device_entity);

        let sensors = model.list_entities_by_type(TwinEntityType::Sensor);
        assert_eq!(sensors.len(), 1);
        assert_eq!(sensors[0].name, "Temperature Sensor");
    }

    #[test]
    fn test_update_entity() {
        let model = TwinModel::new();
        let mut properties = HashMap::new();
        properties.insert("key1".to_string(), serde_json::json!("value1"));

        let entity = TwinEntity {
            id: "update-test".to_string(),
            name: "Update Test".to_string(),
            entity_type: TwinEntityType::Device,
            properties,
            state: TwinState::Online,
            parent_id: None,
            children_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        model.add_entity(entity);

        let mut new_properties = HashMap::new();
        new_properties.insert("key2".to_string(), serde_json::json!("value2"));

        let result = model.update_entity("update-test", new_properties);
        assert!(result);

        let updated = model.get_entity("update-test").unwrap();
        assert!(updated.properties.contains_key("key1"));
        assert!(updated.properties.contains_key("key2"));
    }

    #[test]
    fn test_update_state() {
        let model = TwinModel::new();

        let entity = TwinEntity {
            id: "state-test".to_string(),
            name: "State Test".to_string(),
            entity_type: TwinEntityType::Sensor,
            properties: HashMap::new(),
            state: TwinState::Online,
            parent_id: None,
            children_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        model.add_entity(entity);

        let update = TwinStateUpdate {
            entity_id: "state-test".to_string(),
            state: TwinState::Degraded,
            properties: None,
            timestamp: Utc::now(),
            source: "test".to_string(),
        };

        let result = model.update_state(update);
        assert!(result);

        let updated = model.get_entity("state-test").unwrap();
        assert_eq!(updated.state, TwinState::Degraded);
    }

    #[test]
    fn test_get_state_history() {
        let model = TwinModel::new();

        let entity = TwinEntity {
            id: "history-test".to_string(),
            name: "History Test".to_string(),
            entity_type: TwinEntityType::Sensor,
            properties: HashMap::new(),
            state: TwinState::Online,
            parent_id: None,
            children_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        model.add_entity(entity);

        let update1 = TwinStateUpdate {
            entity_id: "history-test".to_string(),
            state: TwinState::Degraded,
            properties: None,
            timestamp: Utc::now(),
            source: "test1".to_string(),
        };

        let update2 = TwinStateUpdate {
            entity_id: "history-test".to_string(),
            state: TwinState::Failed,
            properties: None,
            timestamp: Utc::now(),
            source: "test2".to_string(),
        };

        model.update_state(update1);
        model.update_state(update2);

        let history = model.get_state_history("history-test", None);
        assert_eq!(history.len(), 2);

        let limited_history = model.get_state_history("history-test", Some(1));
        assert_eq!(limited_history.len(), 1);
    }

    #[test]
    fn test_create_and_update_command() {
        let model = TwinModel::new();

        let command = TwinCommand {
            id: String::new(),
            target_entity_id: "entity-1".to_string(),
            command_type: "start".to_string(),
            parameters: HashMap::new(),
            issued_at: Utc::now(),
            timeout_seconds: None,
            status: CommandStatus::Pending,
        };

        let command_id = model.create_command(command);
        assert!(!command_id.is_empty());

        let result = model.update_command_status(&command_id, CommandStatus::Executing);
        assert!(result);

        let retrieved = model.get_command(&command_id).unwrap();
        assert_eq!(retrieved.status, CommandStatus::Executing);
    }

    #[test]
    fn test_list_commands() {
        let model = TwinModel::new();

        let command1 = TwinCommand {
            id: String::new(),
            target_entity_id: "entity-1".to_string(),
            command_type: "start".to_string(),
            parameters: HashMap::new(),
            issued_at: Utc::now(),
            timeout_seconds: None,
            status: CommandStatus::Pending,
        };

        let command2 = TwinCommand {
            id: String::new(),
            target_entity_id: "entity-2".to_string(),
            command_type: "stop".to_string(),
            parameters: HashMap::new(),
            issued_at: Utc::now(),
            timeout_seconds: None,
            status: CommandStatus::Completed,
        };

        model.create_command(command1);
        model.create_command(command2);

        let all_commands = model.list_commands(None, None);
        assert_eq!(all_commands.len(), 2);

        let entity1_commands = model.list_commands(Some("entity-1"), None);
        assert_eq!(entity1_commands.len(), 1);

        let completed_commands = model.list_commands(None, Some(CommandStatus::Completed));
        assert_eq!(completed_commands.len(), 1);
    }

    #[test]
    fn test_remove_entity() {
        let model = TwinModel::new();

        let entity = TwinEntity {
            id: "remove-test".to_string(),
            name: "Remove Test".to_string(),
            entity_type: TwinEntityType::Device,
            properties: HashMap::new(),
            state: TwinState::Online,
            parent_id: None,
            children_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        model.add_entity(entity);
        assert!(model.get_entity("remove-test").is_some());

        let result = model.remove_entity("remove-test");
        assert!(result);
        assert!(model.get_entity("remove-test").is_none());
    }

    #[test]
    fn test_get_nonexistent_entity() {
        let model = TwinModel::new();
        let entity = model.get_entity("nonexistent");
        assert!(entity.is_none());
    }

    #[test]
    fn test_update_nonexistent_entity() {
        let model = TwinModel::new();
        let result = model.update_entity("nonexistent", HashMap::new());
        assert!(!result);
    }

    #[test]
    fn test_update_nonexistent_command() {
        let model = TwinModel::new();
        let result = model.update_command_status("nonexistent", CommandStatus::Completed);
        assert!(!result);
    }
}
