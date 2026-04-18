#[cfg(test)]
mod e2e_tests {
    use aetheris::agent::config::config::AgentConfig;
    use aetheris::agent::config::storage::StorageManager;
    use aetheris::agent::config::template::AgentTemplateEngine;
    use tempfile::tempdir;

    #[test]
    fn test_template_engine() {
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

        println!("✅ Template engine test passed!");
    }

    #[tokio::test]
    async fn test_in_memory_storage() {
        let manager = StorageManager::with_in_memory();

        let mut config = AgentConfig::default();
        config.meta.id = "test_agent_e2e".to_string();

        manager
            .save_config("test_agent_e2e", &config)
            .await
            .unwrap();

        let exists = manager.config_exists("test_agent_e2e").await.unwrap();
        assert!(exists);

        let loaded = manager.get_config("test_agent_e2e").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().meta.id, "test_agent_e2e");

        let list = manager.list_agents().await.unwrap();
        assert_eq!(list.len(), 1);

        let deleted = manager.delete_config("test_agent_e2e").await.unwrap();
        assert!(deleted);

        let exists_after = manager.config_exists("test_agent_e2e").await.unwrap();
        assert!(!exists_after);

        println!("✅ InMemoryStorage E2E test passed");
    }

    #[tokio::test]
    async fn test_local_storage() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::with_local(temp_dir.path().to_path_buf()).unwrap();

        let mut config = AgentConfig::default();
        config.meta.id = "test_local_agent".to_string();

        manager
            .save_config("test_local_agent", &config)
            .await
            .unwrap();

        let exists = manager.config_exists("test_local_agent").await.unwrap();
        assert!(exists);

        let loaded = manager.get_config("test_local_agent").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().meta.id, "test_local_agent");

        println!("✅ LocalStorage E2E test passed");
    }

    #[tokio::test]
    async fn test_storage_multiple_agents() {
        let manager = StorageManager::with_in_memory();

        let mut config1 = AgentConfig::default();
        config1.meta.id = "agent_1".to_string();
        config1.meta.name = "Agent 1".to_string();

        let mut config2 = AgentConfig::default();
        config2.meta.id = "agent_2".to_string();
        config2.meta.name = "Agent 2".to_string();

        let mut config3 = AgentConfig::default();
        config3.meta.id = "agent_3".to_string();
        config3.meta.name = "Agent 3".to_string();

        manager.save_config("agent_1", &config1).await.unwrap();
        manager.save_config("agent_2", &config2).await.unwrap();
        manager.save_config("agent_3", &config3).await.unwrap();

        let list = manager.list_agents().await.unwrap();
        assert_eq!(list.len(), 3);

        let all_configs = manager.list_all_configs().await.unwrap();
        assert_eq!(all_configs.len(), 3);

        manager.delete_config("agent_2").await.unwrap();

        let list_after_delete = manager.list_agents().await.unwrap();
        assert_eq!(list_after_delete.len(), 2);

        println!("✅ Storage multiple agents E2E test passed");
    }

    #[test]
    fn test_config_validation() {
        let mut config = AgentConfig::default();

        let validation = config.validate();
        assert!(validation.is_ok(), "Valid config should pass");
        println!("✅ Valid config passed validation");

        config.meta.id = "".to_string();
        let validation = config.validate();
        assert!(validation.is_err(), "Empty ID should fail validation");
        println!("✅ Empty ID failed validation as expected");
    }

    #[test]
    fn test_config_validation_boundary_conditions() {
        let mut config = AgentConfig::default();

        config.meta.version = "1.0.0".to_string();
        assert!(config.validate().is_ok(), "Valid semver should pass");

        config.meta.version = "invalid-version".to_string();
        assert!(config.validate().is_err(), "Invalid semver should fail");
        println!("✅ Semver validation test passed");

        let mut valid_config = AgentConfig::default();
        valid_config.meta.id = "valid_id".to_string();
        valid_config.meta.name = "Valid Name".to_string();
        valid_config.meta.version = "2.3.4".to_string();
        valid_config.model.primary = "gpt-4".to_string();
        assert!(valid_config.validate().is_ok());
        println!("✅ Full valid config test passed");
    }

    #[tokio::test]
    async fn test_storage_update_operation() {
        let manager = StorageManager::with_in_memory();

        let mut config = AgentConfig::default();
        config.meta.id = "update_test".to_string();
        config.meta.name = "Initial Name".to_string();

        manager.save_config("update_test", &config).await.unwrap();

        let loaded_initial = manager.get_config("update_test").await.unwrap().unwrap();
        assert_eq!(loaded_initial.meta.name, "Initial Name");

        let mut config_updated = config.clone();
        config_updated.meta.name = "Updated Name".to_string();
        manager
            .save_config("update_test", &config_updated)
            .await
            .unwrap();

        let loaded_updated = manager.get_config("update_test").await.unwrap().unwrap();
        assert_eq!(loaded_updated.meta.name, "Updated Name");

        println!("✅ Storage update operation E2E test passed");
    }

    #[tokio::test]
    async fn test_storage_delete_nonexistent() {
        let manager = StorageManager::with_in_memory();

        let deleted = manager.delete_config("nonexistent_agent").await.unwrap();
        assert!(!deleted, "Deleting nonexistent should return false");

        println!("✅ Storage delete nonexistent E2E test passed");
    }

    #[tokio::test]
    async fn test_storage_get_nonexistent() {
        let manager = StorageManager::with_in_memory();

        let loaded = manager.get_config("nonexistent_agent").await.unwrap();
        assert!(loaded.is_none(), "Getting nonexistent should return None");

        println!("✅ Storage get nonexistent E2E test passed");
    }

    #[tokio::test]
    async fn test_complete_workflow() {
        println!("🚀 Starting complete E2E workflow test...");

        println!("1️⃣ Verifying template engine...");
        let engine = AgentTemplateEngine::new();
        assert!(engine.list_templates().len() >= 4);
        println!("   ✅ Template engine verified");

        println!("2️⃣ Creating test config...");
        let mut config = AgentConfig::default();
        config.meta.id = "e2e_test_agent".to_string();
        config.meta.name = "E2E Test Agent".to_string();
        let validation = config.validate();
        assert!(validation.is_ok());
        println!("   ✅ Config created and validated");

        println!("3️⃣ Saving to storage...");
        let manager = StorageManager::with_in_memory();
        manager.save_config(&config.meta.id, &config).await.unwrap();
        println!("   ✅ Config saved successfully");

        println!("4️⃣ Retrieving from storage...");
        let loaded = manager.get_config(&config.meta.id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().meta.id, "e2e_test_agent");
        println!("   ✅ Config retrieved successfully");

        println!("5️⃣ Verifying storage listing...");
        let list = manager.list_agents().await.unwrap();
        assert_eq!(list.len(), 1);
        println!("   ✅ Storage listing verified");

        println!("6️⃣ Updating config...");
        let mut updated_config = config.clone();
        updated_config.meta.name = "E2E Test Agent Updated".to_string();
        manager
            .save_config(&config.meta.id, &updated_config)
            .await
            .unwrap();
        let reloaded = manager.get_config(&config.meta.id).await.unwrap().unwrap();
        assert_eq!(reloaded.meta.name, "E2E Test Agent Updated");
        println!("   ✅ Config update verified");

        println!("7️⃣ Deleting config...");
        manager.delete_config(&config.meta.id).await.unwrap();
        let exists = manager.config_exists(&config.meta.id).await.unwrap();
        assert!(!exists);
        println!("   ✅ Config deletion verified");

        println!("\n🎉 Complete E2E workflow test passed!");
    }

    #[tokio::test]
    async fn test_multi_agent_workflow() {
        println!("🚀 Starting multi-agent E2E workflow test...");

        let manager = StorageManager::with_in_memory();
        let agent_ids = vec!["agent_alpha", "agent_beta", "agent_gamma"];

        for (i, &id) in agent_ids.iter().enumerate() {
            let mut config = AgentConfig::default();
            config.meta.id = id.to_string();
            config.meta.name = format!("Agent {}", i + 1);
            manager.save_config(id, &config).await.unwrap();
            println!("   ✅ Saved agent: {}", id);
        }

        let list = manager.list_agents().await.unwrap();
        assert_eq!(list.len(), 3);
        println!("   ✅ All agents listed");

        let all_configs = manager.list_all_configs().await.unwrap();
        assert_eq!(all_configs.len(), 3);
        println!("   ✅ All configs retrieved");

        for id in &agent_ids {
            assert!(manager.config_exists(id).await.unwrap());
        }
        println!("   ✅ All agents verified to exist");

        manager.delete_config("agent_beta").await.unwrap();
        let list_after_delete = manager.list_agents().await.unwrap();
        assert_eq!(list_after_delete.len(), 2);
        println!("   ✅ Agent deletion verified");

        println!("\n🎉 Multi-agent E2E workflow test passed!");
    }
}
