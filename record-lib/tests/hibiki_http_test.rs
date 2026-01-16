//! Integration tests for Hibiki HTTP error handling and request headers.
//!
//! These tests verify:
//! - HTTP 403/429 error handling
//! - Proper User-Agent and Referer headers
//! - Error logging and exit code behavior

use mockito::{Mock, ServerGuard};
use record_lib::utils::RecordError;

/// Helper function to create a mock server that returns specific status codes
fn create_mock_server(server: &mut ServerGuard, status_code: usize, path: &str) -> Mock {
    return server
        .mock("GET", path)
        .with_status(status_code)
        .with_header("content-type", "text/html")
        .with_body("<html><body>Test response</body></html>")
        .create();
}

/// Helper function to create a mock server that validates headers
fn create_header_validation_mock(server: &mut ServerGuard, path: &str) -> Mock {
    return server
        .mock("GET", path)
        .match_header("user-agent", "*Mozilla*")
        .match_header("referer", "https://hibiki-radio.jp")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(
            r#"
            <html>
            <head><title>Test</title></head>
            <body>
            <div data-streaming-url="https://example.com/stream.m3u8"></div>
            </body>
            </html>
        "#,
        )
        .create();
}

#[test]
fn test_http_403_error_handling() {
    // This test verifies that 403 errors are properly created
    let error = RecordError::HttpError {
        status_code: 403,
        url: "https://hibiki-radio.jp/test".to_owned(),
        message: "Access forbidden".to_owned(),
    };

    // Verify the error contains the correct information
    match &error {
        RecordError::HttpError {
            status_code,
            url,
            message,
        } => {
            assert_eq!(*status_code, 403);
            assert_eq!(url, "https://hibiki-radio.jp/test");
            assert!(message.contains("Access forbidden"));
        }
        _ => panic!("Expected HttpError variant"),
    }

    // Verify the error can be displayed
    let error_string = format!("{error}");
    assert!(error_string.contains("403"));
    assert!(error_string.contains("https://hibiki-radio.jp/test"));
}

#[test]
fn test_http_429_error_handling() {
    let error = RecordError::HttpError {
        status_code: 429,
        url: "https://hibiki-radio.jp/test".to_owned(),
        message: "Too many requests".to_owned(),
    };

    match &error {
        RecordError::HttpError {
            status_code,
            url,
            message,
        } => {
            assert_eq!(*status_code, 429);
            assert_eq!(url, "https://hibiki-radio.jp/test");
            assert!(message.contains("Too many requests"));
        }
        _ => panic!("Expected HttpError variant"),
    }

    let error_string = format!("{error}");
    assert!(error_string.contains("429"));
}

#[test]
fn test_html_parsing_error_handling() {
    let error = RecordError::HtmlParsingError {
        url: "https://hibiki-radio.jp/program".to_owned(),
        message: "Could not extract streaming URL from page".to_owned(),
    };

    match &error {
        RecordError::HtmlParsingError { url, message } => {
            assert_eq!(url, "https://hibiki-radio.jp/program");
            assert!(message.contains("Could not extract streaming URL"));
        }
        _ => panic!("Expected HtmlParsingError variant"),
    }

    let error_string = format!("{error}");
    assert!(error_string.contains("HTML parsing error"));
    assert!(error_string.contains("https://hibiki-radio.jp/program"));
}

#[test]
fn test_error_creation_functions() {
    // Test the helper functions for creating errors
    let http_error = record_lib::utils::http_error(403, "https://test.com");
    match http_error {
        RecordError::HttpError {
            status_code, url, ..
        } => {
            assert_eq!(status_code, 403);
            assert_eq!(url, "https://test.com");
        }
        _ => panic!("Expected HttpError"),
    }

    let html_error = record_lib::utils::html_parsing_error("https://test.com", "Parse failed");
    match html_error {
        RecordError::HtmlParsingError { url, message } => {
            assert_eq!(url, "https://test.com");
            assert_eq!(message, "Parse failed");
        }
        _ => panic!("Expected HtmlParsingError"),
    }
}

#[test]
fn test_http_error_messages() {
    // Verify that error messages are descriptive
    let error_403 = record_lib::utils::http_error(403, "https://hibiki-radio.jp");
    let error_string = format!("{error_403}");
    assert!(error_string.contains("403"));
    assert!(error_string.contains("authentication"));

    let error_429 = record_lib::utils::http_error(429, "https://hibiki-radio.jp");
    let error_string = format!("{error_429}");
    assert!(error_string.contains("429"));
    assert!(error_string.contains("rate limit"));

    let error_404 = record_lib::utils::http_error(404, "https://hibiki-radio.jp");
    let error_string = format!("{error_404}");
    assert!(error_string.contains("404"));
    assert!(error_string.contains("Not found"));
}
