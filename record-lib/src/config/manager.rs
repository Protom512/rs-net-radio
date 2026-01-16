//! Configuration structures for the recording system.
//!
//! This module defines the configuration data structures used throughout
//! the application for batch and streaming recording settings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Main configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Batch recording configuration.
    #[serde(default)]
    pub batch: BatchConfig,

    /// Streaming recording configuration.
    #[serde(default)]
    pub streaming: StreamingConfig,

    /// Logging configuration.
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// Batch recording configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum number of parallel recording jobs.
    #[serde(default = "default_max_parallel_jobs")]
    pub max_parallel_jobs: usize,

    /// Number of retry attempts for failed recordings.
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,

    /// Connection timeout in seconds.
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_parallel_jobs: default_max_parallel_jobs(),
            retry_count: default_retry_count(),
            timeout_seconds: default_timeout_seconds(),
        }
    }
}

fn default_max_parallel_jobs() -> usize {
    3
}

fn default_retry_count() -> u32 {
    3
}

fn default_timeout_seconds() -> u64 {
    30
}

/// Streaming recording configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Size of data chunks in bytes.
    #[serde(default = "default_chunk_size_bytes")]
    pub chunk_size_bytes: usize,

    /// Maximum memory usage in MB.
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: usize,

    /// Initial reconnection delay in seconds.
    #[serde(default = "default_reconnect_initial_delay_secs")]
    pub reconnect_initial_delay_secs: u64,

    /// Maximum reconnection delay in seconds.
    #[serde(default = "default_reconnect_max_delay_secs")]
    pub reconnect_max_delay_secs: u64,

    /// Maximum number of reconnection attempts.
    #[serde(default = "default_max_reconnect_attempts")]
    pub max_reconnect_attempts: u32,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            chunk_size_bytes: default_chunk_size_bytes(),
            max_memory_mb: default_max_memory_mb(),
            reconnect_initial_delay_secs: default_reconnect_initial_delay_secs(),
            reconnect_max_delay_secs: default_reconnect_max_delay_secs(),
            max_reconnect_attempts: default_max_reconnect_attempts(),
        }
    }
}

fn default_chunk_size_bytes() -> usize {
    4096 // 4KB
}

fn default_max_memory_mb() -> usize {
    512
}

fn default_reconnect_initial_delay_secs() -> u64 {
    1
}

fn default_reconnect_max_delay_secs() -> u64 {
    60
}

fn default_max_reconnect_attempts() -> u32 {
    5
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log file path.
    #[serde(default = "default_log_file")]
    pub log_file: PathBuf,

    /// Whether to log to stdout.
    #[serde(default = "default_log_to_stdout")]
    pub log_to_stdout: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_file: default_log_file(),
            log_to_stdout: default_log_to_stdout(),
        }
    }
}

fn default_log_file() -> PathBuf {
    PathBuf::from("error.log")
}

fn default_log_to_stdout() -> bool {
    true
}

/// Cron configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CronConfig {
    /// Scheduled recording tasks.
    #[serde(default)]
    pub schedules: Vec<Schedule>,
}

/// A single scheduled recording task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    /// Unique identifier for this schedule.
    pub id: String,

    /// Cron expression for scheduling.
    pub cron_expression: String,

    /// Stream URL to record.
    pub stream_url: String,

    /// Output file path.
    pub output_path: PathBuf,

    /// Recording duration.
    #[serde(with = "serde_duration")]
    pub duration: Duration,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Module for serde Duration serialization/deserialization.
mod serde_duration {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();

        assert_eq!(config.batch.max_parallel_jobs, 3);
        assert_eq!(config.batch.retry_count, 3);
        assert_eq!(config.batch.timeout_seconds, 30);

        assert_eq!(config.streaming.chunk_size_bytes, 4096);
        assert_eq!(config.streaming.max_memory_mb, 512);
        assert_eq!(config.streaming.reconnect_initial_delay_secs, 1);
        assert_eq!(config.streaming.reconnect_max_delay_secs, 60);
        assert_eq!(config.streaming.max_reconnect_attempts, 5);
    }

    #[test]
    fn test_schedule_serialization() {
        let schedule = Schedule {
            id: "test-schedule".to_string(),
            cron_expression: "0 0 * * * *".to_string(),
            stream_url: "http://example.com/stream".to_string(),
            output_path: PathBuf::from("/tmp/output.m4a"),
            duration: Duration::from_secs(3600),
            metadata: HashMap::new(),
        };

        let toml_str = toml::to_string_pretty(&schedule).unwrap();
        let deserialized: Schedule = toml::from_str(&toml_str).unwrap();

        assert_eq!(deserialized.id, schedule.id);
        assert_eq!(deserialized.cron_expression, schedule.cron_expression);
        assert_eq!(deserialized.stream_url, schedule.stream_url);
        assert_eq!(deserialized.output_path, schedule.output_path);
        assert_eq!(deserialized.duration, schedule.duration);
    }
}
