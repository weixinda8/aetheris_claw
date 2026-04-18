#[cfg(test)]
mod tests {
    use aetheris::agent::{
        Agent, AgentRegistry, CodeAgent, ComplianceAgent, DataAgent, IndustrialAgent, OfficeAgent,
        OpsAgent,
    };
    use aetheris::core::{Task, TaskStatus};
    use aetheris::utils::Result;

    #[tokio::test]
    async fn test_code_agent_generate_code() {
        let mut agent = CodeAgent::new(None, None);
        let task = Task::new("Generate code for a hello world program".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(completed_task.result.as_ref().unwrap().contains("code"));
    }

    #[tokio::test]
    async fn test_code_agent_code_review() {
        let mut agent = CodeAgent::new(None, None);
        let task = Task::new("Please perform a code review on this code".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(completed_task.result.as_ref().unwrap().contains("review"));
    }

    #[tokio::test]
    async fn test_code_agent_execute_code() {
        let mut agent = CodeAgent::new(None, None);
        let task = Task::new("Execute this code and show the results".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
    }

    #[tokio::test]
    async fn test_code_agent_can_handle() {
        let agent = CodeAgent::new(None, None);

        let mut task1 = Task::new("Generate code".to_string(), 5);
        task1.tags = vec!["code".to_string()];
        assert!(agent.can_handle(&task1));

        let mut task2 = Task::new("Code review".to_string(), 5);
        task2.tags = vec!["programming".to_string()];
        assert!(agent.can_handle(&task2));

        let task3 = Task::new("Analyze data".to_string(), 5);
        assert!(!agent.can_handle(&task3));
    }

    #[tokio::test]
    async fn test_data_agent_query_data() {
        let mut agent = DataAgent::new(None, None);
        let task = Task::new("Query data from the database".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(completed_task.result.as_ref().unwrap().contains("query"));
    }

    #[tokio::test]
    async fn test_data_agent_clean_data() {
        let mut agent = DataAgent::new(None, None);
        let task = Task::new("Clean the dataset and remove duplicates".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(completed_task.result.as_ref().unwrap().contains("clean"));
    }

    #[tokio::test]
    async fn test_data_agent_analyze_data() {
        let mut agent = DataAgent::new(None, None);
        let task = Task::new("Analyze the data and generate insights".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(completed_task.result.as_ref().unwrap().contains("analyze"));
    }

    #[tokio::test]
    async fn test_ops_agent_deploy() {
        let mut agent = OpsAgent::new(None, None);
        let task = Task::new("Deploy the application to production".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(completed_task.result.as_ref().unwrap().contains("deploy"));
    }

    #[tokio::test]
    async fn test_ops_agent_monitor() {
        let mut agent = OpsAgent::new(None, None);
        let task = Task::new("Monitor the system health".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(completed_task.result.as_ref().unwrap().contains("monitor"));
    }

    #[tokio::test]
    async fn test_ops_agent_logs() {
        let mut agent = OpsAgent::new(None, None);
        let task = Task::new("Collect and analyze logs".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(completed_task.result.as_ref().unwrap().contains("logs"));
    }

    #[tokio::test]
    async fn test_office_agent_document() {
        let mut agent = OfficeAgent::new(None, None);
        let task = Task::new("Generate a documentation file".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(completed_task.result.as_ref().unwrap().contains("document"));
    }

    #[tokio::test]
    async fn test_office_agent_report() {
        let mut agent = OfficeAgent::new(None, None);
        let task = Task::new("Write a quarterly business report".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(completed_task.result.as_ref().unwrap().contains("report"));
    }

    #[tokio::test]
    async fn test_office_agent_email() {
        let mut agent = OfficeAgent::new(None, None);
        let task = Task::new("Compose and send an email".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(completed_task.result.as_ref().unwrap().contains("email"));
    }

    #[tokio::test]
    async fn test_industrial_agent_monitor() {
        let mut agent = IndustrialAgent::new(None, None);
        let task = Task::new("Monitor the production line".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(completed_task.result.as_ref().unwrap().contains("monitor"));
    }

    #[tokio::test]
    async fn test_industrial_agent_control() {
        let mut agent = IndustrialAgent::new(None, None);
        let task = Task::new("Control the equipment parameters".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(completed_task.result.as_ref().unwrap().contains("control"));
    }

    #[tokio::test]
    async fn test_industrial_agent_automation() {
        let mut agent = IndustrialAgent::new(None, None);
        let task = Task::new("Start the automated production workflow".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(
            completed_task
                .result
                .as_ref()
                .unwrap()
                .contains("automation")
        );
    }

    #[tokio::test]
    async fn test_compliance_agent_compliance() {
        let mut agent = ComplianceAgent::new(None, None);
        let task = Task::new("Perform a compliance check".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(
            completed_task
                .result
                .as_ref()
                .unwrap()
                .contains("compliance")
        );
    }

    #[tokio::test]
    async fn test_compliance_agent_audit() {
        let mut agent = ComplianceAgent::new(None, None);
        let task = Task::new("Conduct a security audit".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(completed_task.result.as_ref().unwrap().contains("audit"));
    }

    #[tokio::test]
    async fn test_compliance_agent_risk() {
        let mut agent = ComplianceAgent::new(None, None);
        let task = Task::new("Evaluate the risk factors".to_string(), 5);

        let result: Result<Task> = agent.execute(task.clone()).await;
        assert!(result.is_ok());

        let completed_task = result.unwrap();
        assert_eq!(completed_task.status, TaskStatus::Completed);
        assert!(completed_task.result.is_some());
        assert!(completed_task.result.as_ref().unwrap().contains("risk"));
    }

    #[tokio::test]
    async fn test_agent_registry_with_domain_agents() {
        let registry = AgentRegistry::new();

        registry
            .register_agent(CodeAgent::new_arc(None, None))
            .unwrap();
        registry
            .register_agent(DataAgent::new_arc(None, None))
            .unwrap();
        registry
            .register_agent(OpsAgent::new_arc(None, None))
            .unwrap();
        registry
            .register_agent(OfficeAgent::new_arc(None, None))
            .unwrap();
        registry
            .register_agent(IndustrialAgent::new_arc(None, None))
            .unwrap();
        registry
            .register_agent(ComplianceAgent::new_arc(None, None))
            .unwrap();

        let agents = registry.list_all_agents();
        assert_eq!(agents.len(), 6);
    }

    #[cfg(feature = "docker-tests")]
    mod docker_tests {
        use aetheris::runtime::sandbox::{DockerSandbox, SandboxConfig};
        use aetheris::utils::Result;

        #[tokio::test]
        async fn test_docker_sandbox_health_check() -> Result<()> {
            let sandbox = DockerSandbox::connect_with_defaults().await?;
            let is_healthy = sandbox.health_check().await?;
            assert!(is_healthy);
            Ok(())
        }

        #[tokio::test]
        async fn test_docker_sandbox_get_version() -> Result<()> {
            let sandbox = DockerSandbox::connect_with_defaults().await?;
            let version = sandbox.get_version().await?;
            assert!(!version.is_empty());
            assert_ne!(version, "Unknown");
            Ok(())
        }

        #[tokio::test]
        async fn test_docker_sandbox_pull_image() -> Result<()> {
            let sandbox = DockerSandbox::connect_with_defaults().await?;
            sandbox.pull_image("alpine:latest").await?;
            Ok(())
        }

        #[tokio::test]
        async fn test_docker_sandbox_execute_echo() -> Result<()> {
            let sandbox = DockerSandbox::connect_with_defaults().await?;
            let result = sandbox
                .execute("alpine:latest", "echo Hello from Docker!")
                .await?;

            assert!(result.success);
            assert!(result.output.is_some());
            assert_eq!(result.output.unwrap(), "Hello from Docker!");
            assert!(result.error.is_none());
            Ok(())
        }

        #[tokio::test]
        async fn test_docker_sandbox_execute_ls() -> Result<()> {
            let sandbox = DockerSandbox::connect_with_defaults().await?;
            let result = sandbox.execute("alpine:latest", "ls -la /").await?;

            assert!(result.success);
            assert!(result.output.is_some());
            let output = result.output.unwrap();
            assert!(output.contains("bin"));
            assert!(output.contains("etc"));
            assert!(output.contains("usr"));
            Ok(())
        }

        #[tokio::test]
        async fn test_docker_sandbox_lifecycle() -> Result<()> {
            let sandbox = DockerSandbox::connect_with_defaults().await?;

            sandbox.pull_image("alpine:latest").await?;

            let container_id = sandbox.create_container("alpine:latest", None).await?;
            assert!(!container_id.is_empty());

            sandbox.start_container(&container_id).await?;

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let (stdout, stderr) = sandbox.exec_command(&container_id, "echo Test").await?;
            assert_eq!(stdout.trim(), "Test");
            assert!(stderr.is_empty());

            sandbox.stop_container(&container_id).await?;
            sandbox.remove_container(&container_id, true).await?;

            Ok(())
        }
    }
}
