//! Domain types with newtype pattern for type safety.
//!
//! This module provides strongly-typed wrappers around primitive types
//! to prevent mixing similar values and improve code clarity.

use std::fmt;
use std::ops::Deref;
use std::path::PathBuf;

/// A program title or name.
///
/// Wraps a String to prevent mixing with other string types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProgramTitle(String);

impl ProgramTitle {
    /// Creates a new ProgramTitle.
    #[must_use]
    pub fn new(title: String) -> Self {
        Self(title)
    }

    /// Gets the inner string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for ProgramTitle {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for ProgramTitle {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ProgramTitle {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<ProgramTitle> for String {
    fn from(value: ProgramTitle) -> Self {
        value.0
    }
}

impl fmt::Display for ProgramTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A streaming URL.
///
/// Wraps a String to prevent mixing URLs with other strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StreamingUrl(String);

impl StreamingUrl {
    /// Creates a new StreamingUrl.
    #[must_use]
    pub fn new(url: String) -> Self {
        Self(url)
    }

    /// Gets the inner string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for StreamingUrl {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for StreamingUrl {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for StreamingUrl {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<StreamingUrl> for String {
    fn from(value: StreamingUrl) -> Self {
        value.0
    }
}

/// An output file path for recording.
///
/// Wraps PathBuf to prevent mixing with other paths.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutputPath(PathBuf);

impl OutputPath {
    /// Creates a new OutputPath.
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    /// Gets the inner PathBuf value.
    #[must_use]
    pub fn as_path_buf(&self) -> &PathBuf {
        &self.0
    }

    /// Gets the inner path as &Path.
    #[must_use]
    pub fn as_path(&self) -> &std::path::Path {
        self.0.as_path()
    }
}

impl Deref for OutputPath {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<PathBuf> for OutputPath {
    fn from(value: PathBuf) -> Self {
        Self(value)
    }
}

impl From<&str> for OutputPath {
    fn from(value: &str) -> Self {
        Self(PathBuf::from(value))
    }
}

impl From<OutputPath> for PathBuf {
    fn from(value: OutputPath) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_title() {
        let title = ProgramTitle::new("Test Program".to_string());
        assert_eq!(title.as_str(), "Test Program");
        assert_eq!(&*title, "Test Program");
    }

    #[test]
    fn test_streaming_url() {
        let url = StreamingUrl::new("http://example.com/stream".to_string());
        assert_eq!(url.as_str(), "http://example.com/stream");
        assert_eq!(&*url, "http://example.com/stream");
    }

    #[test]
    fn test_output_path() {
        let path = OutputPath::new(PathBuf::from("/tmp/output.mp3"));
        assert_eq!(path.as_path(), std::path::Path::new("/tmp/output.mp3"));
    }

    #[test]
    fn test_from_conversions() {
        // String conversions
        let title: ProgramTitle = "Test".into();
        assert_eq!(title.as_str(), "Test");

        let url: StreamingUrl = "http://example.com".into();
        assert_eq!(url.as_str(), "http://example.com");

        let path: OutputPath = "/tmp/test.mp3".into();
        assert_eq!(path.as_path(), std::path::Path::new("/tmp/test.mp3"));
    }
}
