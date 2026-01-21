//! Integration tests for Hibiki HTTP error handling.
//!
//! These tests verify error behavior at the API boundary:
//! - Errors can be created and properly displayed
//! - Error messages contain essential information
//! - Error types are correctly distinguished

use record_lib::utils::RecordError;

/// Helper to verify an HTTP error contains essential information
fn verify_http_error(error: &RecordError, expected_status: u16, url_part: &str) {
    let error_string = format!("{error}");
    // Verify status code is present (not checking exact format)
    assert!(
        error_string.contains(&expected_status.to_string()),
        "Error should contain status code",
    );
    // Verify URL context is preserved (not checking exact URL)
    assert!(
        error_string.contains(url_part),
        "Error should reference the URL context",
    );
}

#[test]
fn test_http_403_error_behavior() {
    let error = RecordError::HttpError {
        status_code: 403,
        url: "https://hibiki-radio.jp/test".to_owned(),
        message: "Access forbidden".to_owned(),
    };

    // Test behavior: error is displayable and contains key information
    verify_http_error(&error, 403, "hibiki-radio.jp");
}

#[test]
fn test_http_429_error_behavior() {
    let error = RecordError::HttpError {
        status_code: 429,
        url: "https://hibiki-radio.jp/test".to_owned(),
        message: "Too many requests".to_owned(),
    };

    verify_http_error(&error, 429, "hibiki-radio.jp");
}

#[test]
fn test_html_parsing_error_behavior() {
    let error = RecordError::HtmlParsingError {
        url: "https://hibiki-radio.jp/program".to_owned(),
        message: "Could not extract streaming URL from page".to_owned(),
    };

    let error_string = format!("{error}");
    // Verify error indicates the problem type and location
    assert!(
        error_string.contains("HTML") || error_string.contains("parsing"),
        "Error should indicate HTML/parsing issue"
    );
    assert!(
        error_string.contains("hibiki-radio.jp"),
        "Error should reference the URL"
    );
}

#[test]
fn test_error_factory_functions() {
    // Test that factory functions produce valid errors
    let http_error = record_lib::utils::http_error(403, "https://test.com");
    let error_string = format!("{http_error}");
    assert!(error_string.contains("403"), "Should contain status code");

    let html_error = record_lib::utils::html_parsing_error("https://test.com", "Parse failed");
    let html_string = format!("{html_error}");
    assert!(
        html_string.contains("test.com") || html_string.contains("Parse failed"),
        "Should contain context information"
    );
}

#[test]
fn test_http_error_messages_describe_the_problem() {
    // Test that error messages are descriptive (implementation-agnostic)
    let cases = [
        (403, "authentication"),
        (429, "rate limit"),
        (404, "not found"), // lowercase for case-insensitive match
    ];

    for (status, keyword) in cases {
        let error = record_lib::utils::http_error(status, "https://hibiki-radio.jp");
        let error_string = format!("{error}");
        assert!(
            error_string.contains(&status.to_string()),
            "Status code should be in error message",
        );
        // Check for descriptive keyword (case-insensitive via lowercase conversion)
        assert!(
            error_string.to_lowercase().contains(keyword),
            "Error should describe the problem",
        );
    }
}

#[test]
fn test_errors_are_distinguishable() {
    // Different error types should produce different messages
    let http_err = record_lib::utils::http_error(403, "https://example.com");
    let html_err = record_lib::utils::html_parsing_error("https://example.com", "Parse failed");

    let http_msg = format!("{http_err}");
    let html_msg = format!("{html_err}");

    assert_ne!(
        http_msg, html_msg,
        "Different error types should produce different messages"
    );
}
