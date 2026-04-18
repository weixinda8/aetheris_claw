use crate::utils::{AetherisError, Result};
use base64::Engine;
use dashmap::DashMap;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RegistryType {
    DockerHub,
    GitHubContainerRegistry,
    GitLabContainerRegistry,
    Quay,
    Harbor,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RegistryAuthType {
    None,
    Basic,
    Bearer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryAuthConfig {
    pub auth_type: RegistryAuthType,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub id: String,
    pub name: String,
    pub registry_type: RegistryType,
    pub base_url: Option<String>,
    pub auth_config: Option<RegistryAuthConfig>,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciImageConfig {
    pub architecture: String,
    pub os: String,
    pub config: Option<serde_json::Value>,
    pub created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciManifestLayer {
    pub media_type: String,
    pub size: u64,
    pub digest: String,
    pub annotations: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciManifest {
    pub schema_version: u32,
    pub media_type: String,
    pub config: OciManifestLayer,
    pub layers: Vec<OciManifestLayer>,
    pub annotations: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciRepository {
    pub name: String,
    pub namespace: String,
    pub registry_id: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub is_public: bool,
    pub pull_count: u64,
    pub star_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciTag {
    pub name: String,
    pub digest: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub manifest_digest: String,
    pub size: u64,
    pub is_latest: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciArtifactMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub license: Option<String>,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciLayerInfo {
    pub digest: String,
    pub media_type: String,
    pub size: u64,
    pub compressed_size: Option<u64>,
    pub annotations: Option<HashMap<String, String>>,
    pub is_remote: bool,
    pub is_cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullProgress {
    pub artifact_name: String,
    pub total_layers: u32,
    pub downloaded_layers: u32,
    pub total_size_bytes: u64,
    pub downloaded_bytes: u64,
    pub current_layer: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub status: PullStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PullStatus {
    Queued,
    Downloading,
    Extracting,
    Verifying,
    Complete,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushProgress {
    pub artifact_name: String,
    pub total_layers: u32,
    pub uploaded_layers: u32,
    pub total_size_bytes: u64,
    pub uploaded_bytes: u64,
    pub current_layer: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub status: PushStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PushStatus {
    Queued,
    Preparing,
    Uploading,
    PushingManifest,
    Complete,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciCacheEntry {
    pub digest: String,
    pub media_type: String,
    pub size: u64,
    pub path: PathBuf,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub access_count: u64,
    pub is_manifest: bool,
}

pub struct EnhancedOciRegistry {
    registries: Arc<DashMap<String, RegistryConfig>>,
    repositories: Arc<DashMap<String, OciRepository>>,
    tags: Arc<DashMap<String, Vec<OciTag>>>,
    pull_progress: Arc<DashMap<String, PullProgress>>,
    push_progress: Arc<DashMap<String, PushProgress>>,
    cache: Arc<DashMap<String, OciCacheEntry>>,
    cache_path: PathBuf,
    http_client: reqwest::Client,
}

impl EnhancedOciRegistry {
    pub fn new(cache_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_path)?;

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(10))
            .build()?;

        let registry = Self {
            registries: Arc::new(DashMap::new()),
            repositories: Arc::new(DashMap::new()),
            tags: Arc::new(DashMap::new()),
            pull_progress: Arc::new(DashMap::new()),
            push_progress: Arc::new(DashMap::new()),
            cache: Arc::new(DashMap::new()),
            cache_path,
            http_client,
        };

        registry.load()?;
        Ok(registry)
    }

    fn save(&self) -> Result<()> {
        let registries_path = self.cache_path.join("registries.json");
        let registries: Vec<_> = self.registries.iter().map(|e| e.value().clone()).collect();
        std::fs::write(&registries_path, serde_json::to_string_pretty(&registries)?)?;

        let repositories_path = self.cache_path.join("repositories.json");
        let repositories: Vec<_> = self
            .repositories
            .iter()
            .map(|e| e.value().clone())
            .collect();
        std::fs::write(
            &repositories_path,
            serde_json::to_string_pretty(&repositories)?,
        )?;

        let tags_path = self.cache_path.join("tags.json");
        let tags_map: Vec<(String, Vec<OciTag>)> = self
            .tags
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect();
        std::fs::write(&tags_path, serde_json::to_string_pretty(&tags_map)?)?;

        Ok(())
    }

    fn load(&self) -> Result<()> {
        let registries_path = self.cache_path.join("registries.json");
        if registries_path.exists() {
            let content = std::fs::read_to_string(&registries_path)?;
            let registries: Vec<RegistryConfig> = serde_json::from_str(&content)?;
            for registry in registries {
                self.registries.insert(registry.id.clone(), registry);
            }
        }

        let repositories_path = self.cache_path.join("repositories.json");
        if repositories_path.exists() {
            let content = std::fs::read_to_string(&repositories_path)?;
            let repositories: Vec<OciRepository> = serde_json::from_str(&content)?;
            for repo in repositories {
                let repo_key = format!("{}:{}", repo.registry_id, repo.name);
                self.repositories.insert(repo_key, repo);
            }
        }

        let tags_path = self.cache_path.join("tags.json");
        if tags_path.exists() {
            let content = std::fs::read_to_string(&tags_path)?;
            let tags_map: Vec<(String, Vec<OciTag>)> = serde_json::from_str(&content)?;
            for (repo_key, tags) in tags_map {
                self.tags.insert(repo_key, tags);
            }
        }

        Ok(())
    }

    pub fn add_registry(&self, config: RegistryConfig) -> Result<()> {
        if self.registries.contains_key(&config.id) {
            return Err(AetherisError::Validation(format!(
                "Registry with ID '{}' already exists",
                config.id
            )));
        }

        info!("Adding OCI registry: {} ({})", config.name, config.id);
        self.registries.insert(config.id.clone(), config);
        self.save()?;

        Ok(())
    }

    pub fn remove_registry(&self, registry_id: &str) -> Result<()> {
        if self.registries.remove(registry_id).is_none() {
            return Err(AetherisError::NotFound(format!(
                "Registry not found: {}",
                registry_id
            )));
        }

        info!("Removed OCI registry: {}", registry_id);
        self.save()?;
        Ok(())
    }

    pub fn get_registry(&self, registry_id: &str) -> Option<RegistryConfig> {
        self.registries.get(registry_id).map(|r| r.value().clone())
    }

    pub fn list_registries(&self, include_disabled: bool) -> Vec<RegistryConfig> {
        self.registries
            .iter()
            .filter(|entry| include_disabled || entry.value().enabled)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub async fn fetch_manifest(
        &self,
        registry_id: &str,
        repository: &str,
        reference: &str,
    ) -> Result<OciManifest> {
        let registry = self.registries.get(registry_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Registry not found: {}", registry_id))
        })?;

        let cache_key = format!("{}:{}:{}:manifest", registry_id, repository, reference);
        if let Some(cached) = self.cache.get(&cache_key) {
            if cached.is_manifest {
                let content = std::fs::read_to_string(&cached.path)?;
                let manifest: OciManifest = serde_json::from_str(&content)?;
                return Ok(manifest);
            }
        }

        let base_url = registry.base_url.as_ref().ok_or_else(|| {
            AetherisError::Validation("Registry base URL not configured".to_string())
        })?;

        let url = format!("{}/v2/{}/manifests/{}", base_url, repository, reference);
        info!("Fetching manifest from: {}", url);

        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.oci.image.manifest.v1+json"),
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.docker.distribution.manifest.v2+json"),
        );

        self.add_auth_headers(&mut headers, &registry)?;

        let response = self.http_client.get(&url).headers(headers).send().await?;

        if !response.status().is_success() {
            return Err(AetherisError::External(format!(
                "Failed to fetch manifest: HTTP {}",
                response.status()
            )));
        }

        let manifest: OciManifest = response.json().await?;

        let manifest_path = self.cache_path.join(format!("{}.json", reference));
        std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

        let cache_entry = OciCacheEntry {
            digest: reference.to_string(),
            media_type: "application/vnd.oci.image.manifest.v1+json".to_string(),
            size: serde_json::to_vec(&manifest)?.len() as u64,
            path: manifest_path,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 1,
            is_manifest: true,
        };

        self.cache.insert(cache_key, cache_entry);

        Ok(manifest)
    }

    pub async fn pull_artifact(
        &self,
        registry_id: &str,
        repository: &str,
        reference: &str,
    ) -> Result<PullProgress> {
        let pull_id = uuid::Uuid::new_v4().to_string();
        let artifact_name = format!("{}/{}:{}", registry_id, repository, reference);

        let progress = PullProgress {
            artifact_name: artifact_name.clone(),
            total_layers: 0,
            downloaded_layers: 0,
            total_size_bytes: 0,
            downloaded_bytes: 0,
            current_layer: None,
            started_at: chrono::Utc::now(),
            status: PullStatus::Queued,
            error: None,
        };

        self.pull_progress.insert(pull_id.clone(), progress.clone());

        info!("Starting pull for: {}", artifact_name);

        let manifest = self
            .fetch_manifest(registry_id, repository, reference)
            .await?;

        let total_layers = manifest.layers.len() as u32 + 1;
        let total_size_bytes =
            manifest.layers.iter().map(|l| l.size).sum::<u64>() + manifest.config.size;

        if let Some(mut p) = self.pull_progress.get_mut(&pull_id) {
            p.total_layers = total_layers;
            p.total_size_bytes = total_size_bytes;
            p.status = PullStatus::Downloading;
        }

        for (i, layer) in manifest.layers.iter().enumerate() {
            let layer_progress = self
                .pull_layer(registry_id, repository, layer, &pull_id)
                .await;

            if let Err(err) = layer_progress {
                if let Some(mut p) = self.pull_progress.get_mut(&pull_id) {
                    p.status = PullStatus::Failed;
                    p.error = Some(err.to_string());
                }
                return Err(AetherisError::External(format!(
                    "Failed to pull layer {}: {:?}",
                    layer.digest, err
                )));
            }

            if let Some(mut p) = self.pull_progress.get_mut(&pull_id) {
                p.downloaded_layers = (i + 1) as u32;
                p.downloaded_bytes += layer.size;
            }
        }

        if let Some(mut p) = self.pull_progress.get_mut(&pull_id) {
            p.downloaded_layers = total_layers;
            p.downloaded_bytes = total_size_bytes;
            p.status = PullStatus::Complete;
        }

        info!("Successfully pulled artifact: {}", artifact_name);

        self.pull_progress
            .get(&pull_id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| AetherisError::Internal("Pull progress not found".to_string()))
    }

    async fn pull_layer(
        &self,
        registry_id: &str,
        repository: &str,
        layer: &OciManifestLayer,
        pull_id: &str,
    ) -> Result<()> {
        if let Some(mut p) = self.pull_progress.get_mut(pull_id) {
            p.current_layer = Some(layer.digest.clone());
        }

        let cache_key = format!("{}:{}:{}:layer", registry_id, repository, layer.digest);
        if self.cache.contains_key(&cache_key) {
            debug!("Layer already cached: {}", layer.digest);
            return Ok(());
        }

        let registry = self.registries.get(registry_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Registry not found: {}", registry_id))
        })?;

        let base_url = registry.base_url.as_ref().ok_or_else(|| {
            AetherisError::Validation("Registry base URL not configured".to_string())
        })?;

        let url = format!("{}/v2/{}/blobs/{}", base_url, repository, layer.digest);
        debug!("Pulling layer from: {}", url);

        let mut headers = HeaderMap::new();
        self.add_auth_headers(&mut headers, &registry)?;

        let response = self.http_client.get(&url).headers(headers).send().await?;

        if !response.status().is_success() {
            return Err(AetherisError::External(format!(
                "Failed to pull layer: HTTP {}",
                response.status()
            )));
        }

        let bytes = response.bytes().await?;
        let layer_path = self.cache_path.join(layer.digest.replace(':', "_"));
        std::fs::write(&layer_path, &bytes)?;

        let cache_entry = OciCacheEntry {
            digest: layer.digest.clone(),
            media_type: layer.media_type.clone(),
            size: layer.size,
            path: layer_path,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 1,
            is_manifest: false,
        };

        self.cache.insert(cache_key, cache_entry);

        Ok(())
    }

    pub async fn push_artifact(
        &self,
        registry_id: &str,
        repository: &str,
        tag: &str,
        manifest: OciManifest,
        layers: Vec<Vec<u8>>,
    ) -> Result<PushProgress> {
        let push_id = uuid::Uuid::new_v4().to_string();
        let artifact_name = format!("{}/{}:{}", registry_id, repository, tag);

        let progress = PushProgress {
            artifact_name: artifact_name.clone(),
            total_layers: (manifest.layers.len() + 1) as u32,
            uploaded_layers: 0,
            total_size_bytes: manifest.layers.iter().map(|l| l.size).sum::<u64>()
                + manifest.config.size,
            uploaded_bytes: 0,
            current_layer: None,
            started_at: chrono::Utc::now(),
            status: PushStatus::Queued,
            error: None,
        };

        self.push_progress.insert(push_id.clone(), progress.clone());

        info!("Starting push for: {}", artifact_name);

        if let Some(mut p) = self.push_progress.get_mut(&push_id) {
            p.status = PushStatus::Preparing;
        }

        for (i, layer) in manifest.layers.iter().enumerate() {
            if i < layers.len() {
                let push_result = self
                    .push_layer(registry_id, repository, layer, &layers[i], &push_id)
                    .await;

                if let Err(err) = push_result {
                    if let Some(mut p) = self.push_progress.get_mut(&push_id) {
                        p.status = PushStatus::Failed;
                        p.error = Some(err.to_string());
                    }
                    return Err(AetherisError::External(format!(
                        "Failed to push layer {}: {:?}",
                        layer.digest, err
                    )));
                }

                if let Some(mut p) = self.push_progress.get_mut(&push_id) {
                    p.uploaded_layers = (i + 1) as u32;
                    p.uploaded_bytes += layer.size;
                }
            }
        }

        if let Some(mut p) = self.push_progress.get_mut(&push_id) {
            p.status = PushStatus::PushingManifest;
        }

        self.push_manifest(registry_id, repository, tag, &manifest)
            .await?;

        if let Some(mut p) = self.push_progress.get_mut(&push_id) {
            p.uploaded_layers = (manifest.layers.len() + 1) as u32;
            p.uploaded_bytes = p.total_size_bytes;
            p.status = PushStatus::Complete;
        }

        info!("Successfully pushed artifact: {}", artifact_name);

        self.push_progress
            .get(&push_id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| AetherisError::Internal("Push progress not found".to_string()))
    }

    async fn push_layer(
        &self,
        registry_id: &str,
        repository: &str,
        layer: &OciManifestLayer,
        layer_data: &[u8],
        push_id: &str,
    ) -> Result<()> {
        if let Some(mut p) = self.push_progress.get_mut(push_id) {
            p.current_layer = Some(layer.digest.clone());
            p.status = PushStatus::Uploading;
        }

        let registry = self.registries.get(registry_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Registry not found: {}", registry_id))
        })?;

        let base_url = registry.base_url.as_ref().ok_or_else(|| {
            AetherisError::Validation("Registry base URL not configured".to_string())
        })?;

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_str(&layer.media_type)?);
        self.add_auth_headers(&mut headers, &registry)?;

        let url = format!("{}/v2/{}/blobs/uploads/", base_url, repository);
        let upload_response = self
            .http_client
            .post(&url)
            .headers(headers.clone())
            .send()
            .await?;

        if !upload_response.status().is_success() {
            return Err(AetherisError::External(format!(
                "Failed to start upload: HTTP {}",
                upload_response.status()
            )));
        }

        let upload_url = upload_response
            .headers()
            .get("Location")
            .ok_or_else(|| AetherisError::External("No Location header in response".to_string()))?;

        let final_url = format!(
            "{}&digest={}",
            upload_url
                .to_str()
                .map_err(|e| AetherisError::External(e.to_string()))?,
            layer.digest
        );
        let put_response = self
            .http_client
            .put(&final_url)
            .headers(headers)
            .body(layer_data.to_vec())
            .send()
            .await?;

        if !put_response.status().is_success() {
            return Err(AetherisError::External(format!(
                "Failed to upload layer: HTTP {}",
                put_response.status()
            )));
        }

        Ok(())
    }

    async fn push_manifest(
        &self,
        registry_id: &str,
        repository: &str,
        tag: &str,
        manifest: &OciManifest,
    ) -> Result<()> {
        let registry = self.registries.get(registry_id).ok_or_else(|| {
            AetherisError::NotFound(format!("Registry not found: {}", registry_id))
        })?;

        let base_url = registry.base_url.as_ref().ok_or_else(|| {
            AetherisError::Validation("Registry base URL not configured".to_string())
        })?;

        let url = format!("{}/v2/{}/manifests/{}", base_url, repository, tag);
        debug!("Pushing manifest to: {}", url);

        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/vnd.oci.image.manifest.v1+json"),
        );
        self.add_auth_headers(&mut headers, &registry)?;

        let response = self
            .http_client
            .put(&url)
            .headers(headers)
            .json(manifest)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AetherisError::External(format!(
                "Failed to push manifest: HTTP {}",
                response.status()
            )));
        }

        Ok(())
    }

    fn add_auth_headers(&self, headers: &mut HeaderMap, registry: &RegistryConfig) -> Result<()> {
        if let Some(auth) = &registry.auth_config {
            match auth.auth_type {
                RegistryAuthType::Basic => {
                    if let (Some(username), Some(password)) = (&auth.username, &auth.password) {
                        let credentials = base64::engine::general_purpose::STANDARD
                            .encode(format!("{}:{}", username, password));
                        headers.insert(
                            AUTHORIZATION,
                            HeaderValue::from_str(&format!("Basic {}", credentials))?,
                        );
                    }
                }
                RegistryAuthType::Bearer => {
                    if let Some(token) = &auth.token {
                        headers.insert(
                            AUTHORIZATION,
                            HeaderValue::from_str(&format!("Bearer {}", token))?,
                        );
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn get_pull_progress(&self, pull_id: &str) -> Option<PullProgress> {
        self.pull_progress.get(pull_id).map(|p| p.value().clone())
    }

    pub fn get_push_progress(&self, push_id: &str) -> Option<PushProgress> {
        self.push_progress.get(push_id).map(|p| p.value().clone())
    }

    pub fn list_cached_artifacts(&self) -> Vec<OciCacheEntry> {
        self.cache.iter().map(|e| e.value().clone()).collect()
    }

    pub fn clear_cache(&self, older_than: Option<Duration>) -> Result<usize> {
        let now = chrono::Utc::now();
        let mut removed = 0;

        let keys_to_remove: Vec<String> = self
            .cache
            .iter()
            .filter(|entry| {
                if let Some(duration) = older_than {
                    if let Ok(chrono_duration) = chrono::Duration::from_std(duration) {
                        (now - entry.value().created_at) > chrono_duration
                    } else {
                        false
                    }
                } else {
                    true
                }
            })
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys_to_remove {
            if let Some((_, entry)) = self.cache.remove(&key) {
                if entry.path.exists() {
                    let _ = std::fs::remove_file(&entry.path);
                }
                removed += 1;
            }
        }

        info!("Cleared {} cached artifacts", removed);

        Ok(removed)
    }

    pub fn registry_count(&self) -> usize {
        self.registries.len()
    }

    pub fn cache_size(&self) -> u64 {
        self.cache.iter().map(|e| e.value().size).sum()
    }
}

impl Default for EnhancedOciRegistry {
    fn default() -> Self {
        let cache_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".aetheris")
            .join("oci-cache");

        Self::new(cache_path).unwrap_or_else(|_| {
            let temp_dir = tempfile::tempdir().unwrap();
            Self::new(temp_dir.path().to_path_buf()).unwrap()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_oci_registry_new() {
        let temp_dir = tempfile::tempdir().unwrap();
        let registry = EnhancedOciRegistry::new(temp_dir.path().to_path_buf());
        assert!(registry.is_ok());
    }
}
