use crate::agent::config::config::{AgentConfig, AgentConfigError};
use crate::agent::config::loader::AgentConfigLoader;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct WatchedConfig {
    pub path: PathBuf,
    pub current_hash: String,
    pub config: Option<AgentConfig>,
    pub last_loaded: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub enum ConfigChangeEvent {
    Added {
        path: PathBuf,
        config: Box<AgentConfig>,
    },
    Modified {
        path: PathBuf,
        old_config: Option<Box<AgentConfig>>,
        new_config: Box<AgentConfig>,
    },
    Removed {
        path: PathBuf,
        old_config: Option<Box<AgentConfig>>,
    },
}

pub type ConfigChangeCallback = Arc<dyn Fn(ConfigChangeEvent) + Send + Sync>;

pub struct HotReloadManager {
    loader: AgentConfigLoader,
    watched_configs: RwLock<HashMap<PathBuf, WatchedConfig>>,
    callbacks: RwLock<Vec<ConfigChangeCallback>>,
    poll_interval: Duration,
    is_running: RwLock<bool>,
    poll_handle: RwLock<Option<JoinHandle<()>>>,
}

impl HotReloadManager {
    pub fn new() -> Self {
        Self {
            loader: AgentConfigLoader::new(),
            watched_configs: RwLock::new(HashMap::new()),
            callbacks: RwLock::new(Vec::new()),
            poll_interval: Duration::from_secs(5),
            is_running: RwLock::new(false),
            poll_handle: RwLock::new(None),
        }
    }

    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    pub fn with_loader(mut self, loader: AgentConfigLoader) -> Self {
        self.loader = loader;
        self
    }

    pub async fn watch_config<P: Into<PathBuf>>(&self, path: P) -> Result<(), AgentConfigError> {
        let path = path.into();
        let hash = AgentConfigLoader::compute_file_hash(&path)?;

        let config = if path.extension().and_then(|e| e.to_str()) == Some("yaml")
            || path.extension().and_then(|e| e.to_str()) == Some("yml")
        {
            Some(self.loader.load_from_yaml_file(&path)?)
        } else if path.extension().and_then(|e| e.to_str()) == Some("json5") {
            Some(self.loader.load_from_json5_file(&path)?)
        } else {
            None
        };

        let config_clone = config.clone();
        let watched = WatchedConfig {
            path: path.clone(),
            current_hash: hash,
            config,
            last_loaded: Some(chrono::Utc::now()),
        };

        self.watched_configs
            .write()
            .await
            .insert(path.clone(), watched);

        if let Some(cfg) = &config_clone {
            self.emit_event(ConfigChangeEvent::Added {
                path,
                config: Box::new(cfg.clone()),
            })
            .await;
        }

        Ok(())
    }

    pub async fn watch_directory<P: Into<PathBuf>>(&self, dir: P) -> Result<(), AgentConfigError> {
        let dir = dir.into();

        if !dir.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if ext_str == "yaml" || ext_str == "yml" || ext_str == "json5" {
                        let _ = self.watch_config(&path).await;
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn unwatch_config<P: Into<PathBuf>>(&self, path: P) {
        let path = path.into();
        let mut watched = self.watched_configs.write().await;

        if let Some(old) = watched.remove(&path) {
            self.emit_event(ConfigChangeEvent::Removed {
                path,
                old_config: old.config.map(Box::new),
            })
            .await;
        }
    }

    pub async fn get_config<P: Into<PathBuf>>(&self, path: P) -> Option<AgentConfig> {
        let path = path.into();
        let watched = self.watched_configs.read().await;
        watched.get(&path).and_then(|w| w.config.clone())
    }

    pub async fn list_watched(&self) -> Vec<WatchedConfig> {
        let watched = self.watched_configs.read().await;
        watched.values().cloned().collect()
    }

    pub async fn register_callback<F>(&self, callback: F)
    where
        F: Fn(ConfigChangeEvent) + Send + Sync + 'static,
    {
        self.callbacks.write().await.push(Arc::new(callback));
    }

    pub async fn start_polling(&self) {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return;
        }
        *is_running = true;
        drop(is_running);

        let manager = Arc::new(self.clone());
        let interval = self.poll_interval;

        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                let running = manager.is_running.read().await;
                if !*running {
                    break;
                }
                drop(running);

                if let Err(e) = manager.check_all_changes().await {
                    warn!("Error checking config changes: {}", e);
                }
            }
        });

        let mut poll_handle = self.poll_handle.write().await;
        *poll_handle = Some(handle);
    }

    pub async fn stop_polling(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        let mut poll_handle = self.poll_handle.write().await;
        if let Some(handle) = poll_handle.take() {
            handle.abort();
        }
    }

    async fn check_all_changes(&self) -> Result<(), AgentConfigError> {
        let paths: Vec<PathBuf> = {
            let watched = self.watched_configs.read().await;
            watched.keys().cloned().collect()
        };

        for path in paths {
            self.check_single_change(&path).await?;
        }

        Ok(())
    }

    async fn check_single_change(&self, path: &PathBuf) -> Result<(), AgentConfigError> {
        let watched_configs = self.watched_configs.read().await;
        let Some(watched) = watched_configs.get(path) else {
            return Ok(());
        };

        if !path.exists() {
            drop(watched_configs);
            let mut watched_configs_mut = self.watched_configs.write().await;
            if let Some(old) = watched_configs_mut.remove(path) {
                self.emit_event(ConfigChangeEvent::Removed {
                    path: path.clone(),
                    old_config: old.config.map(Box::new),
                })
                .await;
            }
            return Ok(());
        }

        let current_hash = AgentConfigLoader::compute_file_hash(path)?;

        if current_hash == watched.current_hash {
            return Ok(());
        }

        let old_config = watched.config.clone();
        drop(watched_configs);

        let new_config = if path.extension().and_then(|e| e.to_str()) == Some("yaml")
            || path.extension().and_then(|e| e.to_str()) == Some("yml")
        {
            Some(self.loader.load_from_yaml_file(path)?)
        } else if path.extension().and_then(|e| e.to_str()) == Some("json5") {
            Some(self.loader.load_from_json5_file(path)?)
        } else {
            None
        };

        let mut watched_configs_mut = self.watched_configs.write().await;

        if let Some(new_cfg) = &new_config {
            let updated = WatchedConfig {
                path: path.clone(),
                current_hash,
                config: Some(new_cfg.clone()),
                last_loaded: Some(chrono::Utc::now()),
            };

            watched_configs_mut.insert(path.clone(), updated);

            self.emit_event(ConfigChangeEvent::Modified {
                path: path.clone(),
                old_config: old_config.map(Box::new),
                new_config: Box::new(new_cfg.clone()),
            })
            .await;
        }

        Ok(())
    }

    async fn emit_event(&self, event: ConfigChangeEvent) {
        let callbacks = self.callbacks.read().await;
        for callback in callbacks.iter() {
            callback(event.clone());
        }
    }
}

impl Clone for HotReloadManager {
    fn clone(&self) -> Self {
        Self {
            loader: self.loader.clone(),
            watched_configs: RwLock::new(HashMap::new()),
            callbacks: RwLock::new(Vec::new()),
            poll_interval: self.poll_interval,
            is_running: RwLock::new(false),
            poll_handle: RwLock::new(None),
        }
    }
}

impl Default for HotReloadManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_hot_reload_manager_creation() {
        let manager = HotReloadManager::new();
        assert!(manager.list_watched().await.is_empty());
    }

    #[tokio::test]
    async fn test_watch_and_unwatch_config() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test-config.yaml");

        let config = crate::agent::config::config::AgentConfig::default();
        crate::agent::config::loader::AgentConfigLoader::save_to_yaml_file(&config, &file_path)
            .unwrap();

        let manager = HotReloadManager::new();

        manager.watch_config(&file_path).await.unwrap();

        let watched = manager.list_watched().await;
        assert_eq!(watched.len(), 1);

        let loaded_config = manager.get_config(&file_path).await;
        assert!(loaded_config.is_some());

        manager.unwatch_config(&file_path).await;

        let watched_after = manager.list_watched().await;
        assert!(watched_after.is_empty());

        let config_after = manager.get_config(&file_path).await;
        assert!(config_after.is_none());
    }

    #[tokio::test]
    async fn test_with_poll_interval() {
        let manager =
            HotReloadManager::new().with_poll_interval(std::time::Duration::from_secs(10));
        assert_eq!(manager.poll_interval, std::time::Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_register_callback() {
        let manager = HotReloadManager::new();
        let event_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = event_count.clone();

        manager
            .register_callback(move |_event| {
                count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            })
            .await;

        assert_eq!(manager.callbacks.read().await.len(), 1);
    }
}
