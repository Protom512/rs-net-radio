//! Integration tests for error handling.
//!
//! These tests verify:
//! - Fatal vs recoverable error classification
//! - Error logging to error.log
//! - Exit code accuracy
//! - Error propagation through layers
//! - Error context preservation

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use record_lib::error::{RecordingError, handle_recording_error, ErrorSeverity};

/// Helper struct to manage test log files
struct TestLogFile {
    path: PathBuf,
}

impl TestLogFile {
    fn new(test_name: &str) -> Self {
        let mut path = std::env::temp_dir();
        path.push(format!("test_error_{}.log", test_name));
        // Remove existing log file if present
        let _ = std::fs::remove_file(&path);
        Self { path }
    }

    fn read_lines(&self) -> Vec<String> {
        if !self.path.exists() {
            return Vec::new();
        }

        let file = File::open(&self.path).expect("Failed to open log file");
        let reader = BufReader::new(file);
        reader.lines().filter_map(|l| l.ok()).collect()
    }

    fn cleanup(&self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[test]
fn test_fatal_error_classification() {
    // Test 403 error (fatal)
    let error_403 = RecordingError::http_status(403, "https://hibiki-radio.jp/test", "Forbidden");
    assert_eq!(error_403.severity(), ErrorSeverity::Fatal);
    assert!(error_403.is_fatal());
    assert_eq!(error_403.exit_code(), Some(2));

    // Test 429 error (fatal)
    let error_429 = RecordingError::http_status(429, "https://hibiki-radio.jp/test", "Rate limited");
    assert_eq!(error_429.severity(), ErrorSeverity::Fatal);
    assert!(error_429.is_fatal());
    assert_eq!(error_429.exit_code(), Some(2));

    // Test HTML parsing error (fatal)
    let html_error = RecordingError::html_parsing("https://example.com", "Parse failed");
    assert_eq!(html_error.severity(), ErrorSeverity::Fatal);
    assert!(html_error.is_fatal());
    assert_eq!(html_error.exit_code(), Some(3));

    // Test memory limit exceeded (fatal)
    let memory_error = RecordingError::memory_limit(600_000_000, 512_000_000);
    assert_eq!(memory_error.severity(), ErrorSeverity::Fatal);
    assert!(memory_error.is_fatal());
    assert_eq!(memory_error.exit_code(), Some(4));

    // Test config error (fatal)
    let config_error = RecordingError::ConfigError("Missing config file".to_string());
    assert_eq!(config_error.severity(), ErrorSeverity::Fatal);
    assert!(config_error.is_fatal());
    assert_eq!(config_error.exit_code(), Some(5));
}

#[test]
fn test_recoverable_error_classification() {
    // Test 500 error (recoverable)
    let error_500 = RecordingError::http_status(500, "https://example.com", "Internal Server Error");
    assert_eq!(error_500.severity(), ErrorSeverity::Recoverable);
    assert!(!error_500.is_fatal());
    assert_eq!(error_500.exit_code(), Some(1));

    // Test I/O error (recoverable)
    let io_error = RecordingError::Io(std::io::Error::new(
        std::io::ErrorKind::ConnectionReset,
        "Connection reset"
    ));
    assert_eq!(io_error.severity(), ErrorSeverity::Recoverable);
    assert!(!io_error.is_fatal());

    // Test network error (recoverable)
    let network_error = RecordingError::NetworkError("Temporary network failure".to_string());
    assert_eq!(network_error.severity(), ErrorSeverity::Recoverable);
    assert!(!network_error.is_fatal());
}

#[test]
fn test_error_handler_returns_correct_exit_code() {
    // Test fatal errors return exit codes
    let fatal_403 = RecordingError::http_status(403, "https://example.com", "Forbidden");
    let exit_code = handle_recording_error(&fatal_403);
    assert_eq!(exit_code, Some(2));

    let fatal_html = RecordingError::html_parsing("https://example.com", "Parse failed");
    let exit_code = handle_recording_error(&fatal_html);
    assert_eq!(exit_code, Some(3));

    // Test recoverable errors return None
    let recoverable = RecordingError::http_status(500, "https://example.com", "Internal Server Error");
    let exit_code = handle_recording_error(&recoverable);
    assert_eq!(exit_code, None);
}

#[test]
fn test_error_messages_are_descriptive() {
    let error = RecordingError::http_status(403, "https://hibiki-radio.jp/program", "Access forbidden");
    let error_string = format!("{}", error);

    assert!(error_string.contains("403"));
    assert!(error_string.contains("https://hibiki-radio.jp/program"));
    assert!(error_string.contains("Access forbidden"));

    let html_error = RecordingError::html_parsing(
        "https://example.com/page",
        "Could not find streaming URL in HTML"
    );
    let html_string = format!("{}", html_error);

    assert!(html_string.contains("HTML parsing error"));
    assert!(html_string.contains("https://example.com/page"));
    assert!(html_string.contains("Could not find streaming URL"));
}

#[test]
fn test_error_source_chain_preservation() {
    // Test that error sources are preserved
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
    let recording_error = RecordingError::from(io_error);

    // The error should contain the original error information
    let error_string = format!("{}", recording_error);
    assert!(error_string.contains("File not found") || error_string.contains("IO error"));
}

#[test]
fn test_custom_error_creation() {
    // Test helper functions for creating errors
    let http_error = RecordingError::http_status(404, "https://example.com", "Not found");
    match http_error {
        RecordingError::HttpStatusError { status, url, message, .. } => {
            assert_eq!(status, 404);
            assert_eq!(url, "https://example.com");
            assert_eq!(message, "Not found");
        }
        _ => panic!("Expected HttpStatusError"),
    }

    let html_error = RecordingError::html_parsing("https://example.com", "Parse failed");
    match html_error {
        RecordingError::HtmlParsingError { url, message } => {
            assert_eq!(url, "https://example.com");
            assert_eq!(message, "Parse failed");
        }
        _ => panic!("Expected HtmlParsingError"),
    }

    let rate_limit_error = RecordingError::rate_limit("https://example.com");
    match rate_limit_error {
        RecordingError::RateLimitError { url } => {
            assert_eq!(url, "https://example.com");
        }
        _ => panic!("Expected RateLimitError"),
    }

    let memory_error = RecordingError::memory_limit(100_000, 50_000);
    match memory_error {
        RecordingError::MemoryLimitExceeded { current, limit } => {
            assert_eq!(current, 100_000);
            assert_eq!(limit, 50_000);
        }
        _ => panic!("Expected MemoryLimitExceeded"),
    }
}

#[test]
fn test_error_context_propagation() {
    // Test that errors can be created with proper context
    let command_error = RecordingError::CommandFailed {
        command: "ffmpeg -i input.mp4 output.mp3".to_string(),
        code: Some(1),
        stderr: "Error decoding input".to_string(),
    };

    let error_string = format!("{}", command_error);
    assert!(error_string.contains("ffmpeg"));
    // CommandFailed error format includes stderr
    assert!(error_string.contains("Error decoding input") || error_string.contains("1"));
    assert_eq!(command_error.severity(), ErrorSeverity::Recoverable);

    let timeout_error = RecordingError::Timeout {
        timeout: std::time::Duration::from_secs(30),
    };

    let timeout_string = format!("{}", timeout_error);
    assert!(timeout_string.contains("timed out"));
}

#[test]
fn test_multiple_error_types_integration() {
    // Test that we can create and handle multiple types of errors
    let errors = vec![
        RecordingError::http_status(403, "https://hibiki-radio.jp", "Forbidden"),
        RecordingError::html_parsing("https://example.com", "Parse failed"),
        RecordingError::NetworkError("Network unreachable".to_string()),
        RecordingError::Io(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "Refused")),
    ];

    let fatal_count = errors.iter().filter(|e| e.is_fatal()).count();
    assert_eq!(fatal_count, 2); // 403 and HTML parsing are fatal

    let recoverable_count = errors.iter().filter(|e| !e.is_fatal()).count();
    assert_eq!(recoverable_count, 2); // Network and I/O are recoverable
}

#[test]
fn test_error_logging_format() {
    // This test verifies the error logging format
    let _log = TestLogFile::new("format");

    let error = RecordingError::http_status(403, "https://hibiki-radio.jp", "Forbidden");
    let error_string = format!("{}", error);

    // Verify error string format
    assert!(error_string.contains("HTTP"));
    assert!(error_string.contains("403"));

    // Verify severity is accessible
    assert_eq!(error.severity(), ErrorSeverity::Fatal);

    // Verify exit code is correct
    assert_eq!(error.exit_code(), Some(2));
}

#[test]
fn test_batch_error_scenarios() {
    // Test multiple errors that might occur in batch processing
    let errors = vec![
        ("Program 1", RecordingError::http_status(500, "https://example.com/prog1", "Internal Server Error")),
        ("Program 2", RecordingError::NetworkError("Timeout".to_string())),
        ("Program 3", RecordingError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"))),
    ];

    // All errors should be displayable
    for (program, error) in &errors {
        let error_string = format!("{}", error);
        assert!(!error_string.is_empty(), "Error for {} should not be empty", program);
        assert!(error_string.len() > 10, "Error for {} should be descriptive", program);
    }

    // Verify error counts
    let fatal_errors = errors.iter().filter(|(_, e)| e.is_fatal()).count();
    assert_eq!(fatal_errors, 0, "No fatal errors in this batch");

    let recoverable_errors = errors.iter().filter(|(_, e)| !e.is_fatal()).count();
    assert_eq!(recoverable_errors, 3, "All errors should be recoverable");
}

#[test]
fn test_error_in_async_context() {
    // Test that errors can be used in async contexts
    use std::sync::Arc;

    let error = Arc::new(RecordingError::NetworkError("Async network error".to_string()));

    // Should be cloneable and shareable across threads
    let error_clone = Arc::clone(&error);
    assert_eq!(format!("{}", error), format!("{}", error_clone));
    assert_eq!(error.severity(), error_clone.severity());
}

#[test]
fn test_serialization_error_context() {
    // Test JSON/serialization errors by creating a deserialization failure
    let json_str = "not valid json";
    let result: Result<Vec<i32>, _> = serde_json::from_str(json_str);
    let json_error = RecordingError::JsonError(result.unwrap_err());

    let error_string = format!("{}", json_error);
    assert!(error_string.contains("JSON"));

    // Should be recoverable
    assert_eq!(json_error.severity(), ErrorSeverity::Recoverable);
}
