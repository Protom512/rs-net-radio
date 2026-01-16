//! Comprehensive error handling framework for the recording library.
//!
//! This module provides structured error handling with proper classification
//! of fatal vs recoverable errors, context propagation, and error logging.

use std::sync::Arc;
use thiserror::Error;

/// Classification of error severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Fatal error that requires application termination.
    Fatal,
    /// Recoverable error that allows continuation.
    Recoverable,
    /// Warning that doesn't prevent operation.
    Warning,
}

/// Comprehensive error type for the recording library.
#[derive(Error, Debug)]
pub enum RecordingError {
    /// I/O error during file operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP request error.
    #[error("HTTP request error: {0}")]
    HttpError(String),

    /// HTTP status code error with custom handling.
    #[error("HTTP {status} error for {url}: {message}")]
    HttpStatusError {
        status: u16,
        url: String,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// JSON parsing/serialization error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// HTML parsing error.
    #[error("HTML parsing error for {url}: {message}")]
    HtmlParsingError { url: String, message: String },

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// FFmpeg execution error.
    #[error("FFmpeg error: {0}")]
    FfmpegError(String),

    /// Network error.
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Authentication error.
    #[error("Authentication error: {0}")]
    AuthError(String),

    /// Rate limiting error.
    #[error("Rate limit exceeded for {url}")]
    RateLimitError { url: String },

    /// Memory limit exceeded.
    #[error("Memory limit exceeded: {current} bytes > {limit} bytes")]
    MemoryLimitExceeded { current: usize, limit: usize },

    /// External command failed.
    #[error("Command '{command}' failed with exit code {code:?}")]
    CommandFailed {
        command: String,
        code: Option<i32>,
        stderr: String,
    },

    /// Timeout error.
    #[error("Operation timed out after {timeout:?}")]
    Timeout { timeout: std::time::Duration },

    /// Generic error with context.
    #[error("{0}")]
    Other(String),
}

impl RecordingError {
    /// Returns the severity level of this error.
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            RecordingError::HttpStatusError { status, .. } => match status {
                403 | 429 => ErrorSeverity::Fatal,
                500..=599 => ErrorSeverity::Recoverable,
                _ => ErrorSeverity::Recoverable,
            },
            RecordingError::HtmlParsingError { .. } => ErrorSeverity::Fatal,
            RecordingError::ConfigError(_) => ErrorSeverity::Fatal,
            RecordingError::MemoryLimitExceeded { .. } => ErrorSeverity::Fatal,
            RecordingError::CommandFailed { code: Some(1..=125), .. } => ErrorSeverity::Recoverable,
            RecordingError::CommandFailed { code: Some(126..), .. } => ErrorSeverity::Fatal,
            RecordingError::Io(_) | RecordingError::JsonError(_) => ErrorSeverity::Recoverable,
            RecordingError::Timeout { .. } => ErrorSeverity::Recoverable,
            _ => ErrorSeverity::Recoverable,
        }
    }

    /// Returns the exit code associated with this error (if any).
    pub fn exit_code(&self) -> Option<i32> {
        match self {
            RecordingError::HttpStatusError { status: 403 | 429, .. } => Some(2),
            RecordingError::HtmlParsingError { .. } => Some(3),
            RecordingError::MemoryLimitExceeded { .. } => Some(4),
            RecordingError::ConfigError(_) => Some(5),
            _ => Some(1),
        }
    }

    /// Checks if this error is fatal and requires application termination.
    pub fn is_fatal(&self) -> bool {
        matches!(self.severity(), ErrorSeverity::Fatal)
    }

    /// Creates an HTTP status error.
    pub fn http_status(status: u16, url: &str, message: &str) -> Self {
        RecordingError::HttpStatusError {
            status,
            url: url.to_string(),
            message: message.to_string(),
            source: None,
        }
    }

    /// Creates an HTML parsing error.
    pub fn html_parsing(url: &str, message: &str) -> Self {
        RecordingError::HtmlParsingError {
            url: url.to_string(),
            message: message.to_string(),
        }
    }

    /// Creates a rate limit error.
    pub fn rate_limit(url: &str) -> Self {
        RecordingError::RateLimitError {
            url: url.to_string(),
        }
    }

    /// Creates a memory limit error.
    pub fn memory_limit(current: usize, limit: usize) -> Self {
        RecordingError::MemoryLimitExceeded { current, limit }
    }
}

/// Result type alias for RecordingError.
pub type Result<T> = std::result::Result<T, RecordingError>;

/// Handles a recording error with appropriate action based on severity.
///
/// # Arguments
///
/// * `error` - The error to handle.
///
/// # Returns
///
/// The exit code if the error is fatal, None otherwise.
pub fn handle_recording_error(error: &RecordingError) -> Option<i32> {
    let severity = error.severity();

    match severity {
        ErrorSeverity::Fatal => {
            tracing::error!("Fatal error: {}", error);
            tracing::error!("Application must exit due to fatal error");

            if let Some(exit_code) = error.exit_code() {
                tracing::error!("Exit code: {}", exit_code);
                Some(exit_code)
            } else {
                Some(1)
            }
        }
        ErrorSeverity::Recoverable => {
            tracing::warn!("Recoverable error: {}", error);
            tracing::info!("Continuing despite error...");
            None
        }
        ErrorSeverity::Warning => {
            tracing::info!("Warning: {}", error);
            None
        }
    }
}

/// Handles a recording error and logs it to error.log.
///
/// # Arguments
///
/// * `error` - The error to handle.
///
/// # Returns
///
/// The exit code if the error is fatal, None otherwise.
pub fn handle_and_log_error(error: &RecordingError) -> Option<i32> {
    // Log to error.log
    if let Err(e) = log_error_to_file(error) {
        tracing::error!("Failed to write error to error.log: {}", e);
    }

    handle_recording_error(error)
}

/// Logs an error to the error.log file.
///
/// # Arguments
///
/// * `error` - The error to log.
///
/// # Errors
///
/// Returns an error if writing to the log file fails.
fn log_error_to_file(error: &RecordingError) -> std::io::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let log_path = "error.log";
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let severity = match error.severity() {
        ErrorSeverity::Fatal => "FATAL",
        ErrorSeverity::Recoverable => "ERROR",
        ErrorSeverity::Warning => "WARN",
    };

    writeln!(file, "[{}] [{}] {}", timestamp, severity, error)?;

    if let Some(source) = std::error::Error::source(error) {
        writeln!(file, "  Caused by: {}", source)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity() {
        let http_403 = RecordingError::http_status(403, "https://example.com", "Forbidden");
        assert_eq!(http_403.severity(), ErrorSeverity::Fatal);
        assert!(http_403.is_fatal());
        assert_eq!(http_403.exit_code(), Some(2));

        let http_500 = RecordingError::http_status(500, "https://example.com", "Internal Server Error");
        assert_eq!(http_500.severity(), ErrorSeverity::Recoverable);
        assert!(!http_500.is_fatal());

        let html_error = RecordingError::html_parsing("https://example.com", "Failed to parse");
        assert_eq!(html_error.severity(), ErrorSeverity::Fatal);
        assert!(html_error.is_fatal());
        assert_eq!(html_error.exit_code(), Some(3));
    }

    #[test]
    fn test_memory_limit_error() {
        let error = RecordingError::memory_limit(600_000_000, 512_000_000);
        assert_eq!(error.severity(), ErrorSeverity::Fatal);
        assert!(error.is_fatal());
        assert_eq!(error.exit_code(), Some(4));
    }

    #[test]
    fn test_handle_recording_error() {
        let fatal_error = RecordingError::http_status(403, "https://example.com", "Forbidden");
        let exit_code = handle_recording_error(&fatal_error);
        assert_eq!(exit_code, Some(2));

        let recoverable_error = RecordingError::http_status(500, "https://example.com", "Internal Server Error");
        let exit_code = handle_recording_error(&recoverable_error);
        assert_eq!(exit_code, None);
    }
}
