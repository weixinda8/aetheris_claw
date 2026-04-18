#[cfg(test)]
mod tests {
    use aetheris::agent::{AgentConfig, AgentRegistry, AgentType, BaseAgent};
    use aetheris::streaming::state::InMemoryStateBackend;
    use aetheris::streaming::traits::StateBackend;
    use aetheris::utils::concurrency::{ConcurrencyLimiter, ConcurrencyMetrics};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Barrier;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_agent_registry_concurrent_registration() {
        let registry = Arc::new(AgentRegistry::new());
        let num_agents = 100;
        let barrier = Arc::new(Barrier::new(num_agents));
        
        let mut handles = Vec::with_capacity(num_agents);
        
        for i in 0..num_agents {
            let registry_clone = registry.clone();
            let barrier_clone = barrier.clone();
            
            let handle = tokio::spawn(async move {
                barrier_clone.wait().await;
                
                let config = AgentConfig::new(
                    format!("agent-{}", i),
                    format!("Test Agent {}", i),
                    AgentType::Generic,
                );
                let agent = BaseAgent::new_arc(config);
                
                registry_clone.register_agent(agent).unwrap();
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            handle.await.unwrap();
        }
        
        assert_eq!(registry.get_available_agents().len(), num_agents);
    }

    #[tokio::test]
    async fn test_agent_registry_concurrent_unregistration() {
        let registry = Arc::new(AgentRegistry::new());
        let num_agents = 50;
        
        for i in 0..num_agents {
            let config = AgentConfig::new(
                format!("agent-{}", i),
                format!("Test Agent {}", i),
                AgentType::Generic,
            );
            let agent = BaseAgent::new_arc(config);
            registry.register_agent(agent).unwrap();
        }
        
        assert_eq!(registry.get_available_agents().len(), num_agents);
        
        let barrier = Arc::new(Barrier::new(num_agents));
        let mut handles = Vec::with_capacity(num_agents);
        
        for i in 0..num_agents {
            let registry_clone = registry.clone();
            let barrier_clone = barrier.clone();
            
            let handle = tokio::spawn(async move {
                barrier_clone.wait().await;
                registry_clone.unregister_agent(&format!("agent-{}", i)).unwrap();
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            handle.await.unwrap();
        }
        
        assert_eq!(registry.get_available_agents().len(), 0);
    }

    #[tokio::test]
    async fn test_in_memory_state_backend_concurrent_access() {
        let backend = Arc::new(InMemoryStateBackend::new());
        let num_ops = 200;
        let barrier = Arc::new(Barrier::new(num_ops));
        
        let mut handles = Vec::with_capacity(num_ops);
        
        for i in 0..num_ops {
            let backend_clone = backend.clone();
            let barrier_clone = barrier.clone();
            
            let handle = tokio::spawn(async move {
                barrier_clone.wait().await;
                
                let key = format!("key-{}", i % 20);
                let value = format!("value-{}", i).into_bytes();
                
                let mut backend_mut = backend_clone.clone();
                backend_mut.put(key.clone(), value.clone()).await.unwrap();
                
                let retrieved = backend_clone.get(key).await.unwrap();
                assert!(retrieved.is_some());
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_concurrency_limiter() {
        let limiter = ConcurrencyLimiter::new(3);
        let num_tasks = 10;
        let barrier = Arc::new(Barrier::new(num_tasks));
        
        let mut handles = Vec::with_capacity(num_tasks);
        
        for i in 0..num_tasks {
            let limiter_clone = limiter.clone();
            let barrier_clone = barrier.clone();
            
            let handle = tokio::spawn(async move {
                barrier_clone.wait().await;
                
                let _guard = limiter_clone.acquire().await.unwrap();
                sleep(Duration::from_millis(10)).await;
                
                assert!(limiter_clone.available_permits() <= 3);
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            handle.await.unwrap();
        }
        
        assert_eq!(limiter.metrics().get_total_ops(), num_tasks as u64);
        assert_eq!(limiter.available_permits(), 3);
    }

    #[tokio::test]
    async fn test_concurrency_metrics() {
        let metrics = ConcurrencyMetrics::new();
        
        assert_eq!(metrics.get_total_ops(), 0);
        assert_eq!(metrics.get_successful_ops(), 0);
        assert_eq!(metrics.get_failed_ops(), 0);
        assert_eq!(metrics.get_success_rate(), 1.0);
        
        metrics.record_op(true);
        metrics.record_op(true);
        metrics.record_op(false);
        
        assert_eq!(metrics.get_total_ops(), 3);
        assert_eq!(metrics.get_successful_ops(), 2);
        assert_eq!(metrics.get_failed_ops(), 1);
        assert!((metrics.get_success_rate() - 0.666).abs() < 0.01);
        
        metrics.reset();
        
        assert_eq!(metrics.get_total_ops(), 0);
        assert_eq!(metrics.get_successful_ops(), 0);
        assert_eq!(metrics.get_failed_ops(), 0);
    }

    #[tokio::test]
    async fn test_mixed_concurrent_operations() {
        let registry = Arc::new(AgentRegistry::new());
        let backend = Arc::new(InMemoryStateBackend::new());
        let num_ops = 50;
        let barrier = Arc::new(Barrier::new(num_ops * 2));
        
        let mut handles = Vec::with_capacity(num_ops * 2);
        
        for i in 0..num_ops {
            let registry_clone = registry.clone();
            let backend_clone = backend.clone();
            let barrier_clone = barrier.clone();
            
            let registry_handle = tokio::spawn(async move {
                barrier_clone.wait().await;
                
                let config = AgentConfig::new(
                    format!("agent-{}", i),
                    format!("Test Agent {}", i),
                    AgentType::Generic,
                );
                let agent = BaseAgent::new_arc(config);
                registry_clone.register_agent(agent).unwrap();
            });
            
            let backend_handle = tokio::spawn(async move {
                barrier_clone.wait().await;
                
                let key = format!("key-{}", i);
                let value = format!("value-{}", i).into_bytes();
                let mut backend_mut = backend_clone.clone();
                backend_mut.put(key, value).await.unwrap();
            });
            
            handles.push(registry_handle);
            handles.push(backend_handle);
        }
        
        for handle in handles {
            handle.await.unwrap();
        }
        
        assert_eq!(registry.get_available_agents().len(), num_ops);
    }

    #[tokio::test]
    async fn test_concurrent_read_write() {
        let backend = Arc::new(InMemoryStateBackend::new());
        let num_readers = 50;
        let num_writers = 10;
        let total_ops = num_readers + num_writers;
        let barrier = Arc::new(Barrier::new(total_ops));
        
        let key = "shared-key".to_string();
        let initial_value = b"initial".to_vec();
        
        let mut backend_init = backend.clone();
        backend_init.put(key.clone(), initial_value.clone()).await.unwrap();
        
        let mut handles = Vec::with_capacity(total_ops);
        
        for _ in 0..num_readers {
            let backend_clone = backend.clone();
            let key_clone = key.clone();
            let barrier_clone = barrier.clone();
            
            let handle = tokio::spawn(async move {
                barrier_clone.wait().await;
                
                for _ in 0..10 {
                    let _ = backend_clone.get(key_clone.clone()).await.unwrap();
                }
            });
            
            handles.push(handle);
        }
        
        for i in 0..num_writers {
            let backend_clone = backend.clone();
            let key_clone = key.clone();
            let barrier_clone = barrier.clone();
            
            let handle = tokio::spawn(async move {
                barrier_clone.wait().await;
                
                let value = format!("writer-{}", i).into_bytes();
                let mut backend_mut = backend_clone.clone();
                backend_mut.put(key_clone, value).await.unwrap();
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            handle.await.unwrap();
        }
    }
}
