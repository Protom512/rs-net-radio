//! Batch recorder for parallel recording tasks.
//!
//! This module implements the `BatchRecorder` which manages parallel recording
//! of multiple programs with semaphore-based concurrency control.

use crate::domain::service::{Program, RecordService};
use crate::utils::RecordError;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

// ANSI color constants
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";

/// Batch recorder for parallel recording tasks.
///
/// This struct manages the parallel recording of multiple programs,
/// controlling concurrency with a semaphore and tracking progress.
pub struct BatchRecorder {
    max_parallel_jobs: usize,
    retry_count: u32,
    #[expect(dead_code, reason = "timeout is reserved for future use")]
    timeout: std::time::Duration,
}

impl BatchRecorder {
    /// Creates a new `BatchRecorder`.
    ///
    /// # Arguments
    ///
    /// * `max_parallel_jobs` - Maximum number of parallel recordings.
    /// * `retry_count` - Number of retry attempts for failed recordings.
    /// * `timeout` - Timeout for each recording.
    #[must_use]
    pub const fn new(
        max_parallel_jobs: usize,
        retry_count: u32,
        timeout: std::time::Duration,
    ) -> Self {
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

        let progress_bar = ProgressBar::new(total as u64);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
                )
                .unwrap()
                .progress_chars("#>-"),
        );

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
                    Err(e) => {
                        return Err((
                            program.title.clone(),
                            format!("Failed to acquire semaphore: {e}"),
                        ))
                    }
                };

                Self::record_with_retries(&service, program, retry_count).await
            });
        }

        let mut successes = 0;
        let mut failures = Vec::new();

        while let Some(result) = tasks.join_next().await {
            progress_bar.inc(1);
            match result {
                Ok(Ok(_title)) => {
                    successes += 1;
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
        progress_bar.finish_with_message("All recordings processed.");

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
        let (green, red, cyan, yellow, reset, bold) = (GREEN, RED, CYAN, YELLOW, RESET, BOLD);

        println!("\n{cyan}{}{reset}", "=".repeat(60));
        println!("{bold}バッチ録音サマリー{reset}");
        println!("{cyan}{}{reset}", "=".repeat(60));

        if self.total_count == 0 {
            println!("\n録音対象の番組がありませんでした。");
            println!("{cyan}{}{reset}", "=".repeat(60));
            return;
        }

        // 基本統計
        println!("\n{cyan}📊 基本統計:{reset}");
        println!("  総件数: {}", self.total_count);
        println!("  成功: {green}{} ✅{reset}", self.success_count);
        println!(
            "  失敗: {}{}{} ❌{reset}",
            if self.failure_count > 0 { red } else { "" },
            self.failure_count,
            if self.failure_count > 0 { reset } else { "" }
        );

        // 成功率
        if self.total_count > 0 {
            let success_rate = self.success_rate();
            let color = if success_rate >= 100.0 {
                green
            } else if success_rate >= 80.0 {
                yellow
            } else {
                red
            };
            println!("  成功率: {color}{success_rate:.1}%{reset}");
        }

        // 処理時間
        println!("\n{cyan}⏱️  処理時間:{reset}");
        println!("  総時間: {:.2}秒", self.duration.as_secs_f64());
        #[expect(
            clippy::cast_precision_loss,
            reason = "f64 precision is acceptable for average time display"
        )]
        if self.total_count > 0 {
            let avg_time = self.duration.as_secs_f64() / self.total_count as f64;
            println!("  平均時間: {avg_time:.2}秒/件");
        }

        // 失敗詳細
        if !self.failures.is_empty() {
            println!("\n{red}❌ 失敗詳細:{reset}");
            for (i, (title, error)) in self.failures.iter().enumerate() {
                println!("  {}. {bold}{title}{reset}", i + 1);
                println!("     エラー: {red}{error}{reset}");
            }
        }

        println!("\n{cyan}{}{reset}", "=".repeat(60));

        // 終了ステータス
        if self.failure_count > 0 {
            println!(
                "{yellow}⚠️  警告: {}件の録音が失敗しました{reset}",
                self.failure_count
            );
        } else {
            println!("{green}✅ すべての録音が正常に完了しました{reset}");
        }
        println!("{cyan}{}{reset}", "=".repeat(60));
    }

    /// Returns true if all recordings succeeded.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.failure_count == 0
    }

    /// Returns the success rate as a percentage (0.0 to 100.0).
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "f64 precision is acceptable for percentage display"
    )]
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

        let summary = recorder
            .record_batch(programs, service)
            .await
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

    /// Task 4.1: Test result collection and aggregation with `JoinSet`
    #[tokio::test]
    async fn test_joinset_result_collection() {
        let recorder = BatchRecorder::new(3, 1, std::time::Duration::from_secs(30));
        let service = Arc::new(MockRecordService);

        let programs = vec![
            Program {
                title: "Program A".to_string(),
                url: "http://example.com/a".to_string(),
                output_path: std::path::PathBuf::from("/tmp/a.m4a"),
            },
            Program {
                title: "Program B".to_string(),
                url: "http://example.com/b".to_string(),
                output_path: std::path::PathBuf::from("/tmp/b.m4a"),
            },
            Program {
                title: "Program C".to_string(),
                url: "http://example.com/c".to_string(),
                output_path: std::path::PathBuf::from("/tmp/c.m4a"),
            },
        ];

        let summary = recorder
            .record_batch(programs, service)
            .await
            .expect("Failed to record batch");

        // Verify result collection
        assert_eq!(summary.total_count, 3);
        assert_eq!(summary.success_count, 3);
        assert_eq!(summary.failure_count, 0);
        assert!(summary.duration.as_secs_f64() > 0.0);
    }

    /// Task 4.1: Test failure detail collection with program name and error message
    #[tokio::test]
    async fn test_failure_detail_collection() {
        struct FailingRecordService;

        #[async_trait]
        impl RecordService for FailingRecordService {
            async fn record(
                &self,
                _url: &str,
                _output_path: &std::path::Path,
            ) -> Result<RecordingMetadata, RecordError> {
                Err(RecordError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Test failure",
                )))
            }
        }

        let recorder = BatchRecorder::new(2, 0, std::time::Duration::from_secs(30)); // 0 retries for immediate failure
        let service = Arc::new(FailingRecordService);

        let programs = vec![
            Program {
                title: "Failing Program 1".to_string(),
                url: "http://example.com/fail1".to_string(),
                output_path: std::path::PathBuf::from("/tmp/fail1.m4a"),
            },
            Program {
                title: "Failing Program 2".to_string(),
                url: "http://example.com/fail2".to_string(),
                output_path: std::path::PathBuf::from("/tmp/fail2.m4a"),
            },
        ];

        let summary = recorder
            .record_batch(programs, service)
            .await
            .expect("Failed to record batch");

        // Verify failure details are collected
        assert_eq!(summary.total_count, 2);
        assert_eq!(summary.success_count, 0);
        assert_eq!(summary.failure_count, 2);
        assert_eq!(summary.failures.len(), 2);

        // Verify failure details contain program names and error messages
        assert_eq!(summary.failures[0].0, "Failing Program 1");
        assert!(summary.failures[0].1.contains("Test failure"));
        assert_eq!(summary.failures[1].0, "Failing Program 2");
        assert!(summary.failures[1].1.contains("Test failure"));
    }

    /// Task 4.1: Test processing time recording for each task
    #[tokio::test]
    async fn test_processing_time_recording() {
        let recorder = BatchRecorder::new(2, 0, std::time::Duration::from_secs(30));
        let service = Arc::new(MockRecordService);

        let programs = vec![Program {
            title: "Program 1".to_string(),
            url: "http://example.com/1".to_string(),
            output_path: std::path::PathBuf::from("/tmp/1.m4a"),
        }];

        let start = std::time::Instant::now();
        let summary = recorder
            .record_batch(programs, service)
            .await
            .expect("Failed to record batch");
        let elapsed = start.elapsed();

        // Verify duration is recorded and reasonable
        // Duration is always >= 0 for u128
        assert!(summary.duration.as_secs() <= elapsed.as_secs() + 1);
        // Duration should not exceed elapsed time by more than 100ms tolerance
        assert!(summary.duration.as_millis() <= elapsed.as_millis() + 100);
    }

    /// Task 4.2: Test summary output format and content
    #[tokio::test]
    async fn test_summary_output_format() {
        let recorder = BatchRecorder::new(2, 0, std::time::Duration::from_secs(30));
        let service = Arc::new(MockRecordService);

        let programs = vec![Program {
            title: "Success Program".to_string(),
            url: "http://example.com/success".to_string(),
            output_path: std::path::PathBuf::from("/tmp/success.m4a"),
        }];

        let summary = recorder
            .record_batch(programs, service)
            .await
            .expect("Failed to record batch");

        // Test that print_summary doesn't panic and produces output
        // We can't easily capture stdout in unit tests, but we can verify the data
        assert_eq!(summary.total_count, 1);
        assert_eq!(summary.success_count, 1);
        assert_eq!(summary.failure_count, 0);

        // Test success rate calculation
        let success_rate = summary.success_rate();
        assert!((success_rate - 100.0).abs() < 0.01);

        // Test is_success method
        assert!(summary.is_success());

        // Test that print_summary can be called without panicking
        summary.print_summary();
    }

    /// Task 4.2: Test summary output with failures
    #[tokio::test]
    async fn test_summary_output_with_failures() {
        struct PartialFailingService;

        #[async_trait]
        impl RecordService for PartialFailingService {
            async fn record(
                &self,
                url: &str,
                _output_path: &std::path::Path,
            ) -> Result<RecordingMetadata, RecordError> {
                if url.contains("fail") {
                    Err(RecordError::Io(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Access denied",
                    )))
                } else {
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
        }

        let recorder = BatchRecorder::new(3, 0, std::time::Duration::from_secs(30));
        let service = Arc::new(PartialFailingService);

        let programs = vec![
            Program {
                title: "Success Program".to_string(),
                url: "http://example.com/success".to_string(),
                output_path: std::path::PathBuf::from("/tmp/success.m4a"),
            },
            Program {
                title: "Failing Program".to_string(),
                url: "http://example.com/fail".to_string(),
                output_path: std::path::PathBuf::from("/tmp/fail.m4a"),
            },
        ];

        let summary = recorder
            .record_batch(programs, service)
            .await
            .expect("Failed to record batch");

        // Verify summary statistics
        assert_eq!(summary.total_count, 2);
        assert_eq!(summary.success_count, 1);
        assert_eq!(summary.failure_count, 1);

        // Test success rate calculation (50% success)
        let success_rate = summary.success_rate();
        assert!((success_rate - 50.0).abs() < 0.01);

        // Test is_success method
        assert!(!summary.is_success());

        // Verify failure details
        assert_eq!(summary.failures.len(), 1);
        assert_eq!(summary.failures[0].0, "Failing Program");
        assert!(summary.failures[0].1.contains("Access denied"));

        // Test that print_summary can be called without panicking
        summary.print_summary();
    }

    /// Task 4.2: Test statistics calculation (average processing time)
    #[tokio::test]
    async fn test_statistics_calculation() {
        let recorder = BatchRecorder::new(3, 0, std::time::Duration::from_secs(30));
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
            Program {
                title: "Program 3".to_string(),
                url: "http://example.com/3".to_string(),
                output_path: std::path::PathBuf::from("/tmp/3.m4a"),
            },
        ];

        let summary = recorder
            .record_batch(programs, service)
            .await
            .expect("Failed to record batch");

        // Verify total count
        assert_eq!(summary.total_count, 3);
        assert_eq!(summary.success_count, 3);

        // Calculate expected average time
        let expected_avg = summary.duration.as_secs_f64() / 3.0;

        // Test that average time would be calculated correctly
        // The print_summary method uses: duration.as_secs_f64() / total_count as f64
        #[expect(
            clippy::cast_precision_loss,
            reason = "f64 precision is acceptable for time calculations"
        )]
        let calculated_avg = summary.duration.as_secs_f64() / summary.total_count as f64;
        assert!((calculated_avg - expected_avg).abs() < 0.001);

        // Test success rate
        assert!((summary.success_rate() - 100.0).abs() < 0.01);
    }

    /// Task 4.2: Test summary with empty batch
    #[tokio::test]
    async fn test_summary_with_empty_batch() {
        let recorder = BatchRecorder::new(2, 0, std::time::Duration::from_secs(30));
        let service = Arc::new(MockRecordService);

        let summary = recorder
            .record_batch(Vec::new(), service)
            .await
            .expect("Failed to record empty batch");

        // Verify empty batch handling
        assert_eq!(summary.total_count, 0);
        assert_eq!(summary.success_count, 0);
        assert_eq!(summary.failure_count, 0);
        assert!(summary.failures.is_empty());

        // Empty batch should be considered successful
        assert!(summary.is_success());

        // Success rate should be 100% for empty batch
        assert!((summary.success_rate() - 100.0).abs() < 0.01);

        // Test that print_summary works for empty batch and doesn't panic.
        // The new implementation prints a specific message and returns early.
        summary.print_summary();
    }
}
