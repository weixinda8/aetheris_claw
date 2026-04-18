use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::OnceLock;
use tempfile::{TempDir, tempdir};
use uuid::Uuid;

pub static TEST_CONFIG: OnceLock<TestConfig> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub test_database_url: Option<String>,
    pub enable_integration_tests: bool,
    pub enable_benchmarks: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            test_database_url: None,
            enable_integration_tests: false,
            enable_benchmarks: false,
        }
    }
}

pub fn init_test_config() -> &'static TestConfig {
    TEST_CONFIG.get_or_init(|| TestConfig::default())
}

pub struct TestTempDir {
    dir: TempDir,
}

impl TestTempDir {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self { dir: tempdir()? })
    }

    pub fn path(&self) -> &std::path::Path {
        self.dir.path()
    }

    pub fn create_file(&self, name: &str, content: &str) -> std::io::Result<PathBuf> {
        let file_path = self.path().join(name);
        std::fs::write(&file_path, content)?;
        Ok(file_path)
    }

    pub fn create_json_file(
        &self,
        name: &str,
        value: &serde_json::Value,
    ) -> std::io::Result<PathBuf> {
        let content = serde_json::to_string_pretty(value)?;
        self.create_file(name, &content)
    }

    pub fn create_toml_file<T: Serialize>(
        &self,
        name: &str,
        value: &T,
    ) -> std::io::Result<PathBuf> {
        let content = toml::to_string_pretty(value)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        self.create_file(name, &content)
    }
}

#[derive(Debug, Clone)]
pub struct MockPersona {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub version: String,
    pub created_at: DateTime<Utc>,
}

impl MockPersona {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: format!("Mock persona for testing: {}", name),
            version: "1.0.0".to_string(),
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    pub fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "id": self.id.to_string(),
            "name": self.name,
            "description": self.description,
            "version": self.version,
            "created_at": self.created_at.to_rfc3339(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MockSkill {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
}

impl MockSkill {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: format!("Mock skill for testing: {}", name),
            version: "1.0.0".to_string(),
            author: "test-author".to_string(),
        }
    }

    pub fn with_author(mut self, author: &str) -> Self {
        self.author = author.to_string();
        self
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "id": self.id.to_string(),
            "name": self.name,
            "description": self.description,
            "version": self.version,
            "author": self.author,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MockSecurityContext {
    pub user_id: Uuid,
    pub capabilities: Vec<String>,
    pub is_admin: bool,
}

impl MockSecurityContext {
    pub fn new() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            capabilities: vec!["read".to_string(), "write".to_string()],
            is_admin: false,
        }
    }

    pub fn admin() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            capabilities: vec!["*".to_string()],
            is_admin: true,
        }
    }

    pub fn with_capability(mut self, capability: &str) -> Self {
        self.capabilities.push(capability.to_string());
        self
    }
}

impl Default for MockSecurityContext {
    fn default() -> Self {
        Self::new()
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
}

pub mod generators {
    use super::*;
    use rand::Rng;

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

    pub fn generate_random_email() -> String {
        format!(
            "{}@{}.com",
            generate_random_string(10),
            generate_random_string(8)
        )
    }

    pub fn generate_random_port() -> u16 {
        let mut rng = rand::thread_rng();
        rng.gen_range(1024..65535)
    }

    pub fn generate_mock_personas(count: usize) -> Vec<MockPersona> {
        (0..count)
            .map(|i| MockPersona::new(&format!("Persona {}", i)))
            .collect()
    }

    pub fn generate_mock_skills(count: usize) -> Vec<MockSkill> {
        (0..count)
            .map(|i| MockSkill::new(&format!("Skill {}", i)))
            .collect()
    }
}

pub mod async_helpers {
    use tokio::time::{Duration, timeout};

    pub async fn with_timeout<F, T>(fut: F, duration: Duration) -> T
    where
        F: std::future::Future<Output = T>,
    {
        timeout(duration, fut).await.expect("Test timed out")
    }

    pub async fn assert_eventually_true<F, Fut>(predicate: F, max_wait: Duration)
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        let start = std::time::Instant::now();
        let check_interval = Duration::from_millis(10);

        while start.elapsed() < max_wait {
            if predicate().await {
                return;
            }
            tokio::time::sleep(check_interval).await;
        }

        panic!("Condition did not become true within {:?}", max_wait);
    }
}
