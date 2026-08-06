use log::debug;
use sanitize_filename as sanitize_filename_crate;
use std::error::Error as StdError;
use std::fmt;
use std::{env, fs, path::Path}; // Alias to avoid conflict
/// Represents an error that can occur during the recording process.
#[derive(Debug)]
pub enum RecordError {
    /// An I/O error.
    Io(std::io::Error),
    /// An error related to environment variables.
    EnvVar(std::env::VarError),
    /// An error from the `reqwest` HTTP client.
    Reqwest(reqwest::Error),
    /// A JSON serialization/deserialization error.
    SerdeJson(serde_json::Error),
    // SerdeXml(serde_xml_rs::Error), // Add if direct usage occurs
    /// An error representing a failure in an external command.
    CommandFailed {
        command: String,
        exit_code: Option<i32>,
        stderr: String,
    },
    /// An error for when the temporary directory cannot be found.
    TempDir,
    /// A catch-all for other types of errors.
    Other(String),
    /// HTTP error with status code and URL
    HttpError {
        status_code: u16,
        url: String,
        message: String,
    },
    /// HTML parsing error
    HtmlParsingError { url: String, message: String },
}

// Manual Clone implementation since reqwest::Error and serde_json::Error don't implement Clone
impl Clone for RecordError {
    fn clone(&self) -> Self {
        match self {
            RecordError::Io(e) => RecordError::Other(format!("IO error: {e}")),
            RecordError::EnvVar(e) => {
                RecordError::Other(format!("Environment variable error: {e}"))
            }
            RecordError::Reqwest(e) => RecordError::Other(format!("Reqwest error: {e}")),
            RecordError::SerdeJson(e) => RecordError::Other(format!("Serde JSON error: {e}")),
            RecordError::CommandFailed {
                command,
                exit_code,
                stderr,
            } => RecordError::CommandFailed {
                command: command.clone(),
                exit_code: *exit_code,
                stderr: stderr.clone(),
            },
            RecordError::TempDir => RecordError::TempDir,
            RecordError::Other(s) => RecordError::Other(s.clone()),
            RecordError::HttpError {
                status_code,
                url,
                message,
            } => RecordError::HttpError {
                status_code: *status_code,
                url: url.clone(),
                message: message.clone(),
            },
            RecordError::HtmlParsingError { url, message } => RecordError::HtmlParsingError {
                url: url.clone(),
                message: message.clone(),
            },
        }
    }
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RecordError::Io(e) => write!(f, "IO error: {e}"),
            RecordError::EnvVar(e) => write!(f, "Environment variable error: {e}"),
            RecordError::Reqwest(e) => write!(f, "Reqwest error: {e}"),
            RecordError::SerdeJson(e) => write!(f, "Serde JSON error: {e}"),
            RecordError::CommandFailed {
                command,
                exit_code,
                stderr,
            } => write!(
                f,
                "Command '{command}' failed with code {exit_code:?}. Stderr: {stderr}"
            ),
            RecordError::TempDir => write!(f, "Failed to get temporary directory path"),
            RecordError::Other(s) => write!(f, "Other error: {s}"),
            RecordError::HttpError {
                status_code,
                url,
                message,
            } => write!(f, "HTTP error {status_code} for {url}: {message}"),
            RecordError::HtmlParsingError { url, message } => {
                write!(f, "HTML parsing error for {url}: {message}")
            }
        }
    }
}

impl StdError for RecordError {}

impl From<std::io::Error> for RecordError {
    fn from(err: std::io::Error) -> RecordError {
        RecordError::Io(err)
    }
}
impl From<std::env::VarError> for RecordError {
    fn from(err: std::env::VarError) -> RecordError {
        RecordError::EnvVar(err)
    }
}
impl From<reqwest::Error> for RecordError {
    fn from(err: reqwest::Error) -> RecordError {
        RecordError::Reqwest(err)
    }
}
impl From<serde_json::Error> for RecordError {
    fn from(err: serde_json::Error) -> RecordError {
        RecordError::SerdeJson(err)
    }
}

const RS_NET_ARCHIVE_PATH_ENV_VAR: &str = "RS_NET_ARCHIVE_PATH";

/// Ensures the archive path for a given service exists and returns it.
/// The path will be "{`RS_NET_ARCHIVE_PATH}/{service_name`}".
///
/// # Errors
///
/// Returns `RecordError` if the environment variable is not set or if directory creation fails.
pub fn ensure_archive_path(service_name: &str) -> Result<String, RecordError> {
    let base_path = env::var(RS_NET_ARCHIVE_PATH_ENV_VAR)?; // Uses From<VarError>

    let service_path_str = format!("{base_path}/{service_name}");
    let service_path = Path::new(&service_path_str);

    if !service_path.is_dir() {
        fs::create_dir_all(service_path)?; // Uses From<io::Error>
        debug!("Created directory: {service_path_str}");
    }
    Ok(service_path_str)
}

/// Sanitizes a filename by replacing characters forbidden by common filesystems.
#[must_use]
pub fn sanitize_filename(filename: &str) -> String {
    sanitize_filename_crate::sanitize(filename)
}

/// Creates an HTTP error from status code and URL.
#[must_use]
pub fn http_error(status_code: u16, url: &str) -> RecordError {
    let message = match status_code {
        403 => "Access forbidden - may need authentication or different headers",
        429 => "Too many requests - rate limit exceeded",
        404 => "Not found",
        500..=599 => "Server error",
        _ => "HTTP request failed",
    };

    RecordError::HttpError {
        status_code,
        url: url.to_string(),
        message: message.to_string(),
    }
}

/// Checks HTTP response status and returns appropriate error if needed.
///
/// # Errors
///
/// Returns `RecordError` if the response status indicates an error (4xx or 5xx),
/// including 403 (forbidden) and 429 (rate limited). The decision to terminate
/// the process on a fatal status belongs to the application boundary, not the
/// library — callers receive a structured `HttpError` and may map it to an exit
/// code there.
pub fn check_http_status(
    response: &reqwest::blocking::Response,
    url: &str,
) -> Result<(), RecordError> {
    let status = response.status();

    if status.is_client_error() || status.is_server_error() {
        let status_code = status.as_u16();
        if status_code == 403 || status_code == 429 {
            log::error!("HTTP {status_code} error for {url}. Access denied or rate limited.");
            log::error!("Please try again later or check your access permissions.");
        }
        return Err(http_error(status_code, url));
    }

    Ok(())
}

/// Creates an HTML parsing error.
#[must_use]
pub fn html_parsing_error(url: &str, message: &str) -> RecordError {
    RecordError::HtmlParsingError {
        url: url.to_string(),
        message: message.to_string(),
    }
}

/// Builds an HTML parsing error and logs context for site structure changes.
///
/// Returns a structured `HtmlParsingError`; the decision to terminate the
/// process belongs to the application boundary, not the library.
#[must_use]
pub fn handle_html_parsing_error(url: &str, error: &str) -> RecordError {
    log::error!("HTML parsing failed for URL: {url}");
    log::error!("Error: {error}");
    log::error!("This may indicate a change in the website structure.");
    log::error!("Please report this issue so the scraper can be updated.");

    html_parsing_error(url, error)
}
