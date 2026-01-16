//! Recording metadata structures.
//!
//! This module defines value objects for recording metadata,
//! following Domain-Driven Design principles.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Metadata for a completed recording.
///
/// This value object contains all relevant information about a recording,
/// including timing, file size, and quality metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingMetadata {
    /// Program name or title.
    pub program_name: String,

    /// Recording start time.
    pub start_time: DateTime<Utc>,

    /// Recording end time.
    pub end_time: DateTime<Utc>,

    /// Output file path.
    pub file_path: String,

    /// File size in bytes.
    pub file_size: u64,

    /// Bitrate in kbps.
    pub bitrate: u32,
}

impl RecordingMetadata {
    /// Creates new recording metadata.
    ///
    /// # Arguments
    ///
    /// * `program_name` - Name of the program.
    /// * `start_time` - When recording started.
    /// * `end_time` - When recording ended.
    /// * `file_path` - Path to the recorded file.
    /// * `file_size` - Size of the file in bytes.
    /// * `bitrate` - Audio bitrate in kbps.
    #[must_use]
    pub const fn new(
        program_name: String,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        file_path: String,
        file_size: u64,
        bitrate: u32,
    ) -> Self {
        Self {
            program_name,
            start_time,
            end_time,
            file_path,
            file_size,
            bitrate,
        }
    }

    /// Returns the recording duration in seconds.
    #[must_use]
    pub fn duration_secs(&self) -> i64 {
        self.end_time.timestamp() - self.start_time.timestamp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recording_metadata_duration() {
        let start = DateTime::from_timestamp(1_609_459_200, 0).unwrap(); // 2021-01-01 00:00:00 UTC
        let end = DateTime::from_timestamp(1_609_462_800, 0).unwrap(); // 2021-01-01 01:00:00 UTC

        let metadata = RecordingMetadata::new(
            "Test Program".to_string(),
            start,
            end,
            "/tmp/test.m4a".to_string(),
            1024 * 1024, // 1MB
            128,
        );

        assert_eq!(metadata.duration_secs(), 3600); // 1 hour
    }
}
