//! Batch recorder for parallel recording tasks.
//!
//! This module implements the BatchRecorder which manages parallel recording
//! of multiple programs with semaphore-based concurrency control.

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

use crate::domain::service::{Program, RecordService};
use crate::utils::RecordError;

/// Batch recorder for parallel recording tasks.
///
/// This struct manages the parallel recording of multiple programs,
/// controlling concurrency with a semaphore and tracking progress.
pub struct BatchRecorder {
    max_parallel_jobs: usize,
    retry_count: u32,
    timeout: std::time::Duration,
}

impl BatchRecorder {
    /// Creates a new BatchRecorder.
    ///
    /// # Arguments
    ///
    /// * `max_parallel_jobs` - Maximum number of parallel recordings.
    /// * `retry_count` - Number of retry attempts for failed recordings.
    /// * `timeout` - Timeout for each recording.
    #[must_use]
    pub const fn new(max_parallel_jobs: usize, retry_count: u32, timeout: std::time::Duration) -> Self {
        Self {
            max_parallel_jobs,
            retry_count,
            timeout,
        }
    }

    /// Records a batch of programs in parallel.
    ///
    /// # Arguments
    ///
    /// * `programs` - List of programs to record.
    /// * `service` - The recording service to use.
    ///
    /// # Errors
    ///
    /// Returns `BatchError` if the batch recording fails.
    ///
    /// # Panics
    ///
    /// Panics if `max_parallel_jobs` is 0.
    pub async fn record_batch(
        &self,
        programs: Vec<Program>,
        service: Arc<dyn RecordService>,
    ) -> Result<BatchSummary, RecordError> {
        if programs.is_empty() {
            return Ok(BatchSummary {
                total_count: 0,
                success_count: 0,
                failure_count: 0,
                failures: Vec::new(),
                duration: std::time::Duration::from_secs(0),
            });
        }

        assert!(self.max_parallel_jobs > 0, "max_parallel_jobs must be > 0");

        let start_time = Instant::now();
        let total = programs.len();
        let semaphore = Arc::new(Semaphore::new(self.max_parallel_jobs));
        let mut tasks = tokio::task::JoinSet::new();

        info!(
            "Starting batch recording: {} programs, max parallel: {}",
            total, self.max_parallel_jobs
        );

        for program in programs {
            let permit = Arc::clone(&semaphore);
            let service = Arc::clone(&service);
            let retry_count = self.retry_count;

            tasks.spawn(async move {
                let _permit = match permit.acquire().await {
                    Ok(p) => p,
                    Err(e) => return Err((program.title.clone(), format!("Failed to acquire semaphore: {}", e))),
                };

                Self::record_with_retries(&service, program, retry_count).await
            });
        }

        let mut successes = 0;
        let mut failures = Vec::new();

        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(_title)) => {
                    successes += 1;
                    Self::report_progress(successes, total);
                }
                Ok(Err((title, err))) => {
                    failures.push((title.clone(), err));
                    error!("Recording failed: {}", title);
                }
                Err(e) => {
                    error!("Task panicked: {}", e);
                }
            }
        }

        let duration = start_time.elapsed();

        info!(
            "Batch recording completed: {} succeeded, {} failed, duration: {:?}",
            successes,
            failures.len(),
            duration
        );

        Ok(BatchSummary {
            total_count: total,
            success_count: successes,
            failure_count: failures.len(),
            failures,
            duration,
        })
    }

    /// Records a single program with retry logic.
    ///
    /// # Arguments
    ///
    /// * `service` - The recording service.
    /// * `program` - The program to record.
    /// * `retry_count` - Number of retry attempts.
    ///
    /// # Errors
    ///
    /// Returns the program title and error if all retries fail.
    async fn record_with_retries(
        service: &Arc<dyn RecordService>,
        program: Program,
        retry_count: u32,
    ) -> Result<String, (String, String)> {
        let Program {
            title,
            url,
            output_path,
        } = program;

        let mut attempts = 0;

        loop {
            attempts += 1;

            match service.record(&url, &output_path).await {
                Ok(_) => {
                    info!("Recording successful: {}", title);
                    return Ok(title);
                }
                Err(e) if attempts <= retry_count => {
                    warn!(
                        "Recording attempt {} failed for {}: {}, retrying...",
                        attempts, title, e
                    );

                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
                Err(e) => {
                    error!(
                        "Recording failed after {} attempts for {}: {}",
                        attempts, title, e
                    );
                    return Err((title, e.to_string()));
                }
            }
        }
    }

    /// Reports recording progress.
    ///
    /// # Arguments
    ///
    /// * `completed` - Number of completed recordings.
    /// * `total` - Total number of recordings.
    fn report_progress(completed: usize, total: usize) {
        let progress = (completed as f64 / total as f64) * 100.0;
        info!("Progress: {}/{} ({:.1}%)", completed, total, progress);
    }
}

/// Summary of a batch recording operation.
#[derive(Debug, Clone)]
pub struct BatchSummary {
    /// Total number of programs.
    pub total_count: usize,

    /// Number of successful recordings.
    pub success_count: usize,

    /// Number of failed recordings.
    pub failure_count: usize,

    /// List of failures with program name and error message.
    pub failures: Vec<(String, String)>,

    /// Total duration of the batch operation.
    pub duration: std::time::Duration,
}

impl BatchSummary {
    /// Prints the summary to stdout.
    ///
    /// This method displays a comprehensive summary of the batch recording operation,
    /// including success/failure counts, duration, and detailed failure information.
    pub fn print_summary(&self) {
        println!("\n{}", "=".repeat(60));
        println!("バッチ録音サマリー");
        println!("{}", "=".repeat(60));

        // 基本統計
        println!("\n📊 基本統計:");
        println!("  総件数: {}", self.total_count);
        println!("  成功: {} ✅", self.success_count);
        println!("  失敗: {} ❌", self.failure_count);

        // 成功率
        if self.total_count > 0 {
            let success_rate = (self.success_count as f64 / self.total_count as f64) * 100.0;
            println!("  成功率: {:.1}%", success_rate);
        }

        // 処理時間
        println!("\n⏱️  処理時間:");
        println!("  総時間: {:.2}秒", self.duration.as_secs_f64());
        if self.total_count > 0 {
            let avg_time = self.duration.as_secs_f64() / self.total_count as f64;
            println!("  平均時間: {:.2}秒/件", avg_time);
        }

        // 失敗詳細
        if !self.failures.is_empty() {
            println!("\n❌ 失敗詳細:");
            for (i, (title, error)) in self.failures.iter().enumerate() {
                println!("  {}. {}", i + 1, title);
                println!("     エラー: {}", error);
            }
        }

        println!("\n{}", "=".repeat(60));

        // 終了ステータス
        if self.failure_count > 0 {
            println!("⚠️  警告: {}件の録音が失敗しました", self.failure_count);
        } else {
            println!("✅ すべての録音が正常に完了しました");
        }
        println!("{}", "=".repeat(60));
    }

    /// Returns true if all recordings succeeded.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.failure_count == 0
    }

    /// Returns the success rate as a percentage (0.0 to 100.0).
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        if self.total_count == 0 {
            return 100.0;
        }
        (self.success_count as f64 / self.total_count as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::metadata::RecordingMetadata;
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
    async fn test_batch_recorder() {
        let recorder = BatchRecorder::new(2, 1, std::time::Duration::from_secs(30));
        let service = Arc::new(MockRecordService);

        let programs = vec![
            Program {
                title: "Program 1".to_string(),
                url: "http://example.com/1".to_string(),
                output_path: std::path::PathBuf::from("/tmp/1.m4a"),
            },
            Program {
                title: "Program 2".to_string(),
                url: "http://example.com/2".to_string(),
                output_path: std::path::PathBuf::from("/tmp/2.m4a"),
            },
        ];

        let summary = recorder.record_batch(programs, service).await
            .expect("Failed to record batch");

        assert_eq!(summary.total_count, 2);
        assert_eq!(summary.success_count, 2);
        assert_eq!(summary.failure_count, 0);
    }

    #[tokio::test]
    async fn test_empty_batch() {
        let recorder = BatchRecorder::new(2, 1, std::time::Duration::from_secs(30));
        let service = Arc::new(MockRecordService);

        let summary = recorder
            .record_batch(Vec::new(), service)
            .await
            .expect("Failed to record empty batch");

        assert_eq!(summary.total_count, 0);
        assert_eq!(summary.success_count, 0);
        assert_eq!(summary.failure_count, 0);
    }
}
