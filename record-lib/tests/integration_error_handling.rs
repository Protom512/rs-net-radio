//! Integration tests for error handling behavior.
//!
//! These tests verify:
//! - Fatal vs recoverable error classification (via public API)
//! - Error handler returns appropriate exit codes
//! - Errors are displayable and distinguishable
//! - Error context is preserved

use record_lib::error::{handle_recording_error, ErrorSeverity, RecordingError};

/// Helper to verify error classification behavior
fn verify_error_classification(error: &RecordingError, expected_severity: ErrorSeverity) {
    assert_eq!(
        error.severity(),
        expected_severity,
        "Error should have expected severity"
    );
    assert_eq!(
        error.is_fatal(),
        matches!(expected_severity, ErrorSeverity::Fatal),
        "is_fatal() should match severity"
    );
}

#[test]
fn test_fatal_error_classification() {
    // Test that auth/rate-limit errors are classified as fatal
    let auth_error = RecordingError::http_status(403, "https://hibiki-radio.jp/test", "Forbidden");
    verify_error_classification(&auth_error, ErrorSeverity::Fatal);

    let rate_limit = RecordingError::http_status(429, "https://hibiki-radio.jp/test", "Rate limited");
    verify_error_classification(&rate_limit, ErrorSeverity::Fatal);

    // HTML parsing errors are fatal (site structure changed)
    let html_error = RecordingError::html_parsing("https://example.com", "Parse failed");
    verify_error_classification(&html_error, ErrorSeverity::Fatal);

    // Memory limit exceeded is fatal
    let memory_error = RecordingError::memory_limit(600_000_000, 512_000_000);
    verify_error_classification(&memory_error, ErrorSeverity::Fatal);

    // Config errors are fatal
    let config_error = RecordingError::ConfigError("Missing config file".to_string());
    verify_error_classification(&config_error, ErrorSeverity::Fatal);
}

#[test]
fn test_recoverable_error_classification() {
    // Server errors are recoverable
    let server_error =
        RecordingError::http_status(500, "https://example.com", "Internal Server Error");
    verify_error_classification(&server_error, ErrorSeverity::Recoverable);

    // I/O errors are recoverable
    let io_error = RecordingError::Io(std::io::Error::new(
        std::io::ErrorKind::ConnectionReset,
        "Connection reset",
    ));
    verify_error_classification(&io_error, ErrorSeverity::Recoverable);

    // Network errors are recoverable
    let network_error = RecordingError::NetworkError("Temporary network failure".to_string());
    verify_error_classification(&network_error, ErrorSeverity::Recoverable);
}

#[test]
fn test_error_handler_returns_exit_code_for_fatal_errors() {
    // Fatal errors should return an exit code
    let fatal_cases = [
        RecordingError::http_status(403, "https://example.com", "Forbidden"),
        RecordingError::html_parsing("https://example.com", "Parse failed"),
        RecordingError::memory_limit(100_000, 50_000),
        RecordingError::ConfigError("Bad config".to_string()),
    ];

    for error in &fatal_cases {
        let exit_code = handle_recording_error(error);
        assert!(
            exit_code.is_some(),
            "Fatal error should return an exit code: {}",
            error
        );
        // Verify exit code is non-zero
        assert!(
            exit_code.unwrap() != 0,
            "Exit code should be non-zero for fatal errors"
        );
    }
}

#[test]
fn test_error_handler_returns_none_for_recoverable_errors() {
    // Recoverable errors should not force exit
    let recoverable_cases = [
        RecordingError::http_status(500, "https://example.com", "Server error"),
        RecordingError::NetworkError("Timeout".to_string()),
        RecordingError::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "Refused",
        )),
    ];

    for error in &recoverable_cases {
        let exit_code = handle_recording_error(error);
        assert_eq!(
            exit_code, None,
            "Recoverable error should not return exit code: {}",
            error
        );
    }
}

#[test]
fn test_error_messages_are_displayable() {
    // All errors should produce meaningful display output
    let errors = [
        RecordingError::http_status(403, "https://hibiki-radio.jp/program", "Access forbidden"),
        RecordingError::html_parsing("https://example.com/page", "Could not find streaming URL"),
        RecordingError::NetworkError("Connection failed".to_string()),
        RecordingError::ConfigError("Missing field".to_string()),
    ];

    for error in &errors {
        let msg = format!("{error}");
        assert!(
            !msg.is_empty(),
            "Error message should not be empty"
        );
        assert!(
            msg.len() > 10,
            "Error message should be descriptive: {}",
            msg
        );
    }
}

#[test]
fn test_error_source_chain_preservation() {
    // Test that error sources are preserved through the chain
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
    let recording_error = RecordingError::from(io_error);

    // The error should contain the original error information
    let error_string = format!("{recording_error}");
    assert!(
        error_string.contains("File not found") || error_string.contains("I/O"),
        "Source error should be reflected in error message"
    );
}

#[test]
fn test_errors_are_distinguishable() {
    // Different error scenarios should produce different messages
    let error_403 = RecordingError::http_status(403, "https://example.com", "Forbidden");
    let error_404 = RecordingError::http_status(404, "https://example.com", "Not found");
    let error_html = RecordingError::html_parsing("https://example.com", "Parse failed");

    let msg_403 = format!("{error_403}");
    let msg_404 = format!("{error_404}");
    let msg_html = format!("{error_html}");

    assert_ne!(msg_403, msg_404, "Different status codes should produce different messages");
    assert_ne!(
        msg_403, msg_html,
        "Different error types should produce different messages"
    );
}

#[test]
fn test_error_context_in_messages() {
    // Verify that error context (URLs, messages) is preserved in output
    let url = "https://hibiki-radio.jp/program";
    let message = "Access forbidden";

    let error = RecordingError::http_status(403, url, message);
    let error_string = format!("{error}");

    // URL context should be present
    assert!(
        error_string.contains("hibiki-radio.jp"),
        "Error should include URL context"
    );
    // Status code should be present
    assert!(error_string.contains("403"), "Error should include status code");
}

#[test]
fn test_batch_processing_errors() {
    // Simulate batch processing with various error types
    let errors = [
        ("Program 1", RecordingError::http_status(500, "https://example.com/prog1", "Server error")),
        ("Program 2", RecordingError::NetworkError("Timeout".to_string())),
        ("Program 3", RecordingError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        ))),
    ];

    // All errors should be displayable (for logging)
    for (_name, error) in &errors {
        let msg = format!("{error}");
        assert!(
            !msg.is_empty(),
            "Error should produce a message",
        );
    }

    // Count severity types
    let fatal_count = errors.iter().filter(|(_, e)| e.is_fatal()).count();
    assert_eq!(fatal_count, 0, "No fatal errors in this batch");

    let recoverable_count = errors.iter().filter(|(_, e)| !e.is_fatal()).count();
    assert_eq!(recoverable_count, 3, "All should be recoverable");
}

#[test]
fn test_errors_work_in_async_context() {
    // Verify errors can be used in async contexts (require Send/Sync)
    use std::sync::Arc;

    let error = Arc::new(RecordingError::NetworkError("Async network error".to_string()));

    // Should be cloneable and shareable across threads
    let error_clone = Arc::clone(&error);
    assert_eq!(format!("{error}"), format!("{}", error_clone));
    assert_eq!(error.severity(), error_clone.severity());
}

#[test]
fn test_serialization_error_context() {
    // JSON/serialization errors should provide context
    let json_str = "not valid json";
    let result: Result<Vec<i32>, _> = serde_json::from_str(json_str);
    let json_error = RecordingError::JsonError(result.unwrap_err());

    let error_string = format!("{json_error}");
    assert!(
        error_string.contains("JSON") || error_string.contains("json"),
        "Serialization error should indicate the problem type"
    );
    assert_eq!(json_error.severity(), ErrorSeverity::Recoverable);
}

#[test]
fn test_command_error_context() {
    let cmd_error = RecordingError::CommandFailed {
        command: "ffmpeg -i input.mp4 output.mp3".to_string(),
        code: Some(1),
        stderr: "Error decoding input".to_string(),
    };

    let error_string = format!("{cmd_error}");
    // Should contain useful debugging info
    assert!(
        error_string.contains("ffmpeg") || error_string.contains("command"),
        "Command error should include the command name"
    );
    assert_eq!(cmd_error.severity(), ErrorSeverity::Recoverable);
}

#[test]
fn test_timeout_error_context() {
    let timeout = std::time::Duration::from_secs(30);
    let timeout_error = RecordingError::Timeout { timeout };

    let error_string = format!("{timeout_error}");
    assert!(
        error_string.contains("timeout") || error_string.contains("timed out"),
        "Timeout error should indicate timeout occurred"
    );
}
