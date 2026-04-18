use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use super::ModelFormat;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub format: ModelFormat,
    pub version: String,
    pub path: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub struct ModelRegistry {
    models: Arc<DashMap<String, Model>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: Arc::new(DashMap::new()),
        }
    }

    pub fn register_model(&self, mut model: Model) -> crate::utils::Result<Model> {
        let now = chrono::Utc::now();
        model.created_at = now;
        model.updated_at = now;

        info!("Registering model: {} ({})", model.id, model.name);
        self.models.insert(model.id.clone(), model.clone());

        Ok(model)
    }

    pub fn get_model(&self, id: &str) -> Option<Model> {
        self.models.get(id).map(|m| m.clone())
    }

    pub fn list_models(&self) -> Vec<Model> {
        self.models
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn update_model(&self, id: &str, mut model: Model) -> crate::utils::Result<Model> {
        if let Some(mut existing) = self.models.get_mut(id) {
            info!("Updating model: {} ({})", id, model.name);
            let now = chrono::Utc::now();
            model.id = id.to_string();
            model.created_at = existing.created_at;
            model.updated_at = now;

            *existing = model.clone();
            Ok(model)
        } else {
            Err(crate::utils::AetherisError::NotFound(format!(
                "Model not found: {}",
                id
            )))
        }
    }

    pub fn delete_model(&self, id: &str) -> bool {
        info!("Deleting model: {}", id);
        self.models.remove(id).is_some()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::inference::ModelFormat;

    #[test]
    fn test_model_registry_new() {
        let registry = ModelRegistry::new();
        assert!(registry.list_models().is_empty());
    }

    #[test]
    fn test_model_registry_default() {
        let registry = ModelRegistry::default();
        assert!(registry.list_models().is_empty());
    }

    #[test]
    fn test_register_model() {
        let registry = ModelRegistry::new();

        let model = Model {
            id: "model-1".to_string(),
            name: "Test Model".to_string(),
            format: ModelFormat::ONNX,
            version: "1.0.0".to_string(),
            path: "/models/test.onnx".to_string(),
            description: Some("Test model description".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let result = registry.register_model(model.clone());
        assert!(result.is_ok());

        let registered = result.unwrap();
        assert_eq!(registered.id, "model-1");
        assert_eq!(registered.name, "Test Model");

        let retrieved = registry.get_model("model-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Model");
    }

    #[test]
    fn test_list_models() {
        let registry = ModelRegistry::new();

        let model1 = Model {
            id: "model-1".to_string(),
            name: "Model 1".to_string(),
            format: ModelFormat::ONNX,
            version: "1.0.0".to_string(),
            path: "/models/model1.onnx".to_string(),
            description: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let model2 = Model {
            id: "model-2".to_string(),
            name: "Model 2".to_string(),
            format: ModelFormat::TorchScript,
            version: "2.0.0".to_string(),
            path: "/models/model2.pt".to_string(),
            description: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        registry.register_model(model1).unwrap();
        registry.register_model(model2).unwrap();

        let models = registry.list_models();
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn test_update_model() {
        let registry = ModelRegistry::new();

        let original = Model {
            id: "model-1".to_string(),
            name: "Original Name".to_string(),
            format: ModelFormat::ONNX,
            version: "1.0.0".to_string(),
            path: "/models/original.onnx".to_string(),
            description: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        registry.register_model(original).unwrap();

        let updated = Model {
            id: "model-1".to_string(),
            name: "Updated Name".to_string(),
            format: ModelFormat::TFLite,
            version: "2.0.0".to_string(),
            path: "/models/updated.tflite".to_string(),
            description: Some("Updated description".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let result = registry.update_model("model-1", updated);
        assert!(result.is_ok());

        let retrieved = registry.get_model("model-1").unwrap();
        assert_eq!(retrieved.name, "Updated Name");
        assert_eq!(retrieved.version, "2.0.0");
    }

    #[test]
    fn test_update_nonexistent_model() {
        let registry = ModelRegistry::new();

        let model = Model {
            id: "nonexistent".to_string(),
            name: "Test".to_string(),
            format: ModelFormat::ONNX,
            version: "1.0.0".to_string(),
            path: "/models/test.onnx".to_string(),
            description: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let result = registry.update_model("nonexistent", model);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_model() {
        let registry = ModelRegistry::new();

        let model = Model {
            id: "model-1".to_string(),
            name: "Test".to_string(),
            format: ModelFormat::ONNX,
            version: "1.0.0".to_string(),
            path: "/models/test.onnx".to_string(),
            description: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        registry.register_model(model).unwrap();
        assert!(registry.get_model("model-1").is_some());

        let result = registry.delete_model("model-1");
        assert!(result);
        assert!(registry.get_model("model-1").is_none());
    }

    #[test]
    fn test_delete_nonexistent_model() {
        let registry = ModelRegistry::new();
        let result = registry.delete_model("nonexistent");
        assert!(!result);
    }

    #[test]
    fn test_model_format_equality() {
        assert_eq!(ModelFormat::ONNX, ModelFormat::ONNX);
        assert_eq!(ModelFormat::TorchScript, ModelFormat::TorchScript);
        assert_eq!(ModelFormat::TFLite, ModelFormat::TFLite);
        assert_eq!(ModelFormat::Custom, ModelFormat::Custom);
    }
}
