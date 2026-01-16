//! Configuration repository trait and implementations.
//!
//! This module defines the repository pattern for configuration access,
//! allowing for testable and swappable configuration sources.

use async_trait::async_trait;
use std::path::Path;

use crate::config::Config;
use crate::utils::RecordError;

/// Repository trait for configuration access.
///
/// This trait abstracts configuration loading, enabling testable implementations
/// and separation of business logic from data access.
#[async_trait]
pub trait ConfigRepository: Send + Sync {
    /// Loads configuration from the default location.
    ///
    /// # Errors
    ///
    /// Returns `RecordError` if configuration file cannot be read or parsed.
    async fn load(&self) -> Result<Config, RecordError>;

    /// Loads configuration from a specific path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file.
    ///
    /// # Errors
    ///
    /// Returns `RecordError` if configuration file cannot be read or parsed.
    async fn load_from_path(&self, path: &Path) -> Result<Config, RecordError>;
}

/// File-based configuration repository implementation.
pub struct FileConfigRepository;

#[async_trait]
impl ConfigRepository for FileConfigRepository {
    async fn load(&self) -> Result<Config, RecordError> {
        // Try to load from config.toml, fallback to environment variables
        let config_path = Path::new("config.toml");

        if config_path.exists() {
            self.load_from_path(config_path).await
        } else {
            // Fallback to default configuration with environment variables
            Ok(Config::default())
        }
    }

    async fn load_from_path(&self, path: &Path) -> Result<Config, RecordError> {
        use tokio::fs;

        let content = fs::read_to_string(path)
            .await
            .map_err(|e| RecordError::Other(format!("Failed to read config file: {}", e)))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| RecordError::Other(format!("Failed to parse config TOML: {}", e)))?;

        Ok(config)
    }
}

/// Mock configuration repository for testing.
#[cfg(test)]
pub struct MockConfigRepository {
    pub config: Result<Config, RecordError>,
}

#[cfg(test)]
#[async_trait]
impl ConfigRepository for MockConfigRepository {
    async fn load(&self) -> Result<Config, RecordError> {
        self.config.clone()
    }

    async fn load_from_path(&self, _path: &Path) -> Result<Config, RecordError> {
        self.config.clone()
    }
}
