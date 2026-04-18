use crate::skill::agentskills::{AgentSkillManifest, AgentSkillsRegistry};
use crate::utils::{AetherisError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClawHubSkillInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub downloads: u64,
    pub stars: u32,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub deprecated: bool,
    pub average_rating: Option<f64>,
    pub rating_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClawHubSearchResult {
    pub total: u64,
    pub skills: Vec<ClawHubSkillInfo>,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AetherisSkillHubSkillInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub author_id: String,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: String,
    pub download_count: i64,
    pub average_rating: f64,
    pub rating_count: i32,
    pub success_rate: f64,
    pub execution_count: i64,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
    pub deprecated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AetherisSkillHubSearchResult {
    pub skills: Vec<AetherisSkillHubSkillInfo>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReviewRequest {
    pub skill_id: String,
    pub rating: i32,
    pub title: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillReview {
    pub id: String,
    pub skill_id: String,
    pub user_id: String,
    pub rating: i32,
    pub title: Option<String>,
    pub content: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordExecutionRequest {
    pub skill_id: String,
    pub version: String,
    pub success: bool,
    pub execution_time_ms: Option<i64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUpdateCheck {
    pub skill_id: String,
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub changelog: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 10000,
            backoff_multiplier: 2.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClawHubClient {
    base_url: String,
    client: Client,
    cache_dir: PathBuf,
    retry_config: RetryConfig,
}

impl ClawHubClient {
    pub fn new(base_url: String, cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;

        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self {
            base_url,
            client,
            cache_dir,
            retry_config: RetryConfig::default(),
        })
    }

    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    pub fn new_default() -> Result<Self> {
        let cache_dir = dirs::home_dir()
            .ok_or_else(|| AetherisError::ClawHub("Could not find home directory".to_string()))?
            .join(".aetheris")
            .join("clawhub");

        Self::new("https://clawhub.io".to_string(), cache_dir)
    }

    async fn retry_request<F, Fut, T, E>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut last_error = None;
        let mut delay = self.retry_config.initial_delay_ms;

        for attempt in 1..=self.retry_config.max_attempts {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    warn!("Attempt {} failed: {}", attempt, e);
                    last_error = Some(e);

                    if attempt < self.retry_config.max_attempts {
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        delay = std::cmp::min(
                            (delay as f64 * self.retry_config.backoff_multiplier) as u64,
                            self.retry_config.max_delay_ms,
                        );
                    }
                }
            }
        }

        Err(AetherisError::ClawHub(format!(
            "Operation failed after {} attempts: {}",
            self.retry_config.max_attempts,
            last_error.unwrap()
        )))
    }

    pub async fn search_skills(
        &self,
        query: &str,
        page: u32,
        per_page: u32,
    ) -> Result<ClawHubSearchResult> {
        let url = format!(
            "{}/api/v1/skills/search?q={}&page={}&per_page={}",
            self.base_url,
            urlencoding::encode(query),
            page,
            per_page
        );

        self.retry_request(|| async {
            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                return Err(AetherisError::ClawHub(format!(
                    "ClawHub API error: {}",
                    response.status()
                )));
            }

            let result: ClawHubSearchResult = response.json().await?;
            Ok(result)
        })
        .await
    }

    pub async fn get_skill_info(&self, skill_id: &str) -> Result<ClawHubSkillInfo> {
        let url = format!("{}/api/v1/skills/{}", self.base_url, skill_id);

        self.retry_request(|| async {
            let response = self.client.get(&url).send().await?;

            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(AetherisError::NotFound(format!(
                    "Skill not found: {}",
                    skill_id
                )));
            }

            if !response.status().is_success() {
                return Err(AetherisError::ClawHub(format!(
                    "ClawHub API error: {}",
                    response.status()
                )));
            }

            let info: ClawHubSkillInfo = response.json().await?;
            Ok(info)
        })
        .await
    }

    pub async fn download_skill(
        &self,
        skill_id: &str,
        version: Option<&str>,
    ) -> Result<AgentSkillManifest> {
        let url = match version {
            Some(v) => format!(
                "{}/api/v1/skills/{}/{}/manifest",
                self.base_url, skill_id, v
            ),
            None => format!("{}/api/v1/skills/{}/manifest", self.base_url, skill_id),
        };

        self.retry_request(|| async {
            let response = self.client.get(&url).send().await?;

            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(AetherisError::NotFound(format!(
                    "Skill not found: {}",
                    skill_id
                )));
            }

            if !response.status().is_success() {
                return Err(AetherisError::ClawHub(format!(
                    "ClawHub API error: {}",
                    response.status()
                )));
            }

            let manifest: AgentSkillManifest = response.json().await?;
            Ok(manifest)
        })
        .await
    }

    pub async fn install_skill(
        &self,
        registry: &mut AgentSkillsRegistry,
        skill_id: &str,
        version: Option<&str>,
    ) -> Result<AgentSkillManifest> {
        let manifest = self.download_skill(skill_id, version).await?;

        let cache_path = self
            .cache_dir
            .join(format!("{}-{}.yaml", skill_id, manifest.metadata.version));

        manifest.save(cache_path)?;

        if registry.get(&manifest.metadata.id).is_some() {
            registry.remove(&manifest.metadata.id)?;
        }

        registry.add(manifest.clone())?;

        info!(
            "Successfully installed skill {} version {}",
            manifest.metadata.id, manifest.metadata.version
        );

        Ok(manifest)
    }

    pub async fn check_for_update(
        &self,
        skill_id: &str,
        current_version: &str,
    ) -> Result<SkillUpdateCheck> {
        let remote_info = self.get_skill_info(skill_id).await?;
        let has_update = remote_info.version != current_version;

        Ok(SkillUpdateCheck {
            skill_id: skill_id.to_string(),
            current_version: current_version.to_string(),
            latest_version: remote_info.version,
            has_update,
            changelog: None,
        })
    }

    pub async fn update_skill(
        &self,
        registry: &mut AgentSkillsRegistry,
        skill_id: &str,
    ) -> Result<AgentSkillManifest> {
        let local_manifest = registry
            .get(skill_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Skill not found: {}", skill_id)))?;

        let update_check = self
            .check_for_update(skill_id, &local_manifest.metadata.version)
            .await?;

        if update_check.has_update {
            info!(
                "Updating skill {} from {} to {}",
                skill_id, update_check.current_version, update_check.latest_version
            );
            let new_manifest = self.install_skill(registry, skill_id, None).await?;
            return Ok(new_manifest);
        }

        Ok(local_manifest.clone())
    }

    pub async fn uninstall_skill(
        &self,
        registry: &mut AgentSkillsRegistry,
        skill_id: &str,
    ) -> Result<()> {
        registry.remove(skill_id)?;

        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with(skill_id) && file_name.ends_with(".yaml") {
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        }

        info!("Successfully uninstalled skill {}", skill_id);
        Ok(())
    }

    pub async fn list_installed_skills(
        &self,
        registry: &AgentSkillsRegistry,
    ) -> Vec<AgentSkillManifest> {
        registry.list().iter().map(|s| (*s).clone()).collect()
    }

    pub fn get_cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    pub fn get_base_url(&self) -> &str {
        &self.base_url
    }
}

#[derive(Debug, Clone)]
pub struct AetherisSkillHubClient {
    base_url: String,
    client: Client,
    cache_dir: PathBuf,
    retry_config: RetryConfig,
    auth_token: Option<String>,
}

impl AetherisSkillHubClient {
    pub fn new(base_url: String, cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;

        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self {
            base_url,
            client,
            cache_dir,
            retry_config: RetryConfig::default(),
            auth_token: None,
        })
    }

    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    pub fn with_auth_token(mut self, token: String) -> Self {
        self.auth_token = Some(token);
        self
    }

    pub fn new_default() -> Result<Self> {
        let cache_dir = dirs::home_dir()
            .ok_or_else(|| AetherisError::ClawHub("Could not find home directory".to_string()))?
            .join(".aetheris")
            .join("skillhub");

        Self::new("https://skillhub.aetheris.io".to_string(), cache_dir)
    }

    async fn retry_request<F, Fut, T, E>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut last_error = None;
        let mut delay = self.retry_config.initial_delay_ms;

        for attempt in 1..=self.retry_config.max_attempts {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    warn!("Attempt {} failed: {}", attempt, e);
                    last_error = Some(e);

                    if attempt < self.retry_config.max_attempts {
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        delay = std::cmp::min(
                            (delay as f64 * self.retry_config.backoff_multiplier) as u64,
                            self.retry_config.max_delay_ms,
                        );
                    }
                }
            }
        }

        Err(AetherisError::ClawHub(format!(
            "Operation failed after {} attempts: {}",
            self.retry_config.max_attempts,
            last_error.unwrap()
        )))
    }

    fn build_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut builder = self.client.get(url);
        if let Some(token) = &self.auth_token {
            builder = builder.bearer_auth(token);
        }
        builder
    }

    fn build_post_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut builder = self.client.post(url);
        if let Some(token) = &self.auth_token {
            builder = builder.bearer_auth(token);
        }
        builder
    }

    pub async fn search_skills(
        &self,
        query: &str,
        page: u32,
        per_page: u32,
    ) -> Result<AetherisSkillHubSearchResult> {
        let url = format!(
            "{}/api/v1/skills?query={}&page={}&page_size={}",
            self.base_url,
            query,
            page,
            per_page
        );

        self.retry_request(|| async {
            let response = self.build_request(&url).send().await?;

            if !response.status().is_success() {
                return Err(AetherisError::ClawHub(format!(
                    "Aetheris Skill Hub API error: {}",
                    response.status()
                )));
            }

            let result: AetherisSkillHubSearchResult = response.json().await?;
            Ok(result)
        })
        .await
    }

    pub async fn get_skill_info(&self, skill_id: &str) -> Result<AetherisSkillHubSkillInfo> {
        let url = format!("{}/api/v1/skills/{}", self.base_url, skill_id);

        self.retry_request(|| async {
            let response = self.build_request(&url).send().await?;

            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(AetherisError::NotFound(format!(
                    "Skill not found: {}",
                    skill_id
                )));
            }

            if !response.status().is_success() {
                return Err(AetherisError::ClawHub(format!(
                    "Aetheris Skill Hub API error: {}",
                    response.status()
                )));
            }

            let info: AetherisSkillHubSkillInfo = response.json().await?;
            Ok(info)
        })
        .await
    }

    pub async fn download_skill(
        &self,
        skill_id: &str,
        _version: Option<&str>,
    ) -> Result<AgentSkillManifest> {
        let url = format!("{}/api/v1/skills/{}/download", self.base_url, skill_id);

        self.retry_request(|| async {
            let response = self.build_request(&url).send().await?;

            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(AetherisError::NotFound(format!(
                    "Skill not found: {}",
                    skill_id
                )));
            }

            if !response.status().is_success() {
                return Err(AetherisError::ClawHub(format!(
                    "Aetheris Skill Hub API error: {}",
                    response.status()
                )));
            }

            let download_response: serde_json::Value = response.json().await?;
            let content = &download_response["content"];
            let manifest: AgentSkillManifest =
                serde_json::from_value(content.clone()).map_err(|e| {
                    AetherisError::ClawHub(format!("Failed to parse skill manifest: {}", e))
                })?;

            Ok(manifest)
        })
        .await
    }

    pub async fn install_skill(
        &self,
        registry: &mut AgentSkillsRegistry,
        skill_id: &str,
        version: Option<&str>,
    ) -> Result<AgentSkillManifest> {
        let manifest = self.download_skill(skill_id, version).await?;

        let cache_path = self
            .cache_dir
            .join(format!("{}-{}.yaml", skill_id, manifest.metadata.version));

        manifest.save(cache_path)?;

        if registry.get(&manifest.metadata.id).is_some() {
            registry.remove(&manifest.metadata.id)?;
        }

        registry.add(manifest.clone())?;

        info!(
            "Successfully installed skill {} version {} from Aetheris Skill Hub",
            manifest.metadata.id, manifest.metadata.version
        );

        Ok(manifest)
    }

    pub async fn check_for_update(
        &self,
        skill_id: &str,
        current_version: &str,
    ) -> Result<SkillUpdateCheck> {
        let remote_info = self.get_skill_info(skill_id).await?;
        let has_update = remote_info.version != current_version;

        Ok(SkillUpdateCheck {
            skill_id: skill_id.to_string(),
            current_version: current_version.to_string(),
            latest_version: remote_info.version,
            has_update,
            changelog: None,
        })
    }

    pub async fn update_skill(
        &self,
        registry: &mut AgentSkillsRegistry,
        skill_id: &str,
    ) -> Result<AgentSkillManifest> {
        let local_manifest = registry
            .get(skill_id)
            .ok_or_else(|| AetherisError::NotFound(format!("Skill not found: {}", skill_id)))?;

        let update_check = self
            .check_for_update(skill_id, &local_manifest.metadata.version)
            .await?;

        if update_check.has_update {
            info!(
                "Updating skill {} from {} to {} from Aetheris Skill Hub",
                skill_id, update_check.current_version, update_check.latest_version
            );
            let new_manifest = self.install_skill(registry, skill_id, None).await?;
            return Ok(new_manifest);
        }

        Ok(local_manifest.clone())
    }

    pub async fn submit_review(&self, request: CreateReviewRequest) -> Result<SkillReview> {
        let url = format!("{}/api/v1/reviews", self.base_url);

        self.retry_request(|| async {
            let response = self.build_post_request(&url).json(&request).send().await?;

            if !response.status().is_success() {
                return Err(AetherisError::ClawHub(format!(
                    "Failed to submit review: {}",
                    response.status()
                )));
            }

            let review: SkillReview = response.json().await?;
            Ok(review)
        })
        .await
    }

    pub async fn record_execution(&self, request: RecordExecutionRequest) -> Result<()> {
        let url = format!("{}/api/v1/skills/executions", self.base_url);

        self.retry_request(|| async {
            let response = self.build_post_request(&url).json(&request).send().await?;

            if !response.status().is_success() {
                return Err(AetherisError::ClawHub(format!(
                    "Failed to record execution: {}",
                    response.status()
                )));
            }

            Ok(())
        })
        .await
    }

    pub async fn uninstall_skill(
        &self,
        registry: &mut AgentSkillsRegistry,
        skill_id: &str,
    ) -> Result<()> {
        registry.remove(skill_id)?;

        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with(skill_id) && file_name.ends_with(".yaml") {
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        }

        info!(
            "Successfully uninstalled skill {} from Aetheris Skill Hub",
            skill_id
        );
        Ok(())
    }

    pub fn get_cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    pub fn get_base_url(&self) -> &str {
        &self.base_url
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSkillInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub downloads: u64,
    pub stars: Option<u32>,
    pub rating: Option<f64>,
    pub rating_count: Option<u32>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub source: SkillSource,
    pub created_at: String,
    pub updated_at: String,
    pub deprecated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillSource {
    ClawHub,
    AetherisSkillHub,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSearchResult {
    pub total: u64,
    pub skills: Vec<UnifiedSkillInfo>,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Clone)]
pub struct UnifiedSkillHubClient {
    clawhub: ClawHubClient,
    skillhub: AetherisSkillHubClient,
}

impl UnifiedSkillHubClient {
    pub fn new(clawhub: ClawHubClient, skillhub: AetherisSkillHubClient) -> Self {
        Self { clawhub, skillhub }
    }

    pub fn new_default() -> Result<Self> {
        Ok(Self {
            clawhub: ClawHubClient::new_default()?,
            skillhub: AetherisSkillHubClient::new_default()?,
        })
    }

    pub async fn search_skills(
        &self,
        query: &str,
        page: u32,
        per_page: u32,
    ) -> Result<UnifiedSearchResult> {
        debug!("Searching skills in both platforms: {}", query);

        let (clawhub_result, skillhub_result) = tokio::join!(
            self.clawhub.search_skills(query, page, per_page),
            self.skillhub.search_skills(query, page, per_page)
        );

        let mut unified_skills = Vec::new();

        if let Ok(result) = clawhub_result {
            for skill in result.skills {
                unified_skills.push(UnifiedSkillInfo {
                    id: skill.id.clone(),
                    name: skill.name.clone(),
                    version: skill.version.clone(),
                    description: skill.description.clone(),
                    author: skill.author.clone(),
                    downloads: skill.downloads,
                    stars: Some(skill.stars),
                    rating: skill.average_rating,
                    rating_count: skill.rating_count,
                    tags: skill.tags.clone(),
                    categories: skill.categories.clone(),
                    source: SkillSource::ClawHub,
                    created_at: skill.created_at.clone(),
                    updated_at: skill.updated_at.clone(),
                    deprecated: skill.deprecated,
                });
            }
        }

        if let Ok(result) = skillhub_result {
            for skill in result.skills {
                unified_skills.push(UnifiedSkillInfo {
                    id: skill.id.clone(),
                    name: skill.name.clone(),
                    version: skill.version.clone(),
                    description: skill.description.unwrap_or_default(),
                    author: skill.author_id.clone(),
                    downloads: skill.download_count as u64,
                    stars: None,
                    rating: Some(skill.average_rating),
                    rating_count: Some(skill.rating_count as u32),
                    tags: skill.tags.unwrap_or_default(),
                    categories: skill.category.map(|c| vec![c]).unwrap_or_default(),
                    source: SkillSource::AetherisSkillHub,
                    created_at: skill.created_at.clone(),
                    updated_at: skill.updated_at.clone(),
                    deprecated: skill.deprecated_at.is_some(),
                });
            }
        }

        unified_skills.sort_by(|a, b| {
            let a_score = a.rating.unwrap_or(0.0) * (a.rating_count.unwrap_or(0) as f64)
                + (a.downloads as f64).ln();
            let b_score = b.rating.unwrap_or(0.0) * (b.rating_count.unwrap_or(0) as f64)
                + (b.downloads as f64).ln();
            b_score
                .partial_cmp(&a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(UnifiedSearchResult {
            total: unified_skills.len() as u64,
            skills: unified_skills,
            page,
            per_page,
        })
    }

    pub async fn install_skill(
        &self,
        registry: &mut AgentSkillsRegistry,
        skill_id: &str,
        source: &SkillSource,
        version: Option<&str>,
    ) -> Result<AgentSkillManifest> {
        match source {
            SkillSource::ClawHub => {
                self.clawhub
                    .install_skill(registry, skill_id, version)
                    .await
            }
            SkillSource::AetherisSkillHub => {
                self.skillhub
                    .install_skill(registry, skill_id, version)
                    .await
            }
        }
    }

    pub async fn check_all_updates(
        &self,
        registry: &AgentSkillsRegistry,
    ) -> Result<Vec<SkillUpdateCheck>> {
        let mut updates = Vec::new();
        let skills = registry.list();

        for skill in skills {
            let clawhub_check = self
                .clawhub
                .check_for_update(&skill.metadata.id, &skill.metadata.version)
                .await;
            let skillhub_check = self
                .skillhub
                .check_for_update(&skill.metadata.id, &skill.metadata.version)
                .await;

            if let Ok(check) = clawhub_check {
                if check.has_update {
                    updates.push(check);
                }
            }

            if let Ok(check) = skillhub_check {
                if check.has_update {
                    updates.push(check);
                }
            }
        }

        Ok(updates)
    }

    pub fn clawhub(&self) -> &ClawHubClient {
        &self.clawhub
    }

    pub fn skillhub(&self) -> &AetherisSkillHubClient {
        &self.skillhub
    }
}

#[derive(Debug, Clone)]
pub struct ClawHubSync {
    client: ClawHubClient,
    registry: AgentSkillsRegistry,
    auto_update: bool,
    update_interval_hours: u64,
}

impl ClawHubSync {
    pub fn new(client: ClawHubClient, registry: AgentSkillsRegistry) -> Self {
        Self {
            client,
            registry,
            auto_update: true,
            update_interval_hours: 24,
        }
    }

    pub fn with_auto_update(mut self, enabled: bool) -> Self {
        self.auto_update = enabled;
        self
    }

    pub fn with_update_interval(mut self, hours: u64) -> Self {
        self.update_interval_hours = hours;
        self
    }

    pub async fn sync_all_updates(&mut self) -> Result<Vec<String>> {
        let mut updated_skills = Vec::new();
        let skill_ids: Vec<String> = self
            .registry
            .list()
            .iter()
            .map(|skill| skill.metadata.id.clone())
            .collect();

        for skill_id in skill_ids {
            match self
                .client
                .update_skill(&mut self.registry, &skill_id)
                .await
            {
                Ok(_) => updated_skills.push(skill_id),
                Err(e) => {
                    tracing::warn!("Failed to update skill {}: {}", skill_id, e);
                }
            }
        }

        Ok(updated_skills)
    }

    pub fn registry(&self) -> &AgentSkillsRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut AgentSkillsRegistry {
        &mut self.registry
    }

    pub fn client(&self) -> &ClawHubClient {
        &self.client
    }
}

pub type ClawHubImporter = ClawHubClient;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_clawhub_client_creation() {
        let dir = tempdir().unwrap();
        let cache_dir = dir.path().join("clawhub");

        let client = ClawHubClient::new("https://example.com".to_string(), cache_dir);

        assert!(client.is_ok());
    }

    #[test]
    fn test_skillhub_client_creation() {
        let dir = tempdir().unwrap();
        let cache_dir = dir.path().join("skillhub");

        let client = AetherisSkillHubClient::new("https://example.com".to_string(), cache_dir);

        assert!(client.is_ok());
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 10000);
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_skill_source_enum() {
        assert_eq!(SkillSource::ClawHub, SkillSource::ClawHub);
        assert_eq!(SkillSource::AetherisSkillHub, SkillSource::AetherisSkillHub);
        assert_ne!(SkillSource::ClawHub, SkillSource::AetherisSkillHub);
    }
}
