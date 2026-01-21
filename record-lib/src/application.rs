//! Application facade for high-level operations.
//!
//! This module provides a simplified interface for the application layer,
//! reducing coupling between the main entry point and internal modules.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use crate::batch::BatchRecorder;
use crate::config::{ConfigRepository, FileConfigRepository};
use crate::domain::service::Program;
use crate::domain::service::RecordService;
use crate::scheduler::CronManager;

/// Application facade that provides a simplified interface
/// for the main entry point.
///
/// This facade reduces coupling by hiding internal module complexity
/// behind a single, well-defined interface.
pub struct AppFacade {
    config: AppConfig,
}

/// Application configuration loaded from file.
#[derive(Clone)]
pub struct AppConfig {
    pub batch: BatchConfig,
}

/// Batch recording configuration.
#[derive(Clone)]
pub struct BatchConfig {
    pub max_parallel_jobs: usize,
    pub retry_count: u32,
    pub timeout_seconds: u64,
}

impl AppFacade {
    /// Creates a new application facade by loading configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration cannot be loaded.
    pub async fn new() -> Result<Self> {
        let config_repo = FileConfigRepository;
        let config = config_repo
            .load()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;

        Ok(Self {
            config: AppConfig {
                batch: BatchConfig {
                    max_parallel_jobs: config.batch.max_parallel_jobs,
                    retry_count: config.batch.retry_count,
                    timeout_seconds: config.batch.timeout_seconds,
                },
            },
        })
    }

    /// Runs batch recording from a program list file.
    ///
    /// # Arguments
    ///
    /// * `input_path` - Path to the program list file.
    /// * `service` - Recording service implementation.
    ///
    /// # Errors
    ///
    /// Returns an error if batch recording fails.
    pub async fn run_batch(
        &self,
        input_path: PathBuf,
        service: Arc<dyn RecordService>,
    ) -> Result<BatchSummary> {
        info!("Starting batch recording from: {}", input_path.display());

        let programs = Self::parse_program_list(&input_path)?;

        let recorder = BatchRecorder::new(
            self.config.batch.max_parallel_jobs,
            self.config.batch.retry_count,
            Duration::from_secs(self.config.batch.timeout_seconds),
        );

        let internal_summary = recorder
            .record_batch(programs, service)
            .await
            .map_err(|e| anyhow::anyhow!("Batch recording failed: {}", e))?;

        Ok(BatchSummary::from_internal(internal_summary))
    }

    /// Creates a cron manager for scheduled recording.
    ///
    /// # Arguments
    ///
    /// * `service` - Recording service implementation.
    ///
    /// # Errors
    ///
    /// Returns an error if cron manager creation fails.
    pub async fn create_cron_manager(
        &self,
        service: Arc<dyn RecordService>,
    ) -> Result<CronManager> {
        CronManager::new(service)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create cron manager: {}", e))
    }

    /// Parses a program list file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the program list file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    fn parse_program_list(path: &PathBuf) -> Result<Vec<Program>> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read program list: {}", e))?;

        let mut programs = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() != 3 {
                tracing::error!("Invalid format on line {}: {}", line_num + 1, line);
                continue;
            }

            programs.push(Program {
                title: parts[0].into(),
                url: parts[1].into(),
                output_path: parts[2].into(),
            });
        }

        Ok(programs)
    }
}

impl Default for AppFacade {
    fn default() -> Self {
        Self {
            config: AppConfig {
                batch: BatchConfig {
                    max_parallel_jobs: 3,
                    retry_count: 3,
                    timeout_seconds: 300,
                },
            },
        }
    }
}

/// Summary of batch recording operation.
#[derive(Debug)]
pub struct BatchSummary {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
}

impl BatchSummary {
    fn from_internal(summary: crate::batch::BatchSummary) -> Self {
        Self {
            total: summary.total_count,
            successful: summary.success_count,
            failed: summary.failure_count,
        }
    }

    /// Prints the summary to stdout.
    pub fn print_summary(&self) {
        println!("\n=== Batch Recording Summary ===");
        println!("Total: {}", self.total);
        println!("Successful: {}", self.successful);
        println!("Failed: {}", self.failed);

        if self.failed > 0 {
            println!("\nSome recordings failed. Check logs for details.");
        } else {
            println!("\nAll recordings completed successfully!");
        }
    }
}
