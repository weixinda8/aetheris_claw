#[cfg(test)]
mod tests {
    use aetheris::agent::{AgentConfig, AgentRegistry, AgentType, BaseAgent};
    use aetheris::api::auth::{AuthManager, LoginRequest, UserRole};
    use aetheris::api::websocket::WebSocketManager;
    use aetheris::core::progressive_loading::{
        ChunkType, ContentChunk, LoadingContext, LoadingStrategy, ProgressiveLoader, TokenUsage,
    };
    use aetheris::core::{Task, TaskStatus};
    use aetheris::memory::ShortTermMemory;
    use aetheris::observability::{AlertSeverity, Telemetry};
    use aetheris::security::SecurityManager;
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_agent_registry() {
        let registry = AgentRegistry::new();

        let config = AgentConfig::new(
            "test-agent-001".to_string(),
            "Test Agent".to_string(),
            AgentType::Generic,
        );
        let agent = BaseAgent::new(config);

        let result = registry.register_agent(Arc::new(agent));
        assert!(result.is_ok());

        let agents = registry.list_all_agents();
        assert_eq!(agents.len(), 1);
    }

    #[tokio::test]
    async fn test_security_manager() {
        let security = SecurityManager::new();

        let task = Task::new("Test task".to_string(), 5);
        let result = security.validate_task(&task).await;

        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(validation.passed);
    }

    #[tokio::test]
    async fn test_short_term_memory() {
        let memory = ShortTermMemory::new();

        let task = Task::new("Test task".to_string(), 5);
        let task_id = task.id.clone();

        memory.store_task(task.clone());

        let retrieved = memory.get_task(&task_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, task_id);

        let all_tasks = memory.get_all_tasks();
        assert_eq!(all_tasks.len(), 1);
    }

    #[tokio::test]
    async fn test_telemetry() {
        let telemetry = Telemetry::new();

        telemetry
            .metrics
            .record_task_start("test-task-001".to_string(), None);
        telemetry
            .metrics
            .record_task_completion("test-task-001", true, Some(1000), Some(0.01));

        let system_metrics = telemetry
            .metrics
            .get_system_metrics(0, telemetry.uptime_seconds());

        assert_eq!(system_metrics.total_tasks, 1);
        assert_eq!(system_metrics.completed_tasks, 1);
        assert_eq!(system_metrics.success_rate, 1.0);
    }

    #[tokio::test]
    async fn test_alert_system() {
        let telemetry = Telemetry::new();

        let alert_id = telemetry.metrics.create_alert(
            "test_alert".to_string(),
            AlertSeverity::Warning,
            "Test alert message".to_string(),
            Some("test-task-001".to_string()),
            None,
        );

        let alerts = telemetry.metrics.get_alerts(false);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_id, alert_id);

        let resolved = telemetry.metrics.resolve_alert(&alert_id);
        assert!(resolved);

        let alerts_after = telemetry.metrics.get_alerts(false);
        assert_eq!(alerts_after.len(), 0);
    }

    #[tokio::test]
    async fn test_task_status_transitions() {
        let mut task = Task::new("Test task".to_string(), 5);

        assert_eq!(task.status, TaskStatus::Pending);

        task.mark_running();
        assert_eq!(task.status, TaskStatus::Running);

        task.mark_paused();
        assert_eq!(task.status, TaskStatus::Paused);

        task.mark_running();
        assert_eq!(task.status, TaskStatus::Running);

        task.mark_completed();
        assert_eq!(task.status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn test_task_priority() {
        let task_low = Task::new("Low priority task".to_string(), 1);
        let task_normal = Task::new("Normal priority task".to_string(), 2);
        let task_high = Task::new("High priority task".to_string(), 3);
        let task_urgent = Task::new("Urgent priority task".to_string(), 4);

        assert_eq!(task_low.priority, 1);
        assert_eq!(task_normal.priority, 2);
        assert_eq!(task_high.priority, 3);
        assert_eq!(task_urgent.priority, 4);
    }

    #[tokio::test]
    async fn test_end_to_end_task_flow() {
        let telemetry = Telemetry::new();
        let memory = ShortTermMemory::new();
        let security = SecurityManager::new();

        let mut task = Task::new("End-to-end test task".to_string(), 5);
        let task_id = task.id.clone();

        let validation = security.validate_task(&task).await;
        assert!(validation.is_ok());
        assert!(validation.unwrap().passed);

        telemetry.metrics.record_task_start(task_id.clone(), None);

        task.mark_running();
        memory.store_task(task.clone());

        task.mark_completed();
        memory.store_task(task.clone());

        telemetry
            .metrics
            .record_task_completion(&task_id, true, Some(5000), Some(0.1));

        let final_task = memory.get_task(&task_id);
        assert!(final_task.is_some());
        assert_eq!(final_task.unwrap().status, TaskStatus::Completed);

        let system_metrics = telemetry
            .metrics
            .get_system_metrics(0, telemetry.uptime_seconds());
        assert_eq!(system_metrics.completed_tasks, 1);
    }

    #[tokio::test]
    async fn test_multi_agent_collaboration() {
        let registry = AgentRegistry::new();

        let config1 = AgentConfig::new(
            "code-agent-001".to_string(),
            "Code Agent".to_string(),
            AgentType::Code,
        );
        let agent1 = BaseAgent::new(config1);
        registry.register_agent(Arc::new(agent1)).unwrap();

        let config2 = AgentConfig::new(
            "data-agent-001".to_string(),
            "Data Agent".to_string(),
            AgentType::Data,
        );
        let agent2 = BaseAgent::new(config2);
        registry.register_agent(Arc::new(agent2)).unwrap();

        let agents = registry.list_all_agents();
        assert_eq!(agents.len(), 2);

        let code_agents = registry.get_agents_by_type(&AgentType::Code);
        assert_eq!(code_agents.len(), 1);

        let data_agents = registry.get_agents_by_type(&AgentType::Data);
        assert_eq!(data_agents.len(), 1);
    }

    #[tokio::test]
    async fn test_progressive_loader_token_usage() {
        let mut usage = TokenUsage::new();

        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
        assert_eq!(usage.estimated_cost_usd, 0.0);

        let usage2 = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            estimated_cost_usd: 0.015,
        };

        usage.add(&usage2);

        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
        assert_eq!(usage.estimated_cost_usd, 0.015);

        usage.estimate_cost(0.02);
        assert!((usage.estimated_cost_usd - 0.003).abs() < 0.0001);
    }

    #[tokio::test]
    async fn test_loading_context() {
        let task_id = "test-task-001".to_string();
        let mut context = LoadingContext::new(task_id.clone(), LoadingStrategy::Immediate, 3);

        assert_eq!(context.task_id, task_id);
        assert_eq!(context.loading_strategy, LoadingStrategy::Immediate);
        assert_eq!(context.current_depth, 0);
        assert_eq!(context.max_depth, 3);
        assert!(context.loaded_chunks.is_empty());
        assert!(context.pending_chunks.is_empty());

        assert!(context.should_load_next_chunk());
        assert!(context.can_enter_next_depth());

        assert!(context.enter_next_depth());
        assert_eq!(context.current_depth, 1);

        assert!(context.enter_next_depth());
        assert_eq!(context.current_depth, 2);

        assert!(context.enter_next_depth());
        assert_eq!(context.current_depth, 3);

        assert!(!context.enter_next_depth());
        assert_eq!(context.current_depth, 3);

        context.add_pending_chunk("chunk-1".to_string());
        context.add_pending_chunk("chunk-2".to_string());
        assert_eq!(context.pending_chunks.len(), 2);

        context.mark_chunk_loaded("chunk-1".to_string());
        assert_eq!(context.loaded_chunks.len(), 1);
        assert_eq!(context.pending_chunks.len(), 1);
    }

    #[tokio::test]
    async fn test_progressive_loader_creation() {
        let loader = ProgressiveLoader::new(100000, 0.01);

        let task = Task::new("Test task for progressive loading".to_string(), 3);
        let task_id = task.id.clone();

        let context_result = loader
            .create_context(&task, LoadingStrategy::Batch, 3)
            .await;
        assert!(context_result.is_ok());

        let context = context_result.unwrap();
        assert_eq!(context.task_id, task_id);

        let retrieved_context = loader.get_context(&task_id).await;
        assert!(retrieved_context.is_some());
        assert_eq!(retrieved_context.unwrap().task_id, task_id);
    }

    #[tokio::test]
    async fn test_progressive_loader_chunk_management() {
        let loader = ProgressiveLoader::new(100000, 0.01);

        let task = Task::new("Test chunk management".to_string(), 3);
        let task_id = task.id.clone();

        let _ = loader
            .create_context(&task, LoadingStrategy::Immediate, 3)
            .await;

        let chunk = ContentChunk {
            chunk_id: "chunk-001".to_string(),
            content: "Test chunk content".to_string(),
            chunk_type: ChunkType::Context,
            priority: 5,
            estimated_tokens: 100,
            dependencies: Vec::new(),
        };

        loader.register_chunk(chunk.clone()).await;

        let loaded_chunk_result = loader.load_chunk(&task_id, &chunk.chunk_id).await;
        assert!(loaded_chunk_result.is_ok());

        let loaded_chunk = loaded_chunk_result.unwrap();
        assert!(loaded_chunk.is_some());
        assert_eq!(loaded_chunk.unwrap().chunk_id, chunk.chunk_id);

        let token_usage = loader.get_token_usage(&task_id).await;
        assert!(token_usage.is_some());
        assert_eq!(token_usage.unwrap().total_tokens, 100);

        assert!(loader.is_within_budget(&task_id).await);

        let summary = loader.get_loading_summary(&task_id).await;
        assert!(summary.is_some());
        assert_eq!(summary.unwrap().loaded_chunks_count, 1);
    }

    #[tokio::test]
    async fn test_loading_strategies() {
        let task_id = "test-strategies".to_string();

        let immediate_context = LoadingContext::new(task_id.clone(), LoadingStrategy::Immediate, 3);
        assert!(immediate_context.should_load_next_chunk());

        let mut lazy_context = LoadingContext::new(task_id.clone(), LoadingStrategy::Lazy, 3);
        assert!(lazy_context.should_load_next_chunk());
        lazy_context.mark_chunk_loaded("chunk-1".to_string());
        assert!(!lazy_context.should_load_next_chunk());

        let ondemand_context = LoadingContext::new(task_id.clone(), LoadingStrategy::OnDemand, 3);
        assert!(!ondemand_context.should_load_next_chunk());

        let mut batch_context = LoadingContext::new(task_id.clone(), LoadingStrategy::Batch, 3);
        assert!(batch_context.should_load_next_chunk());
        batch_context.mark_chunk_loaded("chunk-1".to_string());
        assert!(batch_context.should_load_next_chunk());
        batch_context.mark_chunk_loaded("chunk-2".to_string());
        assert!(batch_context.should_load_next_chunk());
        batch_context.mark_chunk_loaded("chunk-3".to_string());
        assert!(!batch_context.should_load_next_chunk());
    }

    #[tokio::test]
    async fn test_auth_manager_creation() {
        let secret_key = b"test-secret-key-for-jwt-auth";
        let auth_manager = AuthManager::new(secret_key);

        assert_eq!(auth_manager.user_count().await, 3);
    }

    #[tokio::test]
    async fn test_auth_manager_login() {
        let secret_key = b"test-secret-key-for-jwt-auth";
        let auth_manager = AuthManager::new(secret_key);

        let request = LoginRequest {
            username: "admin".to_string(),
            password: "admin123".to_string(),
        };

        let login_result = auth_manager.login(request).await;
        assert!(login_result.is_ok());

        let response = login_result.unwrap();
        assert!(!response.access_token.is_empty());
        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, 24 * 3600);
        assert_eq!(response.user.username, "admin");
        assert_eq!(response.user.role, UserRole::Admin);
    }

    #[tokio::test]
    async fn test_auth_manager_invalid_credentials() {
        let secret_key = b"test-secret-key-for-jwt-auth";
        let auth_manager = AuthManager::new(secret_key);

        let request = LoginRequest {
            username: "admin".to_string(),
            password: "wrong-password".to_string(),
        };

        let login_result = auth_manager.login(request).await;
        assert!(login_result.is_err());
    }

    #[tokio::test]
    async fn test_auth_manager_token_validation() {
        let secret_key = b"test-secret-key-for-jwt-auth";
        let auth_manager = AuthManager::new(secret_key);

        let request = LoginRequest {
            username: "admin".to_string(),
            password: "admin123".to_string(),
        };

        let response = auth_manager.login(request).await.unwrap();

        let validation_result = auth_manager.validate_token(&response.access_token);
        assert!(validation_result.is_ok());

        let claims = validation_result.unwrap();
        assert_eq!(claims.username, "admin");
        assert_eq!(claims.role, UserRole::Admin);
    }

    #[tokio::test]
    async fn test_auth_manager_add_user() {
        let secret_key = b"test-secret-key-for-jwt-auth";
        let auth_manager = AuthManager::new(secret_key);

        let add_result = auth_manager
            .add_user(
                "newuser".to_string(),
                "password123".to_string(),
                UserRole::Operator,
            )
            .await;
        assert!(add_result.is_ok());

        let login_request = LoginRequest {
            username: "newuser".to_string(),
            password: "password123".to_string(),
        };

        let login_result = auth_manager.login(login_request).await;
        assert!(login_result.is_ok());
    }

    #[tokio::test]
    async fn test_auth_manager_duplicate_user() {
        let secret_key = b"test-secret-key-for-jwt-auth";
        let auth_manager = AuthManager::new(secret_key);

        let add_result = auth_manager
            .add_user(
                "admin".to_string(),
                "password123".to_string(),
                UserRole::Operator,
            )
            .await;
        assert!(add_result.is_err());
    }

    #[tokio::test]
    async fn test_websocket_manager_creation() {
        let manager = WebSocketManager::new();
        assert_eq!(manager.client_count(), 0);
        assert_eq!(manager.task_subscription_count(), 0);
    }

    #[tokio::test]
    async fn test_websocket_client_registration() {
        let manager = WebSocketManager::new();
        let client_id = Uuid::new_v4();

        let _rx = manager.register_client(client_id);
        assert!(manager.has_client(&client_id));

        manager.unregister_client(client_id);
        assert!(!manager.has_client(&client_id));
    }

    #[tokio::test]
    async fn test_websocket_task_subscriptions() {
        let manager = WebSocketManager::new();
        let client_id = Uuid::new_v4();
        let task_id1 = Uuid::new_v4();
        let task_id2 = Uuid::new_v4();

        let _rx = manager.register_client(client_id);

        manager.subscribe_to_tasks(client_id, vec![task_id1, task_id2]);
        assert!(manager.has_task_subscription(&task_id1));
        assert!(manager.has_task_subscription(&task_id2));

        manager.unsubscribe_from_tasks(client_id, vec![task_id1]);
        assert!(!manager.is_subscribed_to_task(&client_id, &task_id1));
        assert!(manager.is_subscribed_to_task(&client_id, &task_id2));
    }

    #[tokio::test]
    async fn test_content_chunk_creation() {
        let chunk = ContentChunk {
            chunk_id: "test-chunk-001".to_string(),
            content: "This is a test content chunk with some information".to_string(),
            chunk_type: ChunkType::TaskDescription,
            priority: 8,
            estimated_tokens: 250,
            dependencies: vec!["dep-1".to_string(), "dep-2".to_string()],
        };

        assert_eq!(chunk.chunk_id, "test-chunk-001");
        assert_eq!(chunk.chunk_type, ChunkType::TaskDescription);
        assert_eq!(chunk.priority, 8);
        assert_eq!(chunk.estimated_tokens, 250);
        assert_eq!(chunk.dependencies.len(), 2);
    }

    #[tokio::test]
    async fn test_chunk_type_enum() {
        let types = vec![
            ChunkType::TaskDescription,
            ChunkType::Context,
            ChunkType::Examples,
            ChunkType::Tools,
            ChunkType::Constraints,
        ];

        for chunk_type in types {
            let chunk = ContentChunk {
                chunk_id: format!("{:?}", chunk_type),
                content: "test".to_string(),
                chunk_type: chunk_type.clone(),
                priority: 5,
                estimated_tokens: 10,
                dependencies: Vec::new(),
            };
            assert_eq!(chunk.chunk_type, chunk_type);
        }
    }

    #[tokio::test]
    async fn test_loading_summary() {
        let loader = ProgressiveLoader::new(100000, 0.01);
        let task = Task::new("Summary test task".to_string(), 3);
        let task_id = task.id.clone();

        let _ = loader.create_context(&task, LoadingStrategy::Lazy, 5).await;

        let summary = loader.get_loading_summary(&task_id).await;
        assert!(summary.is_some());

        let summary = summary.unwrap();
        assert_eq!(summary.task_id, task_id);
        assert_eq!(summary.loaded_chunks_count, 0);
        assert_eq!(summary.pending_chunks_count, 0);
        assert_eq!(summary.current_depth, 0);
        assert_eq!(summary.max_depth, 5);
        assert_eq!(summary.total_tokens_used, 0);
        assert_eq!(summary.loading_strategy, LoadingStrategy::Lazy);
    }

    #[tokio::test]
    async fn test_optimize_prompt() {
        let loader = ProgressiveLoader::new(100000, 0.01);
        let task = Task::new("Prompt optimization test".to_string(), 3);
        let task_id = task.id.clone();

        let _ = loader
            .create_context(&task, LoadingStrategy::Immediate, 3)
            .await;

        let chunk1 = ContentChunk {
            chunk_id: "chunk-1".to_string(),
            content: "Additional context information".to_string(),
            chunk_type: ChunkType::Context,
            priority: 5,
            estimated_tokens: 50,
            dependencies: Vec::new(),
        };

        let chunk2 = ContentChunk {
            chunk_id: "chunk-2".to_string(),
            content: "Example usage pattern".to_string(),
            chunk_type: ChunkType::Examples,
            priority: 7,
            estimated_tokens: 80,
            dependencies: Vec::new(),
        };

        loader.register_chunk(chunk1).await;
        loader.register_chunk(chunk2).await;

        let _ = loader.load_chunk(&task_id, "chunk-1").await;

        let base_prompt = "Base prompt content".to_string();
        let optimized = loader.optimize_prompt(&task_id, base_prompt.clone()).await;

        assert!(optimized.is_ok());
        let optimized_prompt = optimized.unwrap();
        assert!(optimized_prompt.contains(&base_prompt));
        assert!(optimized_prompt.contains("Additional context information"));
        assert!(!optimized_prompt.contains("Example usage pattern"));
    }
}
