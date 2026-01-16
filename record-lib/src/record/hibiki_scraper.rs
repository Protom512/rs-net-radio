//! HTML parsing and streaming URL extraction for Hibiki Radio.
//!
//! This module provides functionality to scrape Hibiki Radio pages
//! and extract streaming URLs from HTML content.

use crate::utils::{handle_html_parsing_error, html_parsing_error, RecordError};
use reqwest::blocking::Client;
use reqwest::header::{USER_AGENT, REFERER};
use scraper::{Html, Selector};
use tracing::{debug, error, info, warn};

/// Hibiki radio scraper for extracting streaming URLs.
pub struct HibikiScraper {
    /// HTTP client for making requests.
    client: Client,
    /// Base URL for Hibiki Radio.
    base_url: String,
}

impl HibikiScraper {
    /// Creates a new HibikiScraper.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn new() -> Result<Self, RecordError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(RecordError::Reqwest)?;

        Ok(Self {
            client,
            base_url: "https://hibiki-radio.jp".to_string(),
        })
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

        // Fetch the page with proper headers
        let response = self
            .client
            .get(page_url)
            .header(
                USER_AGENT,
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
            )
            .header(REFERER, &self.base_url)
            .send()
            .map_err(RecordError::Reqwest)?;

        // Check HTTP status
        crate::utils::check_http_status(&response, page_url)?;

        // Get HTML content
        let html_text = response
            .text()
            .map_err(RecordError::Reqwest)?;

        debug!("Successfully fetched HTML from {}", page_url);

        // Parse HTML
        let document = Html::parse_document(&html_text);

        // Try to find streaming URL in various possible locations
        if let Some(url) = self.try_extract_from_script(&document)? {
            info!("Successfully extracted streaming URL from script tags");
            return Ok(url);
        }

        if let Some(url) = self.try_extract_from_data_attribute(&document)? {
            info!("Successfully extracted streaming URL from data attributes");
            return Ok(url);
        }

        if let Some(url) = self.try_extract_from_iframe(&document)? {
            info!("Successfully extracted streaming URL from iframe");
            return Ok(url);
        }

        // If no URL found, this might be a site structure change
        let error_msg = format!(
            "Could not extract streaming URL from page. The site structure may have changed. URL: {}",
            page_url
        );
        error!("{}", error_msg);

        // Use the HTML parsing error handler which will exit with code 3
        Err(handle_html_parsing_error(page_url, &error_msg))
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
        let script_selector = Selector::parse("script").map_err(|e| {
            html_parsing_error("", &format!("Failed to parse script selector: {}", e))
        })?;

        for element in document.select(&script_selector) {
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
        // Try various data attributes that might contain the streaming URL
        let selectors = [
            "[data-streaming-url]",
            "[data-video-url]",
            "[data-movie-url]",
            "[data-url]",
        ];

        for selector_str in &selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    if let Some(url) = element.value().attr("data-streaming-url")
                        .or(element.value().attr("data-video-url"))
                        .or(element.value().attr("data-movie-url"))
                        .or(element.value().attr("data-url"))
                    {
                        if self.is_valid_streaming_url(url) {
                            debug!("Found URL in data attribute: {}", url);
                            return Ok(Some(url.to_string()));
                        }
                    }
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
        let iframe_selector = Selector::parse("iframe").map_err(|e| {
            html_parsing_error("", &format!("Failed to parse iframe selector: {}", e))
        })?;

        for element in document.select(&iframe_selector) {
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
            r#"https?://[^"'<>]+\.(?:m3u8|mp4|ts)[^"'<>]*"#,  // Direct streaming URLs
            r#""url"\s*:\s*"([^"]+)""#,                           // JSON "url" field
            r#""streamingUrl"\s*:\s*"([^"]+)""#,                 // JSON "streamingUrl" field
            r#""videoUrl"\s*:\s*"([^"]+)""#,                     // JSON "videoUrl" field
            r#"(?:src|href)\s*=\s*"([^"]+\.(?:m3u8|mp4|ts)[^"]*)"#,  // src/href attributes
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(text) {
                    if let Some(url) = captures.get(1) {
                        let url_str = url.as_str();
                        if self.is_valid_streaming_url(url_str) {
                            return Some(url_str.to_string());
                        }
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
        let valid_patterns = [
            ".m3u8",
            ".mp4",
            ".ts",
            "streaming",
            "video",
            "manifest",
        ];

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
            .header(
                USER_AGENT,
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
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
        let scraper = HibikiScraper::new()
            .expect("Failed to create scraper for test");

        assert!(scraper.is_valid_streaming_url("https://example.com/stream.m3u8"));
        assert!(scraper.is_valid_streaming_url("http://example.com/video.mp4"));
        assert!(scraper.is_valid_streaming_url("https://example.com/streaming"));
        assert!(!scraper.is_valid_streaming_url("ftp://example.com/file.m3u8"));
        assert!(!scraper.is_valid_streaming_url("not a url"));
    }
}
