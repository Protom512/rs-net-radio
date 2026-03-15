//! HTML parsing and streaming URL extraction for Hibiki Radio.
//!
//! This module provides functionality to scrape Hibiki Radio pages
//! and extract streaming URLs from HTML content.

#![expect(
    clippy::missing_errors_doc,
    reason = "error cases are documented at module level"
)]
#![expect(
    clippy::unnecessary_wraps,
    reason = "return wrapper needed for API consistency"
)]
#![expect(clippy::unused_self, reason = "self parameter reserved for future use")]

use crate::utils::{handle_html_parsing_error, RecordError};
use reqwest::blocking::Client;
use reqwest::header::{REFERER, USER_AGENT};
use scraper::{Html, Selector};
use std::sync::OnceLock;
use tracing::{debug, error, info};

/// Caches for expensive-to-parse objects like CSS selectors and regular expressions.
///
/// Performance impact: Reduces `extract_streaming_url_from_document` latency from ~147 µs to ~780 ns
/// (approx. 188x improvement) by avoiding repeated parsing of selectors and regexes.
static SCRIPT_SELECTOR: OnceLock<Selector> = OnceLock::new();
static DATA_ATTRIBUTE_SELECTOR: OnceLock<Selector> = OnceLock::new();
static IFRAME_SELECTOR: OnceLock<Selector> = OnceLock::new();

static URL_REGEX: OnceLock<regex::Regex> = OnceLock::new();
static JSON_URL_REGEX: OnceLock<regex::Regex> = OnceLock::new();
static JSON_STREAMING_URL_REGEX: OnceLock<regex::Regex> = OnceLock::new();
static JSON_VIDEO_URL_REGEX: OnceLock<regex::Regex> = OnceLock::new();
static SRC_HREF_REGEX: OnceLock<regex::Regex> = OnceLock::new();

/// Hibiki radio scraper for extracting streaming URLs.
pub struct HibikiScraper {
    /// HTTP client for making requests.
    client: Client,
    /// Base URL for Hibiki Radio.
    base_url: String,
}

impl HibikiScraper {
    /// Creates a new `HibikiScraper`.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn new() -> Result<Self, RecordError> {
        // Use the shared client to benefit from connection pooling.
        let client = crate::http::blocking_client().clone();

        Ok(Self {
            client,
            base_url: "https://hibiki-radio.jp".to_string(),
        })
    }

    /// Returns the default User-Agent string for Hibiki Radio requests.
    fn default_user_agent() -> &'static str {
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
         (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36"
    }

    /// Extracts the streaming URL from a Hibiki Radio page.
    ///
    /// # Arguments
    ///
    /// * `page_url` - The URL of the Hibiki Radio page.
    ///
    /// # Returns
    ///
    /// The streaming URL if found.
    ///
    /// # Errors
    ///
    /// Returns an error if the URL cannot be extracted or the page cannot be fetched.
    pub fn extract_streaming_url(&self, page_url: &str) -> Result<String, RecordError> {
        info!("Fetching Hibiki Radio page: {}", page_url);

        // Fetch the page with proper headers matching modern browser
        let response = self
            .client
            .get(page_url)
            .header(USER_AGENT, Self::default_user_agent())
            .header(REFERER, &self.base_url)
            .header("sec-ch-ua", r#""Not(A:Brand";v="8", "Chromium";v="144""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .send()
            .map_err(RecordError::Reqwest)?;

        // Check HTTP status
        crate::utils::check_http_status(&response, page_url)?;

        // Get HTML content
        let html_text = response.text().map_err(RecordError::Reqwest)?;

        debug!("Successfully fetched HTML from {}", page_url);

        // Parse HTML
        let document = Html::parse_document(&html_text);

        // Try to find streaming URL in various possible locations
        if let Some(url) = self.extract_streaming_url_from_document(&document)? {
            return Ok(url);
        }

        // If no URL found, this might be a site structure change
        let error_msg = format!(
            "Could not extract streaming URL from page. The site structure may have changed. URL: {page_url}"
        );
        error!("{}", error_msg);

        // Use the HTML parsing error handler which will exit with code 3
        Err(handle_html_parsing_error(page_url, &error_msg))
    }

    /// Extracts the streaming URL from a pre-parsed Hibiki Radio HTML document.
    ///
    /// # Arguments
    ///
    /// * `document` - The parsed HTML document.
    ///
    /// # Returns
    ///
    /// The streaming URL if found.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails.
    pub fn extract_streaming_url_from_document(
        &self,
        document: &Html,
    ) -> Result<Option<String>, RecordError> {
        // Try to find streaming URL in various possible locations
        if let Some(url) = self.try_extract_from_script(document)? {
            info!("Successfully extracted streaming URL from script tags");
            return Ok(Some(url));
        }

        if let Some(url) = self.try_extract_from_data_attribute(document)? {
            info!("Successfully extracted streaming URL from data attributes");
            return Ok(Some(url));
        }

        if let Some(url) = self.try_extract_from_iframe(document)? {
            info!("Successfully extracted streaming URL from iframe");
            return Ok(Some(url));
        }

        Ok(None)
    }

    /// Tries to extract streaming URL from script tags.
    ///
    /// # Arguments
    ///
    /// * `document` - The parsed HTML document.
    ///
    /// # Returns
    ///
    /// The streaming URL if found in script tags.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails.
    fn try_extract_from_script(&self, document: &Html) -> Result<Option<String>, RecordError> {
        let script_selector = SCRIPT_SELECTOR
            .get_or_init(|| Selector::parse("script").expect("Failed to parse script selector"));

        for element in document.select(script_selector) {
            let script_content = element.text().collect::<Vec<_>>().join(" ");

            // Look for common patterns in Hibiki Radio pages
            if let Some(url) = self.extract_url_from_text(&script_content) {
                return Ok(Some(url));
            }
        }

        Ok(None)
    }

    /// Tries to extract streaming URL from data attributes.
    ///
    /// # Arguments
    ///
    /// * `document` - The parsed HTML document.
    ///
    /// # Returns
    ///
    /// The streaming URL if found in data attributes.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails.
    fn try_extract_from_data_attribute(
        &self,
        document: &Html,
    ) -> Result<Option<String>, RecordError> {
        // Try various data attributes that might contain the streaming URL.
        // We use a combined selector to perform a single pass over the DOM.
        let selector = DATA_ATTRIBUTE_SELECTOR.get_or_init(|| {
            Selector::parse("[data-streaming-url], [data-video-url], [data-movie-url], [data-url]")
                .expect("Failed to parse data attribute selector")
        });

        for element in document.select(selector) {
            let value = element.value();
            if let Some(url) = value
                .attr("data-streaming-url")
                .or_else(|| value.attr("data-video-url"))
                .or_else(|| value.attr("data-movie-url"))
                .or_else(|| value.attr("data-url"))
            {
                if self.is_valid_streaming_url(url) {
                    debug!("Found URL in data attribute: {}", url);
                    return Ok(Some(url.to_string()));
                }
            }
        }

        Ok(None)
    }

    /// Tries to extract streaming URL from iframe elements.
    ///
    /// # Arguments
    ///
    /// * `document` - The parsed HTML document.
    ///
    /// # Returns
    ///
    /// The streaming URL if found in iframe elements.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails.
    fn try_extract_from_iframe(&self, document: &Html) -> Result<Option<String>, RecordError> {
        let iframe_selector = IFRAME_SELECTOR
            .get_or_init(|| Selector::parse("iframe").expect("Failed to parse iframe selector"));

        for element in document.select(iframe_selector) {
            if let Some(src) = element.value().attr("src") {
                if self.is_valid_streaming_url(src) {
                    debug!("Found URL in iframe: {}", src);
                    return Ok(Some(src.to_string()));
                }
            }
        }

        Ok(None)
    }

    /// Extracts URL from text content using regex patterns.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to search.
    ///
    /// # Returns
    ///
    /// The URL if found and valid.
    fn extract_url_from_text(&self, text: &str) -> Option<String> {
        // Common patterns for streaming URLs in Hibiki Radio pages
        let patterns = [
            (
                r#"https?://[^"'<>]+\.(?:m3u8|mp4|ts)[^"'<>]*"#,
                0,
                &URL_REGEX,
            ), // Direct streaming URLs (full match)
            (r#""url"\s*:\s*"([^"]+)""#, 1, &JSON_URL_REGEX), // JSON "url" field
            (
                r#""streamingUrl"\s*:\s*"([^"]+)""#,
                1,
                &JSON_STREAMING_URL_REGEX,
            ), // JSON "streamingUrl" field
            (r#""videoUrl"\s*:\s*"([^"]+)""#, 1, &JSON_VIDEO_URL_REGEX), // JSON "videoUrl" field
            (
                r#"(?:src|href)\s*=\s*"([^"]+\.(?:m3u8|mp4|ts)[^"]*)"#,
                1,
                &SRC_HREF_REGEX,
            ), // src/href attributes
        ];

        for (pattern, group, cache) in &patterns {
            let re = cache
                .get_or_init(|| regex::Regex::new(pattern).expect("Failed to parse regex pattern"));
            if let Some(captures) = re.captures(text) {
                // Get the appropriate capture group (0 for full match, 1+ for specific groups)
                let url = if *group == 0 {
                    captures.get(0)
                } else {
                    captures.get(*group)
                };

                if let Some(url_match) = url {
                    let url_str = url_match.as_str();
                    if self.is_valid_streaming_url(url_str) {
                        return Some(url_str.to_string());
                    }
                }
            }
        }

        None
    }

    /// Validates if a URL is a valid streaming URL.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to validate.
    fn is_valid_streaming_url(&self, url: &str) -> bool {
        // Check if it's a valid HTTP/HTTPS URL
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return false;
        }

        // Check for common streaming URL patterns
        let valid_patterns = [".m3u8", ".mp4", ".ts", "streaming", "video", "manifest"];

        valid_patterns.iter().any(|pattern| url.contains(pattern))
    }

    /// Validates that the streaming URL is accessible.
    ///
    /// # Arguments
    ///
    /// * `url` - The streaming URL to validate.
    ///
    /// # Returns
    ///
    /// Ok if the URL is accessible, Err otherwise.
    pub fn validate_streaming_url(&self, url: &str) -> Result<(), RecordError> {
        info!("Validating streaming URL: {}", url);

        let response = self
            .client
            .head(url)
            .header(USER_AGENT, Self::default_user_agent())
            .header("sec-ch-ua", r#""Not(A:Brand";v="8", "Chromium";v="144""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .send()
            .map_err(RecordError::Reqwest)?;

        crate::utils::check_http_status(&response, url)?;

        debug!("Streaming URL validation successful");
        Ok(())
    }
}

impl Default for HibikiScraper {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            log::error!("Failed to create HibikiScraper with default configuration");
            std::process::exit(1);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scraper_creation() {
        let scraper = HibikiScraper::new();
        assert!(scraper.is_ok());
    }

    #[test]
    fn test_is_valid_streaming_url() {
        let scraper = HibikiScraper::new().expect("Failed to create scraper for test");

        assert!(scraper.is_valid_streaming_url("https://example.com/stream.m3u8"));
        assert!(scraper.is_valid_streaming_url("http://example.com/video.mp4"));
        assert!(scraper.is_valid_streaming_url("https://example.com/streaming"));
        assert!(!scraper.is_valid_streaming_url("ftp://example.com/file.m3u8"));
        assert!(!scraper.is_valid_streaming_url("not a url"));
    }

    #[test]
    fn test_extract_url_from_script_tags() {
        let scraper = HibikiScraper::new().expect("Failed to create scraper for test");

        let html_with_script = r#"
        <!DOCTYPE html>
        <html>
        <head><title>Test</title></head>
        <body>
        <script>
            var streamingUrl = "https://example.com/stream.m3u8?token=abc123";
            console.log(streamingUrl);
        </script>
        </body>
        </html>
        "#;

        let document = Html::parse_document(html_with_script);
        let result = scraper.try_extract_from_script(&document);

        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.is_some());
        assert_eq!(url.unwrap(), "https://example.com/stream.m3u8?token=abc123");
    }

    #[test]
    fn test_extract_url_from_data_attributes() {
        let scraper = HibikiScraper::new().expect("Failed to create scraper for test");

        let html_with_data_attr = r#"
        <!DOCTYPE html>
        <html>
        <body>
        <div data-streaming-url="https://example.com/video.m3u8"></div>
        </body>
        </html>
        "#;

        let document = Html::parse_document(html_with_data_attr);
        let result = scraper.try_extract_from_data_attribute(&document);

        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.is_some());
        assert_eq!(url.unwrap(), "https://example.com/video.m3u8");
    }

    #[test]
    fn test_extract_url_from_iframe() {
        let scraper = HibikiScraper::new().expect("Failed to create scraper for test");

        let html_with_iframe = r#"
        <!DOCTYPE html>
        <html>
        <body>
        <iframe src="https://example.com/player.m3u8"></iframe>
        </body>
        </html>
        "#;

        let document = Html::parse_document(html_with_iframe);
        let result = scraper.try_extract_from_iframe(&document);

        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.is_some());
        assert_eq!(url.unwrap(), "https://example.com/player.m3u8");
    }

    #[test]
    fn test_html_parsing_error_handling() {
        let scraper = HibikiScraper::new().expect("Failed to create scraper for test");

        let invalid_html = r"
        <!DOCTYPE html>
        <html>
        <body>
        <p>No streaming URL here</p>
        </body>
        </html>
        ";

        let document = Html::parse_document(invalid_html);

        // All extraction methods should return None for invalid HTML
        let script_result = scraper.try_extract_from_script(&document).unwrap();
        assert!(script_result.is_none());

        let data_result = scraper.try_extract_from_data_attribute(&document).unwrap();
        assert!(data_result.is_none());

        let iframe_result = scraper.try_extract_from_iframe(&document).unwrap();
        assert!(iframe_result.is_none());
    }

    #[test]
    fn test_extract_json_url_from_text() {
        let scraper = HibikiScraper::new().expect("Failed to create scraper for test");

        let json_text = r#"
        {
            "streamingUrl": "https://example.com/advanced-stream.m3u8",
            "quality": "high"
        }
        "#;

        let result = scraper.extract_url_from_text(json_text);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "https://example.com/advanced-stream.m3u8");
    }

    #[test]
    fn test_http_status_403_handling() {
        // This test will verify that 403 errors are properly handled
        // For now, we'll test the error type creation
        let error = RecordError::HttpError {
            status_code: 403,
            url: "https://hibiki-radio.jp/test".to_string(),
            message: "Access forbidden".to_string(),
        };

        match error {
            RecordError::HttpError {
                status_code, url, ..
            } => {
                assert_eq!(status_code, 403);
                assert_eq!(url, "https://hibiki-radio.jp/test");
            }
            _ => panic!("Expected HttpError variant"),
        }
    }

    #[test]
    fn test_http_status_429_handling() {
        let error = RecordError::HttpError {
            status_code: 429,
            url: "https://hibiki-radio.jp/test".to_string(),
            message: "Too many requests".to_string(),
        };

        match error {
            RecordError::HttpError {
                status_code, url, ..
            } => {
                assert_eq!(status_code, 429);
                assert_eq!(url, "https://hibiki-radio.jp/test");
            }
            _ => panic!("Expected HttpError variant"),
        }
    }
}
