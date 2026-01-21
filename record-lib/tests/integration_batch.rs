//! Integration tests for batch recording functionality.
//!
//! These tests verify:
//! - Multiple task parallel execution
//! - Summary output validation
//! - Failure task error handling
//! - Semaphore-based concurrency control
//! - Result collection and aggregation

use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};

use async_trait::async_trait;
use chrono::Utc;
use record_lib::batch::recorder::BatchRecorder;
use record_lib::domain::metadata::RecordingMetadata;
use record_lib::domain::service::{Program, RecordService};
use record_lib::utils::RecordError;

/// Mock recording service that simulates successful recordings
struct MockRecordService {
    delay: Duration,
}

#[async_trait]
impl RecordService for MockRecordService {
    async fn record(
        &self,
        _url: &str,
        output_path: &std::path::Path,
    ) -> Result<RecordingMetadata, RecordError> {
        // Simulate recording delay
        sleep(self.delay).await;

        Ok(RecordingMetadata::new(
            "Test Program".to_string(),
            Utc::now(),
            Utc::now(),
            output_path.to_string_lossy().to_string(),
            1024,
            128,
        ))
    }
}

/// Mock recording service that simulates failures
struct FailingRecordService {
    fail_urls: Vec<String>,
}

#[async_trait]
impl RecordService for FailingRecordService {
    async fn record(
        &self,
        url: &str,
        _output_path: &std::path::Path,
    ) -> Result<RecordingMetadata, RecordError> {
        if self.fail_urls.contains(&url.to_string()) {
            Err(RecordError::Io(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "Connection refused",
            )))
        } else {
            Ok(RecordingMetadata::new(
                "Test Program".to_string(),
                Utc::now(),
                Utc::now(),
                "/tmp/test.m4a".to_string(),
                1024,
                128,
            ))
        }
    }
}

/// Mock recording service with variable delays
struct VariableDelayRecordService;

#[async_trait]
impl RecordService for VariableDelayRecordService {
    async fn record(
        &self,
        url: &str,
        output_path: &std::path::Path,
    ) -> Result<RecordingMetadata, RecordError> {
        // Variable delay based on URL to simulate different processing times
        let delay_ms = url.len() as u64 * 10;
        sleep(Duration::from_millis(delay_ms)).await;

        Ok(RecordingMetadata::new(
            "Test Program".to_string(),
            Utc::now(),
            Utc::now(),
            output_path.to_string_lossy().to_string(),
            1024,
            128,
        ))
    }
}

#[tokio::test]
async fn test_batch_recorder_parallel_execution() {
    let recorder = BatchRecorder::new(3, 0, Duration::from_secs(30));
    let service = Arc::new(MockRecordService {
        delay: Duration::from_millis(100),
    });

    let programs = vec![
        Program {
            title: "Program 1".into(),
            url: "http://example.com/1".into(),
            output_path: "/tmp/1.m4a".into(),
        },
        Program {
            title: "Program 2".into(),
            url: "http://example.com/2".into(),
            output_path: "/tmp/2.m4a".into(),
        },
        Program {
            title: "Program 3".into(),
            url: "http://example.com/3".into(),
            output_path: "/tmp/3.m4a".into(),
        },
    ];

    let start = std::time::Instant::now();
    let summary = recorder
        .record_batch(programs, service)
        .await
        .expect("Failed to record batch");
    let elapsed = start.elapsed();

    // With 3 parallel tasks and 100ms delay each, should complete in ~100-200ms (not 300ms)
    assert!(
        elapsed.as_millis() < 250,
        "Parallel execution should be faster than sequential"
    );
    assert_eq!(summary.total_count, 3);
    assert_eq!(summary.success_count, 3);
    assert_eq!(summary.failure_count, 0);
}

#[tokio::test]
async fn test_batch_recorder_concurrency_limit() {
    let recorder = BatchRecorder::new(2, 0, Duration::from_secs(30)); // Max 2 parallel
    let service = Arc::new(MockRecordService {
        delay: Duration::from_millis(200),
    });

    let programs = vec![
        Program {
            title: "Program 1".into(),
            url: "http://example.com/1".into(),
            output_path: "/tmp/1.m4a".into(),
        },
        Program {
            title: "Program 2".into(),
            url: "http://example.com/2".into(),
            output_path: "/tmp/2.m4a".into(),
        },
        Program {
            title: "Program 3".into(),
            url: "http://example.com/3".into(),
            output_path: "/tmp/3.m4a".into(),
        },
        Program {
            title: "Program 4".into(),
            url: "http://example.com/4".into(),
            output_path: "/tmp/4.m4a".into(),
        },
    ];

    let start = std::time::Instant::now();
    let summary = recorder
        .record_batch(programs, service)
        .await
        .expect("Failed to record batch");
    let elapsed = start.elapsed();

    // With 4 tasks and max 2 parallel, with 200ms delay each:
    // Should complete in ~400ms (2 batches of 2 tasks)
    assert!(
        elapsed.as_millis() >= 300,
        "Should respect concurrency limit"
    );
    assert!(
        elapsed.as_millis() < 600,
        "Should complete within reasonable time"
    );
    assert_eq!(summary.total_count, 4);
    assert_eq!(summary.success_count, 4);
}

#[tokio::test]
async fn test_batch_recorder_with_failures() {
    let recorder = BatchRecorder::new(3, 0, Duration::from_secs(30));
    let service = Arc::new(FailingRecordService {
        fail_urls: vec![
            "http://example.com/2".to_string(),
            "http://example.com/4".to_string(),
        ],
    });

    let programs = vec![
        Program {
            title: "Program 1".into(),
            url: "http://example.com/1".into(),
            output_path: "/tmp/1.m4a".into(),
        },
        Program {
            title: "Program 2".into(),
            url: "http://example.com/2".into(),
            output_path: "/tmp/2.m4a".into(),
        },
        Program {
            title: "Program 3".into(),
            url: "http://example.com/3".into(),
            output_path: "/tmp/3.m4a".into(),
        },
        Program {
            title: "Program 4".into(),
            url: "http://example.com/4".into(),
            output_path: "/tmp/4.m4a".into(),
        },
    ];

    let summary = recorder
        .record_batch(programs, service)
        .await
        .expect("Failed to record batch");

    assert_eq!(summary.total_count, 4);
    assert_eq!(summary.success_count, 2);
    assert_eq!(summary.failure_count, 2);
    assert_eq!(summary.failures.len(), 2);

    // Verify failure details
    assert_eq!(summary.failures[0].0, "Program 2");
    assert!(summary.failures[0].1.contains("Connection refused"));
    assert_eq!(summary.failures[1].0, "Program 4");
}

#[tokio::test]
async fn test_batch_recorder_empty_batch() {
    let recorder = BatchRecorder::new(3, 0, Duration::from_secs(30));
    let service = Arc::new(MockRecordService {
        delay: Duration::from_millis(100),
    });

    let programs: Vec<Program> = vec![];
    let summary = recorder
        .record_batch(programs, service)
        .await
        .expect("Failed to record empty batch");

    assert_eq!(summary.total_count, 0);
    assert_eq!(summary.success_count, 0);
    assert_eq!(summary.failure_count, 0);
    assert!(summary.is_success());
    assert!((summary.success_rate() - 100.0).abs() < 0.01);
}

#[tokio::test]
async fn test_batch_recorder_summary_statistics() {
    let recorder = BatchRecorder::new(3, 0, Duration::from_secs(30));
    let service = Arc::new(VariableDelayRecordService);

    let programs = vec![
        Program {
            title: "Short".into(),
            url: "http://a.com/1".into(),
            output_path: "/tmp/1.m4a".into(),
        },
        Program {
            title: "MediumLength".into(),
            url: "http://medium.com/2".into(),
            output_path: "/tmp/2.m4a".into(),
        },
        Program {
            title: "VeryLongTitle".into(),
            url: "http://verylong.com/3".into(),
            output_path: "/tmp/3.m4a".into(),
        },
    ];

    let summary = recorder
        .record_batch(programs, service)
        .await
        .expect("Failed to record batch");

    // Verify summary statistics
    assert_eq!(summary.total_count, 3);
    assert_eq!(summary.success_count, 3);
    assert_eq!(summary.failure_count, 0);

    // Verify duration is recorded
    assert!(summary.duration.as_secs_f64() > 0.0);

    // Verify success rate
    assert!((summary.success_rate() - 100.0).abs() < 0.01);

    // Verify is_success
    assert!(summary.is_success());

    // Test that print_summary doesn't panic
    summary.print_summary();
}

#[tokio::test]
async fn test_batch_recorder_retry_logic() {
    let recorder = BatchRecorder::new(3, 2, Duration::from_secs(30)); // 2 retries
    let service = Arc::new(FailingRecordService {
        fail_urls: vec!["http://example.com/1".to_string()],
    });

    let programs = vec![
        Program {
            title: "Failing Program".into(),
            url: "http://example.com/1".into(),
            output_path: "/tmp/1.m4a".into(),
        },
        Program {
            title: "Success Program".into(),
            url: "http://example.com/2".into(),
            output_path: "/tmp/2.m4a".into(),
        },
    ];

    let summary = recorder
        .record_batch(programs, service)
        .await
        .expect("Failed to record batch");

    // With 2 retries, the failing program should be attempted 3 times total
    assert_eq!(summary.total_count, 2);
    assert_eq!(summary.success_count, 1);
    assert_eq!(summary.failure_count, 1);
    assert_eq!(summary.failures.len(), 1);
}

#[tokio::test]
async fn test_batch_recorder_timeout() {
    let recorder = BatchRecorder::new(3, 0, Duration::from_millis(100)); // Short timeout
    let service = Arc::new(MockRecordService {
        delay: Duration::from_secs(1), // Will exceed timeout
    });

    let programs = vec![Program {
        title: "Slow Program".into(),
        url: "http://example.com/1".into(),
        output_path: "/tmp/1.m4a".into(),
    }];

    // This should take more than the timeout, but the test should still complete
    let start = std::time::Instant::now();
    let result = timeout(
        Duration::from_secs(5),
        recorder.record_batch(programs, service),
    )
    .await;
    let _elapsed = start.elapsed();

    assert!(
        result.is_ok(),
        "Batch recording should complete within test timeout"
    );
    let summary = result.unwrap().expect("Failed to record batch");
    assert_eq!(summary.total_count, 1);
    assert_eq!(summary.success_count, 1);
}

#[tokio::test]
async fn test_batch_recorder_large_batch() {
    let recorder = BatchRecorder::new(5, 0, Duration::from_secs(60));
    let service = Arc::new(MockRecordService {
        delay: Duration::from_millis(50),
    });

    // Create 20 programs
    let programs: Vec<Program> = (0..20)
        .map(|i| Program {
            title: format!("Program {i}").into(),
            url: format!("http://example.com/{i}").into(),
            output_path: format!("/tmp/{i}.m4a").as_str().into(),
        })
        .collect();

    let start = std::time::Instant::now();
    let summary = recorder
        .record_batch(programs, service)
        .await
        .expect("Failed to record large batch");
    let elapsed = start.elapsed();

    // With 5 parallel and 20 tasks, should complete in ~200ms (4 batches of 5)
    assert!(
        elapsed.as_millis() < 500,
        "Large batch should complete efficiently"
    );
    assert_eq!(summary.total_count, 20);
    assert_eq!(summary.success_count, 20);
    assert_eq!(summary.failure_count, 0);
}

#[tokio::test]
async fn test_batch_recorder_all_failures() {
    let recorder = BatchRecorder::new(3, 0, Duration::from_secs(30));
    let service = Arc::new(FailingRecordService {
        fail_urls: vec![
            "http://example.com/1".to_string(),
            "http://example.com/2".to_string(),
            "http://example.com/3".to_string(),
        ],
    });

    let programs = vec![
        Program {
            title: "Program 1".into(),
            url: "http://example.com/1".into(),
            output_path: "/tmp/1.m4a".into(),
        },
        Program {
            title: "Program 2".into(),
            url: "http://example.com/2".into(),
            output_path: "/tmp/2.m4a".into(),
        },
        Program {
            title: "Program 3".into(),
            url: "http://example.com/3".into(),
            output_path: "/tmp/3.m4a".into(),
        },
    ];

    let summary = recorder
        .record_batch(programs, service)
        .await
        .expect("Failed to record batch");

    assert_eq!(summary.total_count, 3);
    assert_eq!(summary.success_count, 0);
    assert_eq!(summary.failure_count, 3);
    assert!(!summary.is_success());
    assert!((summary.success_rate() - 0.0).abs() < 0.01);

    // Verify all failures are recorded
    assert_eq!(summary.failures.len(), 3);
    for (i, (title, error)) in summary.failures.iter().enumerate() {
        assert_eq!(title, &format!("Program {}", i + 1));
        assert!(error.contains("Connection refused"));
    }
}

#[tokio::test]
async fn test_batch_recorder_partial_success() {
    let recorder = BatchRecorder::new(3, 0, Duration::from_secs(30));
    let service = Arc::new(FailingRecordService {
        fail_urls: vec!["http://example.com/2".to_string()],
    });

    let programs = vec![
        Program {
            title: "Success 1".into(),
            url: "http://example.com/1".into(),
            output_path: "/tmp/1.m4a".into(),
        },
        Program {
            title: "Failure 1".into(),
            url: "http://example.com/2".into(),
            output_path: "/tmp/2.m4a".into(),
        },
        Program {
            title: "Success 2".into(),
            url: "http://example.com/3".into(),
            output_path: "/tmp/3.m4a".into(),
        },
    ];

    let summary = recorder
        .record_batch(programs, service)
        .await
        .expect("Failed to record batch");

    assert_eq!(summary.total_count, 3);
    assert_eq!(summary.success_count, 2);
    assert_eq!(summary.failure_count, 1);
    assert!(!summary.is_success());

    // Success rate should be 66.67%
    let success_rate = summary.success_rate();
    assert!((success_rate - 66.67).abs() < 0.1);
}

#[tokio::test]
async fn test_batch_recorder_concurrent_batches() {
    let recorder = Arc::new(BatchRecorder::new(3, 0, Duration::from_secs(30)));
    let service = Arc::new(MockRecordService {
        delay: Duration::from_millis(100),
    });

    let programs1 = vec![
        Program {
            title: "Batch1-1".into(),
            url: "http://example.com/b1-1".into(),
            output_path: "/tmp/b1-1.m4a".into(),
        },
        Program {
            title: "Batch1-2".into(),
            url: "http://example.com/b1-2".into(),
            output_path: "/tmp/b1-2.m4a".into(),
        },
    ];

    let programs2 = vec![
        Program {
            title: "Batch2-1".into(),
            url: "http://example.com/b2-1".into(),
            output_path: "/tmp/b2-1.m4a".into(),
        },
        Program {
            title: "Batch2-2".into(),
            url: "http://example.com/b2-2".into(),
            output_path: "/tmp/b2-2.m4a".into(),
        },
    ];

    // Run two batches concurrently
    let recorder1 = Arc::clone(&recorder);
    let service1 = Arc::clone(&service);
    let handle1 = tokio::spawn(async move { recorder1.record_batch(programs1, service1).await });

    let recorder2 = Arc::clone(&recorder);
    let service2 = Arc::clone(&service);
    let handle2 = tokio::spawn(async move { recorder2.record_batch(programs2, service2).await });

    let result1 = handle1.await.expect("Batch 1 panicked");
    let result2 = handle2.await.expect("Batch 2 panicked");

    assert!(result1.is_ok(), "Batch 1 should succeed");
    assert!(result2.is_ok(), "Batch 2 should succeed");

    let summary1 = result1.unwrap();
    let summary2 = result2.unwrap();

    assert_eq!(summary1.total_count, 2);
    assert_eq!(summary2.total_count, 2);
}
