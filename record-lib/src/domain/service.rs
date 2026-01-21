//! Recording service trait definition.
//!
//! This module defines the core abstraction for recording services,
//! following the Repository pattern and enabling testable implementations.

use async_trait::async_trait;
use std::path::Path;

use crate::domain::metadata::RecordingMetadata;
use crate::domain::types::{OutputPath, ProgramTitle, StreamingUrl};
use crate::utils::RecordError;

/// Trait for recording services.
///
/// This trait abstracts the recording functionality for different radio services,
/// allowing for polymorphic behavior and testable implementations.
///
/// # Preconditions
///
/// - `url` must be a valid stream URL.
/// - `output_path` must be writable.
///
/// # Postconditions
///
/// - Recording file is saved to `output_path`.
/// - Metadata is returned with complete recording information.
///
/// # Invariants
///
/// - On error, resources are cleaned up appropriately.
#[async_trait]
pub trait RecordService: Send + Sync {
    /// Records a stream to a file.
    ///
    /// # Arguments
    ///
    /// * `url` - The stream URL to record.
    /// * `output_path` - The path where the recording will be saved.
    ///
    /// # Errors
    ///
    /// Returns `RecordError` if:
    /// - Network connection fails.
    /// - File I/O fails.
    /// - Stream format is unsupported.
    /// - External process (`FFmpeg`) fails.
    async fn record(&self, url: &str, output_path: &Path)
        -> Result<RecordingMetadata, RecordError>;

    /// Records a batch of programs.
    ///
    /// This default implementation provides sequential recording.
    /// Implementations may override for parallel recording.
    ///
    /// # Arguments
    ///
    /// * `programs` - List of programs to record.
    ///
    /// # Errors
    ///
    /// Returns `RecordError` if any recording fails.
    async fn record_batch(&self, programs: Vec<Program>) -> Result<BatchSummary, RecordError> {
        let mut successes = 0;
        let mut failures = Vec::new();

        for program in programs {
            match self
                .record(program.url.as_str(), program.output_path.as_path())
                .await
            {
                Ok(_) => successes += 1,
                Err(e) => {
                    failures.push((program.title.into(), e.to_string()));
                }
            }
        }

        Ok(BatchSummary {
            success_count: successes,
            failure_count: failures.len(),
            failures,
        })
    }
}

/// A program to be recorded.
///
/// Uses newtype pattern for type safety and to prevent mixing similar values.
#[derive(Debug, Clone)]
pub struct Program {
    /// Program title or name.
    pub title: ProgramTitle,

    /// Stream URL.
    pub url: StreamingUrl,

    /// Output file path.
    pub output_path: OutputPath,
}

/// Summary of a batch recording operation.
#[derive(Debug, Clone)]
pub struct BatchSummary {
    /// Number of successful recordings.
    pub success_count: usize,

    /// Number of failed recordings.
    pub failure_count: usize,

    /// List of failures with program name and error message.
    pub failures: Vec<(String, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct MockRecordService;

    #[async_trait]
    impl RecordService for MockRecordService {
        async fn record(
            &self,
            _url: &str,
            _output_path: &Path,
        ) -> Result<RecordingMetadata, RecordError> {
            Ok(RecordingMetadata::new(
                "Test".to_string(),
                chrono::Utc::now(),
                chrono::Utc::now(),
                "/tmp/test.m4a".to_string(),
                1024,
                128,
            ))
        }
    }

    #[tokio::test]
    async fn test_record_batch() {
        let service = Arc::new(MockRecordService);
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
        ];

        let summary = service.record_batch(programs).await.unwrap();

        assert_eq!(summary.success_count, 2);
        assert_eq!(summary.failure_count, 0);
        assert!(summary.failures.is_empty());
    }
}
