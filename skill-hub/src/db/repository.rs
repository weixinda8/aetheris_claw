use super::entities::Skill as SkillEntity;
use super::models::{
    CallMode, PermissionLevel, SkillMetadata, SkillMetadataExtended, SkillPriority, Version,
};
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

pub struct SkillConverter;

impl SkillConverter {
    pub fn entity_to_metadata(
        entity: &SkillEntity,
    ) -> Result<SkillMetadata, String> {
        let version = Version::from_string(&entity.version)?;
        let call_mode = CallMode::from_str(&entity.call_mode)?;
        let permission_level = PermissionLevel::from_str(&entity.permission_level)?;
        let priority = SkillPriority::from_str(&entity.priority)?;

        let metadata_map: HashMap<String, String> = match &entity.metadata {
            Value::Object(map) => map
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect(),
            _ => HashMap::new(),
        };

        Ok(SkillMetadata {
            id: entity.skill_id.clone(),
            name: entity.name.clone(),
            version,
            description: entity.description.clone(),
            long_description: entity.long_description.clone(),
            tags: entity.tags.clone(),
            categories: entity.categories.clone(),
            author: entity.author_name.clone(),
            created_at: entity.created_at,
            updated_at: entity.updated_at,
            call_mode,
            permission_level,
            priority,
            required_permissions: entity.required_permissions.clone(),
            input_schema: entity.input_schema.clone(),
            output_schema: entity.output_schema.clone(),
            example_input: entity.example_input.clone(),
            example_output: entity.example_output.clone(),
            dependencies: entity.dependencies.clone(),
            is_active: entity.is_active,
            is_deprecated: entity.is_deprecated,
            deprecation_reason: entity.deprecation_reason.clone(),
            metadata: metadata_map,
        })
    }

    pub fn entity_to_extended_metadata(
        entity: &SkillEntity,
    ) -> Result<SkillMetadataExtended, String> {
        let base = Self::entity_to_metadata(entity)?;

        Ok(SkillMetadataExtended {
            base,
            hub_id: entity.id,
            status: entity.status.clone(),
            author_id: entity.author_id,
            author_name: entity.author_name.clone(),
            category: entity.category.clone(),
            download_count: entity.download_count,
            average_rating: entity.average_rating,
            rating_count: entity.rating_count,
            success_rate: entity.success_rate,
            execution_count: entity.execution_count,
            published_at: entity.published_at,
            deprecated_at: entity.deprecated_at,
        })
    }

    pub fn metadata_to_entity(
        metadata: &SkillMetadata,
        hub_id: Uuid,
        author_id: Uuid,
        status: String,
    ) -> SkillEntity {
        let metadata_value = serde_json::to_value(&metadata.metadata).unwrap_or(Value::Object(serde_json::Map::new()));

        SkillEntity {
            id: hub_id,
            skill_id: metadata.id.clone(),
            name: metadata.name.clone(),
            description: metadata.description.clone(),
            long_description: metadata.long_description.clone(),
            version: metadata.version.to_string(),
            author_id,
            author_name: metadata.author.clone(),
            category: None,
            categories: metadata.categories.clone(),
            tags: metadata.tags.clone(),
            status,
            call_mode: metadata.call_mode.as_str().to_string(),
            permission_level: metadata.permission_level.as_str().to_string(),
            priority: metadata.priority.as_str().to_string(),
            required_permissions: metadata.required_permissions.clone(),
            input_schema: metadata.input_schema.clone(),
            output_schema: metadata.output_schema.clone(),
            example_input: metadata.example_input.clone(),
            example_output: metadata.example_output.clone(),
            dependencies: metadata.dependencies.clone(),
            is_active: metadata.is_active,
            is_deprecated: metadata.is_deprecated,
            deprecation_reason: metadata.deprecation_reason.clone(),
            metadata: metadata_value,
            download_count: 0,
            average_rating: 0.0,
            rating_count: 0,
            success_rate: 0.0,
            execution_count: 0,
            created_at: metadata.created_at,
            updated_at: metadata.updated_at,
            published_at: None,
            deprecated_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_version_conversion() {
        let version = Version::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");

        let parsed = Version::from_string("1.2.3").unwrap();
        assert_eq!(parsed.major, 1);
        assert_eq!(parsed.minor, 2);
        assert_eq!(parsed.patch, 3);
    }

    #[test]
    fn test_call_mode_conversion() {
        assert_eq!(CallMode::Text.as_str(), "Text");
        assert_eq!(CallMode::from_str("Text").unwrap(), CallMode::Text);
        assert_eq!(CallMode::Api.as_str(), "Api");
        assert_eq!(CallMode::from_str("Api").unwrap(), CallMode::Api);
    }

    #[test]
    fn test_permission_level_conversion() {
        assert_eq!(PermissionLevel::Public.as_str(), "Public");
        assert_eq!(
            PermissionLevel::from_str("Public").unwrap(),
            PermissionLevel::Public
        );
        assert_eq!(PermissionLevel::Admin.as_str(), "Admin");
        assert_eq!(
            PermissionLevel::from_str("Admin").unwrap(),
            PermissionLevel::Admin
        );
    }

    #[test]
    fn test_skill_priority_conversion() {
        assert_eq!(SkillPriority::Medium.as_str(), "Medium");
        assert_eq!(
            SkillPriority::from_str("Medium").unwrap(),
            SkillPriority::Medium
        );
        assert_eq!(SkillPriority::High.as_str(), "High");
        assert_eq!(
            SkillPriority::from_str("High").unwrap(),
            SkillPriority::High
        );
    }

    #[test]
    fn test_skill_metadata_creation() {
        let version = Version::new(1, 0, 0);
        let metadata = SkillMetadata::new(
            "test-skill".to_string(),
            "Test Skill".to_string(),
            version,
            "A test skill".to_string(),
        );

        assert_eq!(metadata.id, "test-skill");
        assert_eq!(metadata.name, "Test Skill");
        assert_eq!(metadata.is_active, true);
        assert_eq!(metadata.is_deprecated, false);
    }
}
