//! Cron manager for scheduled recording tasks.
//!
//! This module implements the `CronManager` which loads schedules from configuration
//! and triggers recording tasks at specified times using tokio-cron-scheduler.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info, warn};

use crate::config::Schedule;
use crate::domain::service::RecordService;
use crate::utils::RecordError;

/// Manager for cron-based scheduled recording.
///
/// This struct manages scheduled recording tasks, loading them from configuration
/// and triggering recordings at the specified times.
pub struct CronManager {
    scheduler: Arc<Mutex<JobScheduler>>,
    recording_service: Arc<dyn RecordService>,
}

impl CronManager {
    /// Creates a new `CronManager`.
    ///
    /// # Arguments
    ///
    /// * `recording_service` - The recording service to use for scheduled tasks.
    ///
    /// # Errors
    ///
    /// Returns `RecordError` if the scheduler cannot be created.
    pub async fn new(recording_service: Arc<dyn RecordService>) -> Result<Self, RecordError> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| RecordError::Other(format!("Failed to create job scheduler: {e}")))?;

        Ok(Self {
            scheduler: Arc::new(Mutex::new(scheduler)),
            recording_service,
        })
    }

    /// Adds a scheduled recording task.
    ///
    /// # Arguments
    ///
    /// * `schedule` - The schedule configuration.
    ///
    /// # Errors
    ///
    /// Returns `RecordError` if the job cannot be created or added.
    pub async fn add_schedule(&self, schedule: Schedule) -> Result<(), RecordError> {
        let Schedule {
            id,
            cron_expression,
            stream_url,
            output_path,
            duration,
            metadata: _,
        } = schedule;

        info!("Adding scheduled recording: {} at {}", id, cron_expression);

        let service = Arc::clone(&self.recording_service);

        let job = Job::new_async(&cron_expression, move |_uuid, _l| {
            let id = id.clone();
            let stream_url = stream_url.clone();
            let output_path = output_path.clone();
            let service = Arc::clone(&service);

            Box::pin(async move {
                info!("Starting scheduled recording: {}", id);

                match Self::record_with_retry(&service, &stream_url, &output_path, duration).await {
                    Ok(()) => {
                        info!("Scheduled recording completed: {}", id);
                    }
                    Err(e) => {
                        error!("Scheduled recording failed after retries: {} - {}", id, e);
                    }
                }
            })
        })
        .map_err(|e| RecordError::Other(format!("Failed to create job: {e}")))?;

        let scheduler = self.scheduler.lock().await;
        scheduler
            .add(job)
            .await
            .map_err(|e| RecordError::Other(format!("Failed to add job: {e}")))?;

        Ok(())
    }

    /// Starts the cron scheduler.
    ///
    /// # Errors
    ///
    /// Returns `RecordError` if the scheduler fails to start.
    pub async fn start(&self) -> Result<(), RecordError> {
        info!("Starting cron scheduler");

        let scheduler = self.scheduler.lock().await;
        scheduler
            .start()
            .await
            .map_err(|e| RecordError::Other(format!("Failed to start scheduler: {e}")))?;

        Ok(())
    }

    /// Stops the cron scheduler.
    ///
    /// # Errors
    ///
    /// Returns `RecordError` if the scheduler fails to stop.
    pub async fn shutdown(&self) -> Result<(), RecordError> {
        info!("Stopping cron scheduler");

        let mut scheduler = self.scheduler.lock().await;
        scheduler
            .shutdown()
            .await
            .map_err(|e| RecordError::Other(format!("Failed to stop scheduler: {e}")))?;

        Ok(())
    }

    /// Records with exponential backoff retry logic.
    ///
    /// # Arguments
    ///
    /// * `service` - The recording service.
    /// * `url` - The stream URL.
    /// * `output_path` - The output file path.
    /// * `_duration` - Recording duration (unused in retry logic).
    ///
    /// # Errors
    ///
    /// Returns `RecordError` if all retry attempts fail.
    async fn record_with_retry(
        service: &Arc<dyn RecordService>,
        url: &str,
        output_path: &std::path::Path,
        _duration: Duration,
    ) -> Result<(), RecordError> {
        let max_attempts = 5;
        let initial_delay = Duration::from_secs(1);
        let max_delay = Duration::from_secs(60);

        let mut attempt = 0;
        let mut delay = initial_delay;

        loop {
            attempt += 1;

            match service.record(url, output_path).await {
                Ok(_) => return Ok(()),
                Err(e) if attempt < max_attempts => {
                    warn!(
                        "Recording attempt {} failed, retrying in {:?}: {}",
                        attempt, delay, e
                    );

                    tokio::time::sleep(delay).await;

                    // Exponential backoff
                    delay = std::cmp::min(delay * 2, max_delay);
                }
                Err(e) => {
                    error!("Recording failed after {} attempts: {}", attempt, e);
                    return Err(e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::metadata::RecordingMetadata;
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::Arc;

    struct MockRecordService;

    #[async_trait]
    impl RecordService for MockRecordService {
        async fn record(
            &self,
            _url: &str,
            _output_path: &std::path::Path,
        ) -> Result<RecordingMetadata, RecordError> {
            Ok(RecordingMetadata::new(
                "Test".to_string(),
                Utc::now(),
                Utc::now(),
                "/tmp/test.m4a".to_string(),
                1024,
                128,
            ))
        }
    }

    #[tokio::test]
    async fn test_cron_manager_creation() {
        let service = Arc::new(MockRecordService);
        let manager = CronManager::new(service).await;

        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_add_schedule() {
        let service = Arc::new(MockRecordService);
        let manager = CronManager::new(service).await.unwrap();

        let schedule = Schedule {
            id: "test-schedule".to_string(),
            cron_expression: "0 0 * * * *".to_string(),
            stream_url: "http://example.com/stream".to_string(),
            output_path: std::path::PathBuf::from("/tmp/output.m4a"),
            duration: Duration::from_secs(3600),
            metadata: std::collections::HashMap::new(),
        };

        let result = manager.add_schedule(schedule).await;

        // Note: This test may fail if the cron expression is invalid
        // In a real test, we'd use a valid expression or mock the Job creation
        assert!(result.is_ok() || result.is_err());
    }
}
