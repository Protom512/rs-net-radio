//! Integration tests for Hibiki radio functionality.
//!
//! These tests verify:
//! - Mock HTTP server interactions for 403/429 errors
//! - Mock HTML parsing and streaming URL extraction
//! - Error handling integration
//! - Header validation (User-Agent, Referer)

use mockito::{Mock, ServerGuard};
use record_lib::record::hibiki_scraper::HibikiScraper;
use record_lib::utils::{RecordError, check_http_status, http_error, html_parsing_error};

/// Creates a mock server that returns a specific HTTP status code
fn create_status_mock(server: &mut ServerGuard, status_code: usize, path: &str) -> Mock {
    return server
        .mock("GET", path)
        .with_status(status_code)
        .with_header("content-type", "text/html")
        .with_body("<html><body>Test response</body></html>")
        .create()
}

/// Creates a mock server that validates User-Agent and Referer headers
fn create_header_validation_mock(server: &mut ServerGuard, path: &str) -> Mock {
    return server
        .mock("GET", path)
        .match_header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .match_header("referer", "https://hibiki-radio.jp")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(r#"
            <html>
            <head><title>Test</title></head>
            <body>
            <div data-streaming-url="https://example.com/stream.m3u8"></div>
            </body>
            </html>
        "#)
        .create()
}

/// Creates a mock server with valid HTML containing streaming URL
fn create_html_with_streaming_url(server: &mut ServerGuard, path: &str, url: &str) -> Mock {
    let html = format!(r#"
        <!DOCTYPE html>
        <html>
        <head><title>Hibiki Radio Test</title></head>
        <body>
        <div data-streaming-url="{url}"></div>
        </body>
        </html>
    "#);

    return server
        .mock("GET", path)
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(&html)
        .create()
}

/// Creates a mock server with HTML containing script tag with streaming URL
fn create_html_with_script_url(server: &mut ServerGuard, path: &str, url: &str) -> Mock {
    let html = format!(r#"
        <!DOCTYPE html>
        <html>
        <head><title>Hibiki Radio Test</title></head>
        <body>
        <script>
            var streamingUrl = "{url}";
            console.log(streamingUrl);
        </script>
        </body>
        </html>
    "#);

    return server
        .mock("GET", path)
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(&html)
        .create()
}

/// Creates a mock server with iframe containing streaming URL
fn create_html_with_iframe(server: &mut ServerGuard, path: &str, url: &str) -> Mock {
    let html = format!(r#"
        <!DOCTYPE html>
        <html>
        <head><title>Hibiki Radio Test</title></head>
        <body>
        <iframe src="{url}"></iframe>
        </body>
        </html>
    "#);

    return server
        .mock("GET", path)
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(&html)
        .create()
}

#[test]
#[ignore] // TODO: Fix hanging test
fn test_hibiki_scraper_with_valid_html() {
    let mut server = mockito::Server::new();
    let _mock = create_html_with_streaming_url(
        &mut server,
        "/test-program",
        "https://stream.example.com/program.m3u8"
    );

    let scraper = HibikiScraper::new().expect("Failed to create scraper");
    let url = format!("{}/test-program", server.url());

    let result = scraper.extract_streaming_url(&url);

    assert!(result.is_ok(), "Failed to extract streaming URL: {:?}", result.err());
    let streaming_url = result.unwrap();
    assert_eq!(streaming_url, "https://stream.example.com/program.m3u8");
}

#[test]
#[ignore] // TODO: Fix hanging test
fn test_hibiki_scraper_with_script_tag() {
    let mut server = mockito::Server::new();
    let _mock = create_html_with_script_url(
        &mut server,
        "/test-script",
        "https://stream.example.com/script-stream.m3u8"
    );

    let scraper = HibikiScraper::new().expect("Failed to create scraper");
    let url = format!("{}/test-script", server.url());

    let result = scraper.extract_streaming_url(&url);

    assert!(result.is_ok(), "Failed to extract streaming URL from script: {:?}", result.err());
    let streaming_url = result.unwrap();
    assert_eq!(streaming_url, "https://stream.example.com/script-stream.m3u8");
}

#[test]
#[ignore] // TODO: Fix hanging test
fn test_hibiki_scraper_with_iframe() {
    let mut server = mockito::Server::new();
    let _mock = create_html_with_iframe(
        &mut server,
        "/test-iframe",
        "https://stream.example.com/iframe-stream.m3u8"
    );

    let scraper = HibikiScraper::new().expect("Failed to create scraper");
    let url = format!("{}/test-iframe", server.url());

    let result = scraper.extract_streaming_url(&url);

    assert!(result.is_ok(), "Failed to extract streaming URL from iframe: {:?}", result.err());
    let streaming_url = result.unwrap();
    assert_eq!(streaming_url, "https://stream.example.com/iframe-stream.m3u8");
}

#[test]
#[ignore] // TODO: Fix hanging test
fn test_hibiki_scraper_with_html_missing_url() {
    let mut server = mockito::Server::new();
    let _mock = server
        .mock("GET", "/no-url")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body("
            <html>
            <head><title>Test</title></head>
            <body>
            <p>No streaming URL here</p>
            </body>
            </html>
        ")
        .create();

    let scraper = HibikiScraper::new().expect("Failed to create scraper");
    let url = format!("{}/no-url", server.url());

    let result = scraper.extract_streaming_url(&url);

    assert!(result.is_err(), "Should fail when HTML doesn't contain streaming URL");
    match result {
        Err(RecordError::HtmlParsingError { url: _, message }) => {
            assert!(message.contains("Could not extract streaming URL"));
        }
        _ => panic!("Expected HtmlParsingError"),
    }
}

#[test]
fn test_http_status_check_with_403() {
    let mut server = mockito::Server::new();
    let _mock = create_status_mock(&mut server, 403, "/forbidden");

    let client = reqwest::blocking::Client::new();
    let url = format!("{}/forbidden", server.url());
    let _response = client.get(&url).send().expect("Failed to send request");

    // Note: check_http_status will call process::exit(2) for 403
    // We can't actually test that in unit tests, but we can verify the error would be created
    let error = http_error(403, &url);
    match error {
        RecordError::HttpError { status_code, url: error_url, .. } => {
            assert_eq!(status_code, 403);
            assert_eq!(error_url, url);
        }
        _ => panic!("Expected HttpError"),
    }
}

#[test]
fn test_http_status_check_with_429() {
    let mut server = mockito::Server::new();
    let _mock = create_status_mock(&mut server, 429, "/rate-limited");

    let client = reqwest::blocking::Client::new();
    let url = format!("{}/rate-limited", server.url());
    let _response = client.get(&url).send().expect("Failed to send request");

    // Note: check_http_status will call process::exit(2) for 429
    // We can't actually test that in unit tests, but we can verify the error would be created
    let error = http_error(429, &url);
    match error {
        RecordError::HttpError { status_code, url: error_url, .. } => {
            assert_eq!(status_code, 429);
            assert_eq!(error_url, url);
        }
        _ => panic!("Expected HttpError"),
    }
}

#[test]
fn test_http_status_check_with_500() {
    let mut server = mockito::Server::new();
    let _mock = create_status_mock(&mut server, 500, "/server-error");

    let client = reqwest::blocking::Client::new();
    let url = format!("{}/server-error", server.url());
    let response = client.get(&url).send().expect("Failed to send request");

    // 500 errors are recoverable, so check_http_status should return an error instead of exiting
    let result = check_http_status(&response, &url);
    assert!(result.is_err(), "Should return error for 500 status");

    match result {
        Err(RecordError::HttpError { status_code, .. }) => {
            assert_eq!(status_code, 500);
        }
        _ => panic!("Expected HttpError"),
    }
}

#[test]
fn test_http_status_check_with_success() {
    let mut server = mockito::Server::new();
    let _mock = server
        .mock("GET", "/success")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body("<html><body>Success</body></html>")
        .create();

    let client = reqwest::blocking::Client::new();
    let url = format!("{}/success", server.url());
    let response = client.get(&url).send().expect("Failed to send request");

    // Should return Ok for successful status
    let result = check_http_status(&response, &url);
    result.unwrap();
}

#[test]
fn test_user_agent_header_validation() {
    let mut server = mockito::Server::new();
    let _mock = server
        .mock("GET", "/validate-headers")
        .match_header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .match_header("referer", "https://hibiki-radio.jp")
        .with_status(200)
        .with_body("OK")
        .create();

    let client = reqwest::blocking::Client::new();
    let url = format!("{}/validate-headers", server.url());

    let response = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Referer", "https://hibiki-radio.jp")
        .send()
        .expect("Failed to send request");

    assert_eq!(response.status().as_u16(), 200);
}

#[test]
fn test_hibiki_scraper_headers_are_sent() {
    let mut server = mockito::Server::new();
    let _mock = create_header_validation_mock(&mut server, "/check-headers");

    let scraper = HibikiScraper::new().expect("Failed to create scraper");
    let url = format!("{}/check-headers", server.url());

    // This should succeed because the mock validates headers
    let result = scraper.extract_streaming_url(&url);
    assert!(result.is_ok(), "Scraper should send correct headers: {:?}", result.err());
}

#[test]
#[ignore] // TODO: Fix hanging test
fn test_hibiki_scraper_various_html_structures() {
    let test_cases = vec![
        // Data attribute
        (r#"<div data-streaming-url="https://test1.com/stream.m3u8"></div>"#, "https://test1.com/stream.m3u8"),
        // Script tag with variable
        (r#"<script>var url = "https://test2.com/stream.m3u8";</script>"#, "https://test2.com/stream.m3u8"),
        // Iframe src
        (r#"<iframe src="https://test3.com/stream.m3u8"></iframe>"#, "https://test3.com/stream.m3u8"),
        // JSON in script
        (r#"<script>{"streamingUrl": "https://test4.com/stream.m3u8"}</script>"#, "https://test4.com/stream.m3u8"),
    ];

    for (html_snippet, expected_url) in test_cases {
        let mut server = mockito::Server::new();
        let html = format!("<!DOCTYPE html><html><body>{html_snippet}</body></html>");

        let _mock = server
            .mock("GET", "/test")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(&html)
            .create();

        let scraper = HibikiScraper::new().expect("Failed to create scraper");
        let url = format!("{}/test", server.url());

        let result = scraper.extract_streaming_url(&url);
        assert!(
            result.is_ok(),
            "Failed to extract URL from HTML: {} - Error: {:?}",
            html_snippet,
            result.err()
        );

        let extracted_url = result.unwrap();
        assert_eq!(
            extracted_url, expected_url,
            "URL mismatch for HTML: {html_snippet}"
        );
    }
}

#[test]
fn test_html_parsing_error_handling() {
    let error = html_parsing_error("https://hibiki-radio.jp/program", "No streaming URL found");

    // Check error properties via Display instead of moving
    let error_string = format!("{error}");
    assert!(error_string.contains("HTML parsing error"));
    assert!(error_string.contains("https://hibiki-radio.jp/program"));

    // Now we can move the error in the match
    match error {
        RecordError::HtmlParsingError { url, message } => {
            assert_eq!(url, "https://hibiki-radio.jp/program");
            assert_eq!(message, "No streaming URL found");
        }
        _ => panic!("Expected HtmlParsingError"),
    }
}

#[test]
#[ignore] // TODO: Fix hanging test
fn test_hibiki_scraper_with_malformed_html() {
    let mut server = mockito::Server::new();
    let _mock = server
        .mock("GET", "/malformed")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body("<div><p>Unclosed tags</div>")
        .create();

    let scraper = HibikiScraper::new().expect("Failed to create scraper");
    let url = format!("{}/malformed", server.url());

    // Scraper should handle malformed HTML gracefully
    let result = scraper.extract_streaming_url(&url);

    // Should fail but not panic
    result.unwrap_err();
}

#[test]
#[ignore] // TODO: Fix this test - it hangs due to mock server conflicts
fn test_hibiki_scraper_url_validation() {
    // Test URL validation indirectly by checking if URLs are extracted correctly
    let mut server = mockito::Server::new();

    // Test with various URL patterns
    let valid_urls = vec![
        ("https://example.com/stream.m3u8", "m3u8"),
        ("http://example.com/video.mp4", "mp4"),
        ("https://example.com/streaming", "streaming"),
        ("https://example.com/video.ts", "ts"),
        ("https://example.com/manifest", "manifest"),
    ];

    for (i, (url, _desc)) in valid_urls.iter().enumerate() {
        let html = format!(r#"
            <!DOCTYPE html>
            <html>
            <body>
            <div data-streaming-url="{url}"></div>
            </body>
            </html>
        "#);

        let path = format!("/validate-{}", i);
        let _mock = server
            .mock("GET", path.as_str())
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(&html)
            .create();

        let scraper = HibikiScraper::new().expect("Failed to create scraper");
        let test_url = format!("{}{}", server.url(), path);

        let result = scraper.extract_streaming_url(&test_url);
        assert!(result.is_ok(), "Should successfully extract URL: {}", url);
        let extracted = result.unwrap();
        assert_eq!(extracted, *url, "Extracted URL should match expected");
    }

    // Test with invalid URL pattern
    let invalid_html = r#"
        <!DOCTYPE html>
        <html>
        <body>
        <div data-streaming-url="ftp://example.com/file.m3u8"></div>
        </body>
        </html>
    "#;

    let _mock = server
        .mock("GET", "/invalid")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(invalid_html)
        .create();

    let scraper = HibikiScraper::new().expect("Failed to create scraper");
    let test_url = format!("{}/invalid", server.url());

    // Should fail because ftp:// is not a valid streaming URL
    let result = scraper.extract_streaming_url(&test_url);
    result.unwrap_err();
}

#[test]
#[ignore] // TODO: Fix hanging test
fn test_hibiki_scraper_with_network_error() {
    // Use a non-routable IP to simulate network error
    let scraper = HibikiScraper::new().expect("Failed to create scraper");
    let url = "http://192.0.2.1/test"; // TEST-NET-1, should not be routable

    let result = scraper.extract_streaming_url(url);

    // Should fail with a network error
    assert!(result.is_err(), "Should fail with network error");
    match result {
        Err(RecordError::Reqwest(_)) => {
            // Expected
        }
        _ => {
            panic!("Expected Reqwest error for network failure, got: {:?}", result);
        }
    }
}
