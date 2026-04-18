use tempfile::{tempdir, TempDir};
use std::path::PathBuf;
use std::sync::OnceLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub static E2E_TEST_CONFIG: OnceLock<E2EConfig> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2EConfig {
    pub enable_mock_external_services: bool,
    pub test_timeout_seconds: u64,
    pub enable_parallel_execution: bool,
    pub max_concurrent_tests: usize,
}

impl Default for E2EConfig {
    fn default() -> Self {
        Self {
            enable_mock_external_services: true,
            test_timeout_seconds: 120,
            enable_parallel_execution: true,
            max_concurrent_tests: 4,
        }
    }
}

pub fn init_e2e_config() -> &'static E2EConfig {
    E2E_TEST_CONFIG.get_or_init(|| E2EConfig::default())
}

pub struct E2ETempDir {
    dir: TempDir,
}

impl E2ETempDir {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            dir: tempdir()?,
        })
    }

    pub fn path(&self) -> &std::path::Path {
        self.dir.path()
    }

    pub fn create_file(&self, name: &str, content: &str) -> std::io::Result<PathBuf> {
        let file_path = self.path().join(name);
        std::fs::write(&file_path, content)?;
        Ok(file_path)
    }

    pub fn create_json_file(&self, name: &str, value: &serde_json::Value) -> std::io::Result<PathBuf> {
        let content = serde_json::to_string_pretty(value)?;
        self.create_file(name, &content)
    }

    pub fn create_toml_file<T: Serialize>(&self, name: &str, value: &T) -> std::io::Result<PathBuf> {
        let content = toml::to_string_pretty(value)?;
        self.create_file(name, &content)
    }

    pub fn create_dir(&self, name: &str) -> std::io::Result<PathBuf> {
        let dir_path = self.path().join(name);
        std::fs::create_dir_all(&dir_path)?;
        Ok(dir_path)
    }
}

pub struct E2ETestEnvironment {
    pub temp_dir: E2ETempDir,
    pub config: &'static E2EConfig,
    pub test_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub logs: Vec<String>,
}

impl E2ETestEnvironment {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            temp_dir: E2ETempDir::new()?,
            config: init_e2e_config(),
            test_id: Uuid::new_v4(),
            start_time: Utc::now(),
            logs: Vec::new(),
        })
    }

    pub fn log(&mut self, message: &str) {
        let timestamp = Utc::now().to_rfc3339();
        self.logs.push(format!("[{}] {}", timestamp, message));
    }

    pub fn elapsed_time(&self) -> chrono::Duration {
        Utc::now() - self.start_time
    }

    pub fn get_souls_dir(&self) -> std::io::Result<PathBuf> {
        self.temp_dir.create_dir("souls")
    }

    pub fn get_skills_dir(&self) -> std::io::Result<PathBuf> {
        self.temp_dir.create_dir("skills")
    }

    pub fn get_config_dir(&self) -> std::io::Result<PathBuf> {
        self.temp_dir.create_dir("config")
    }
}

pub struct E2EDataGenerator;

impl E2EDataGenerator {
    pub fn generate_test_soul_content(name: &str, version: &str) -> String {
        format!(r#"---
name: {}
description: Test soul for E2E testing
personality: Friendly and helpful
version: {}
tags:
  - e2e
  - test
---

# {}

This is a test soul created for end-to-end testing purposes.
"#, name, version, name)
    }

    pub fn generate_test_config() -> serde_json::Value {
        json!({
            "server": {
                "host": "127.0.0.1",
                "port": 0
            },
            "llm": {
                "provider": "mock",
                "model": "gpt-4",
                "temperature": 0.7
            },
            "security": {
                "enabled": true,
                "sandbox_level": 2
            }
        })
    }

    pub fn generate_test_skill_metadata(id: &str, name: &str, version: &str) -> serde_json::Value {
        json!({
            "id": id,
            "name": name,
            "version": version,
            "description": format!("Test skill: {}", name),
            "author": "e2e-test",
            "priority": "high",
            "categories": ["test", "e2e"]
        })
    }

    pub fn generate_random_string(length: usize) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    pub fn generate_test_task(name: &str) -> serde_json::Value {
        json!({
            "id": Uuid::new_v4().to_string(),
            "name": name,
            "description": format!("Test task: {}", name),
            "status": "pending",
            "created_at": Utc::now().to_rfc3339(),
            "updated_at": Utc::now().to_rfc3339(),
            "dependencies": [],
            "metadata": {}
        })
    }
}

pub mod assertions {
    use super::*;

    pub fn assert_contains(haystack: &str, needle: &str) {
        assert!(
            haystack.contains(needle),
            "Expected '{}' to contain '{}', but it did not",
            haystack,
            needle
        );
    }

    pub fn assert_not_contains(haystack: &str, needle: &str) {
        assert!(
            !haystack.contains(needle),
            "Expected '{}' to NOT contain '{}', but it did",
            haystack,
            needle
        );
    }

    pub fn assert_is_uuid(s: &str) {
        assert!(
            Uuid::parse_str(s).is_ok(),
            "Expected '{}' to be a valid UUID",
            s
        );
    }

    pub fn assert_json_contains(value: &serde_json::Value, key: &str) {
        assert!(
            value.get(key).is_some(),
            "Expected JSON to contain key '{}', but it did not",
            key
        );
    }

    pub fn assert_json_has_string(value: &serde_json::Value, key: &str, expected: &str) {
        assert_json_contains(value, key);
        assert_eq!(
            value[key].as_str(),
            Some(expected),
            "Expected JSON key '{}' to be '{}', but got '{:?}'",
            key,
            expected,
            value[key]
        );
    }

    pub fn assert_path_exists(path: &std::path::Path) {
        assert!(
            path.exists(),
            "Expected path '{}' to exist, but it does not",
            path.display()
        );
    }

    pub fn assert_file_not_empty(path: &std::path::Path) {
        assert_path_exists(path);
        let metadata = std::fs::metadata(path).unwrap();
        assert!(
            metadata.len() > 0,
            "Expected file '{}' to be non-empty, but it is empty",
            path.display()
        );
    }
}

pub mod async_helpers {
    use tokio::time::{timeout, Duration};

    pub async fn with_timeout<F, T>(fut: F, duration: Duration) -> T
    where
        F: std::future::Future<Output = T>,
    {
        timeout(duration, fut)
            .await
            .expect("Test timed out")
    }

    pub async fn assert_eventually_true<F, Fut>(predicate: F, max_wait: Duration)
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        let start = std::time::Instant::now();
        let check_interval = Duration::from_millis(50);

        while start.elapsed() < max_wait {
            if predicate().await {
                return;
            }
            tokio::time::sleep(check_interval).await;
        }

        panic!("Condition did not become true within {:?}", max_wait);
    }
}
