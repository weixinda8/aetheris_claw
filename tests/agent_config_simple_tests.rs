#[cfg(test)]
mod simple_tests {
    use aetheris::agent::config::config::AgentConfig;
    use aetheris::agent::config::storage::{ConfigStorage, InMemoryStorage, StorageManager};
    use aetheris::agent::config::template::AgentTemplateEngine;
    use aetheris::agent::config::version_control::VersionManager;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_basic_template_engine() {
        let engine = AgentTemplateEngine::new();

        let templates = engine.list_templates();
        assert!(
            templates.len() >= 4,
            "Should have at least 4 official templates"
        );

        let template_ids: Vec<_> = templates.iter().map(|t| t.id.as_str()).collect();
        assert!(template_ids.contains(&"code_agent"));
        assert!(template_ids.contains(&"office_agent"));
        assert!(template_ids.contains(&"data_agent"));
        assert!(template_ids.contains(&"ops_agent"));

        println!("✅ 基础模板系统测试通过！");
    }

    #[tokio::test]
    async fn test_in_memory_storage_basic() {
        let storage = InMemoryStorage::new();

        let list = storage.list().await.unwrap();
        assert!(list.is_empty(), "Storage should be empty initially");

        println!("✅ InMemoryStorage 基础测试通过！");
    }

    #[test]
    fn test_example_files_exist() {
        let config_path_yaml = PathBuf::from("examples/agents/code_agent.yaml");
        let config_path_json5 = PathBuf::from("examples/agents/code_agent.json5");

        assert!(config_path_yaml.exists(), "YAML config file not found");
        assert!(config_path_json5.exists(), "JSON5 config file not found");

        println!("✅ 示例配置文件存在！");
    }

    #[test]
    fn test_all_example_configs_exist() {
        let agents = ["code_agent", "office_agent", "data_agent", "ops_agent"];

        for agent in agents {
            let yaml_path = PathBuf::from(format!("examples/agents/{}.yaml", agent));
            assert!(yaml_path.exists(), "{} config file not found", agent);
        }

        println!("✅ 所有示例配置文件都存在！");
    }

    #[tokio::test]
    async fn test_template_and_storage_integration() {
        println!("🚀 测试模板到存储集成...");

        let _engine = AgentTemplateEngine::new();
        let manager = StorageManager::with_in_memory();

        let template_ids = vec!["code_agent", "office_agent", "data_agent", "ops_agent"];

        for template_id in template_ids {
            let mut config = AgentConfig::default();
            config.meta.id = "test-".to_string() + template_id;

            assert!(config.validate().is_ok(), "Config should be valid");

            manager.save_config(&config.meta.id, &config).await.unwrap();

            let loaded = manager.get_config(&config.meta.id).await.unwrap();
            assert!(loaded.is_some(), "Config should be in storage");
            assert_eq!(loaded.unwrap().meta.id, config.meta.id);
        }

        let list = manager.list_agents().await.unwrap();
        assert_eq!(list.len(), 4);

        println!("✅ 模板到存储集成测试通过！");
    }

    #[tokio::test]
    async fn test_version_control_integration() {
        println!("🚀 测试版本控制集成...");

        let version_manager = VersionManager::new();
        let storage = InMemoryStorage::new();

        let mut config = AgentConfig::default();
        config.meta.id = "version_test".to_string();
        config.meta.version = "1.0.0".to_string();
        config.meta.name = "Version 1".to_string();

        let _v1 = version_manager
            .create_version("version_test", config.clone(), None, None)
            .await
            .unwrap();
        storage.put("version_test", &config).await.unwrap();

        let mut config_v2 = config.clone();
        config_v2.meta.version = "1.1.0".to_string();
        config_v2.meta.name = "Version 2".to_string();

        let _v2 = version_manager
            .create_version("version_test", config_v2.clone(), None, None)
            .await
            .unwrap();
        storage.put("version_test", &config_v2).await.unwrap();

        let versions = version_manager.list_versions("version_test").await;
        assert_eq!(versions.len(), 2);

        let rollback = version_manager
            .rollback_to_version("version_test", "1.0.0")
            .await
            .unwrap();
        assert!(rollback.success);

        let rolled_back_config = rollback.config.unwrap();
        storage
            .put("version_test", &rolled_back_config)
            .await
            .unwrap();

        let from_storage = storage.get("version_test").await.unwrap().unwrap();
        assert_eq!(from_storage.meta.version, "1.0.0");
        assert_eq!(from_storage.meta.name, "Version 1");

        println!("✅ 版本控制集成测试通过！");
    }

    #[test]
    fn test_config_validation_integration() {
        println!("🚀 测试配置验证集成...");

        let mut valid_config = AgentConfig::default();
        valid_config.meta.id = "valid_id".to_string();
        valid_config.meta.name = "Valid Agent".to_string();
        valid_config.meta.version = "1.0.0".to_string();
        valid_config.model.primary = "gpt-4".to_string();

        assert!(valid_config.validate().is_ok());
        println!("✅ 有效配置验证通过！");

        let mut invalid_config = AgentConfig::default();
        invalid_config.meta.id = "".to_string();
        assert!(invalid_config.validate().is_err());
        println!("✅ 无效配置验证失败（预期）！");
    }

    #[tokio::test]
    async fn test_complete_config_lifecycle() {
        println!("🚀 测试完整配置生命周期...");

        let storage = StorageManager::with_in_memory();
        let version_manager = VersionManager::new();

        let mut config = AgentConfig::default();
        config.meta.id = "lifecycle-test".to_string();
        config.meta.name = "Lifecycle Test Agent".to_string();

        assert!(config.validate().is_ok());
        storage.save_config(&config.meta.id, &config).await.unwrap();

        let _v1 = version_manager
            .create_version(&config.meta.id, config.clone(), None, None)
            .await
            .unwrap();

        let mut config_v2 = config.clone();
        config_v2.meta.version = "2.0.0".to_string();
        config_v2.meta.description = Some("Updated version".to_string());

        assert!(config_v2.validate().is_ok());
        storage
            .save_config(&config_v2.meta.id, &config_v2)
            .await
            .unwrap();

        let _v2 = version_manager
            .create_version(&config.meta.id, config_v2.clone(), None, None)
            .await
            .unwrap();

        let rollback = version_manager
            .rollback_to_version(&config.meta.id, "1.0.0")
            .await
            .unwrap();
        assert!(rollback.success);

        let rolled_back_config = rollback.config.unwrap();
        storage
            .save_config(&rolled_back_config.meta.id, &rolled_back_config)
            .await
            .unwrap();

        let final_config = storage
            .get_config(&rolled_back_config.meta.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(final_config.meta.version, "1.0.0");

        let agents = storage.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        storage
            .delete_config(&rolled_back_config.meta.id)
            .await
            .unwrap();

        let exists_after = storage
            .config_exists(&rolled_back_config.meta.id)
            .await
            .unwrap();
        assert!(!exists_after);

        println!("✅ 完整配置生命周期集成测试通过！");
    }

    #[tokio::test]
    async fn test_multi_agent_management() {
        println!("🚀 测试多代理管理...");

        let storage = StorageManager::with_in_memory();

        let config_ids = vec!["my-code-agent", "my-office-agent", "my-data-agent"];

        for agent_id in &config_ids {
            let mut config = AgentConfig::default();
            config.meta.id = agent_id.to_string();
            config.meta.name = format!("Agent: {}", agent_id);
            storage.save_config(&config.meta.id, &config).await.unwrap();
        }

        let list = storage.list_agents().await.unwrap();
        assert_eq!(list.len(), 3);

        let all_configs = storage.list_all_configs().await.unwrap();
        assert_eq!(all_configs.len(), 3);

        storage.delete_config("my-office-agent").await.unwrap();

        let list_after_delete = storage.list_agents().await.unwrap();
        assert_eq!(list_after_delete.len(), 2);

        for id in &["my-code-agent", "my-data-agent"] {
            assert!(storage.config_exists(id).await.unwrap());
        }

        println!("✅ 多代理管理集成测试通过！");
    }
}
