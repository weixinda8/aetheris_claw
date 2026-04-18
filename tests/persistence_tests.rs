use aetheris::config::template_library::{ConfigTemplateLibrary, TemplateType};
use aetheris::config::version_control::ConfigVersionControl;
use aetheris::core::plugin::{EnhancedPluginRegistry, PluginMetadata, PluginType};
use aetheris::core::progressive_loading::ProgressiveLoader;
use aetheris::core::smart_preload::{
    ActivityType, PreloadConfig, PreloadableType, SmartPreloader, UserActivity,
};
use serde_json::json;
use std::collections::HashMap;
use tempfile::tempdir;

// #[test]
// fn test_config_version_control_persistence() {
//     let temp_dir = tempdir().unwrap();
//     let storage_path = temp_dir.path();

//     {
//         let vcs = ConfigVersionControl::new(storage_path.to_path_buf()).unwrap();
//         let config_data = json!({"key": "value"});

//         let version = vcs.create_version(
//             ConfigType::Aetheris,
//             "test-config".to_string(),
//             config_data.clone(),
//             "test-user".to_string(),
//             "Initial version".to_string(),
//             None,
//         ).unwrap();

//         assert_eq!(version.version_number, 1);
//         assert_eq!(vcs.version_count(), 1);

//         vcs.save().unwrap();
//     }

//     {
//         let vcs = ConfigVersionControl::new(storage_path.to_path_buf()).unwrap();
//         assert_eq!(vcs.version_count(), 1);

//         let latest = vcs.get_latest_version(&ConfigType::Aetheris, "test-config");
//         assert!(latest.is_some());
//         assert_eq!(latest.unwrap().version_number, 1);
//     }
// }

// #[test]
// fn test_config_version_control_multiple_versions() {
//     let temp_dir = tempdir().unwrap();
//     let storage_path = temp_dir.path();

//     let vcs = ConfigVersionControl::new(storage_path.to_path_buf()).unwrap();

//     let config_data1 = json!({"key": "value1"});
//     let config_data2 = json!({"key": "value2"});

//     vcs.create_version(
//         ConfigType::Aetheris,
//         "test-config".to_string(),
//         config_data1,
//         "test-user".to_string(),
//         "Version 1".to_string(),
//         None,
//     ).unwrap();

//     vcs.create_version(
//         ConfigType::Aetheris,
//         "test-config".to_string(),
//         config_data2,
//         "test-user".to_string(),
//         "Version 2".to_string(),
//         None,
//     ).unwrap();

//     assert_eq!(vcs.version_count(), 2);

//     let latest = vcs.get_latest_version(&ConfigType::Aetheris, "test-config");
//     assert!(latest.is_some());
//     assert_eq!(latest.unwrap().version_number, 2);
// }

// #[test]
// fn test_config_version_control_branches() {
//     let temp_dir = tempdir().unwrap();
//     let storage_path = temp_dir.path();

//     let vcs = ConfigVersionControl::new(storage_path.to_path_buf()).unwrap();

//     let branch = vcs.create_branch(
//         "feature".to_string(),
//         "Feature branch".to_string(),
//         "test-user".to_string(),
//         None,
//     ).unwrap();

//     assert_eq!(branch.name, "feature");
//     assert_eq!(vcs.branch_count(), 1);
// }

#[test]
fn test_config_template_library_persistence() {
    let temp_dir = tempdir().unwrap();
    let storage_path = temp_dir.path();

    {
        let library = ConfigTemplateLibrary::new(storage_path.to_path_buf()).unwrap();

        let template = aetheris::config::template_library::ConfigTemplate {
            template_id: "test-template".to_string(),
            name: "Test Template".to_string(),
            description: "A test template".to_string(),
            template_type: TemplateType::AetherisConfig,
            version: "1.0.0".to_string(),
            author: "test-author".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec!["test".to_string()],
            categories: vec!["example".to_string()],
            content: json!({"key": "{{value}}"}),
            variables: vec![],
            examples: vec![],
            is_official: false,
            is_published: true,
            download_count: 0,
            rating: 0.0,
            rating_count: 0,
        };

        library.register_template(template).unwrap();
        assert_eq!(library.published_template_count(), 1);
        library.save().unwrap();
    }

    {
        let library = ConfigTemplateLibrary::new(storage_path.to_path_buf()).unwrap();
        assert_eq!(library.published_template_count(), 1);

        let template = library.get_template("test-template");
        assert!(template.is_some());
        assert_eq!(template.unwrap().name, "Test Template");
    }
}

#[test]
fn test_config_template_library_search() {
    let temp_dir = tempdir().unwrap();
    let storage_path = temp_dir.path();

    let library = ConfigTemplateLibrary::new(storage_path.to_path_buf()).unwrap();

    let template = aetheris::config::template_library::ConfigTemplate {
        template_id: "test-template".to_string(),
        name: "Test Template".to_string(),
        description: "A test template".to_string(),
        template_type: TemplateType::AetherisConfig,
        version: "1.0.0".to_string(),
        author: "test-author".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        tags: vec!["test".to_string()],
        categories: vec!["example".to_string()],
        content: json!({}),
        variables: vec![],
        examples: vec![],
        is_official: false,
        is_published: true,
        download_count: 0,
        rating: 0.0,
        rating_count: 0,
    };

    library.register_template(template).unwrap();

    let results = library.search_templates("test");
    assert!(!results.is_empty());

    let by_type = library.get_templates_by_type(&TemplateType::AetherisConfig);
    assert!(!by_type.is_empty());

    let by_tag = library.get_templates_by_tag("test");
    assert!(!by_tag.is_empty());
}

#[test]
fn test_enhanced_plugin_registry_persistence() {
    let temp_dir = tempdir().unwrap();
    let storage_path = temp_dir.path();

    {
        let registry = EnhancedPluginRegistry::new(storage_path.to_path_buf()).unwrap();

        let metadata = PluginMetadata::new(
            "test-plugin".to_string(),
            "Test Plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
            PluginType::Skill,
        );

        registry.register_plugin(metadata, None).unwrap();
        assert_eq!(registry.plugin_count(), 1);
        registry.save().unwrap();
    }

    {
        let registry = EnhancedPluginRegistry::new(storage_path.to_path_buf()).unwrap();
        assert_eq!(registry.plugin_count(), 1);

        let plugin = registry.get_plugin("test-plugin");
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().metadata.name, "Test Plugin");
    }
}

#[test]
fn test_enhanced_plugin_registry_list() {
    let temp_dir = tempdir().unwrap();
    let storage_path = temp_dir.path();

    let registry = EnhancedPluginRegistry::new(storage_path.to_path_buf()).unwrap();

    let metadata = PluginMetadata::new(
        "test-plugin".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "A test plugin".to_string(),
        PluginType::Skill,
    );

    registry.register_plugin(metadata, None).unwrap();

    let plugins = registry.list_plugins(None, false);
    assert_eq!(plugins.len(), 1);

    let by_type = registry.get_plugins_by_type(&PluginType::Skill);
    assert_eq!(by_type.len(), 1);
}

#[tokio::test]
async fn test_progressive_loader_persistence() {
    let temp_dir = tempdir().unwrap();
    let storage_path = temp_dir.path();

    {
        let loader =
            ProgressiveLoader::new(100000, 10.0, 0.01, storage_path.to_path_buf()).unwrap();
        let summary = loader.get_loading_summary("nonexistent").await;
        assert!(summary.is_none());
    }

    {
        let loader =
            ProgressiveLoader::new(100000, 10.0, 0.01, storage_path.to_path_buf()).unwrap();
        let within_budget = loader.is_within_budget("nonexistent").await;
        assert!(within_budget);
    }
}

#[test]
fn test_smart_preloader_persistence() {
    let temp_dir = tempdir().unwrap();
    let storage_path = temp_dir.path();

    {
        let preloader =
            SmartPreloader::new(PreloadConfig::default(), storage_path.to_path_buf()).unwrap();
        assert_eq!(preloader.get_preload_stats().total_preloaded, 0);
    }

    {
        let preloader =
            SmartPreloader::new(PreloadConfig::default(), storage_path.to_path_buf()).unwrap();
        assert_eq!(preloader.get_preload_stats().total_preloaded, 0);
    }
}

#[test]
fn test_smart_preloader_activity() {
    let temp_dir = tempdir().unwrap();
    let storage_path = temp_dir.path();

    let preloader =
        SmartPreloader::new(PreloadConfig::default(), storage_path.to_path_buf()).unwrap();

    let activity = UserActivity {
        user_id: "test-user".to_string(),
        item_id: "test-skill".to_string(),
        item_type: PreloadableType::Skill,
        activity_type: ActivityType::Use,
        timestamp: chrono::Utc::now(),
        session_id: "session-1".to_string(),
        context: HashMap::new(),
    };

    preloader.record_activity(activity).unwrap();

    let predictions = preloader.generate_predictions("test-user");
    assert!(!predictions.is_empty());
}

#[test]
fn test_persistence_with_empty_state() {
    let temp_dir = tempdir().unwrap();
    let storage_path = temp_dir.path();

    let vcs = ConfigVersionControl::new(storage_path.to_path_buf()).unwrap();
    assert_eq!(vcs.version_count(), 0);
    assert_eq!(vcs.branch_count(), 0);
    assert_eq!(vcs.merge_request_count(), 0);

    let library = ConfigTemplateLibrary::new(storage_path.to_path_buf()).unwrap();
    assert_eq!(library.template_count(), 0);

    let registry = EnhancedPluginRegistry::new(storage_path.to_path_buf()).unwrap();
    assert_eq!(registry.plugin_count(), 0);
}

// #[test]
// fn test_config_version_control_tags() {
//     let temp_dir = tempdir().unwrap();
//     let storage_path = temp_dir.path();

//     let vcs = ConfigVersionControl::new(storage_path.to_path_buf()).unwrap();

//     let config_data = json!({"key": "value"});

//     let version = vcs.create_version(
//         ConfigType::Aetheris,
//         "test-config".to_string(),
//         config_data,
//         "test-user".to_string(),
//         "Initial version".to_string(),
//         None,
//     ).unwrap();

//     vcs.add_tag(&version.version_id, "important".to_string()).unwrap();

//     let versions_by_tag = vcs.get_versions_by_tag("important");
//     assert_eq!(versions_by_tag.len(), 1);
//     assert_eq!(versions_by_tag[0].version_id, version.version_id);
// }
