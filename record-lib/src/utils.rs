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
    HtmlParsingError {
        url: String,
        message: String,
    },
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RecordError::Io(e) => write!(f, "IO error: {}", e),
            RecordError::EnvVar(e) => write!(f, "Environment variable error: {}", e),
            RecordError::Reqwest(e) => write!(f, "Reqwest error: {}", e),
            RecordError::SerdeJson(e) => write!(f, "Serde JSON error: {}", e),
            RecordError::CommandFailed {
                command,
                exit_code,
                stderr,
            } => write!(
                f,
                "Command '{}' failed with code {:?}. Stderr: {}",
                command, exit_code, stderr
            ),
            RecordError::TempDir => write!(f, "Failed to get temporary directory path"),
            RecordError::Other(s) => write!(f, "Other error: {}", s),
            RecordError::HttpError {
                status_code,
                url,
                message,
            } => write!(
                f,
                "HTTP error {} for {}: {}",
                status_code, url, message
            ),
            RecordError::HtmlParsingError {
                url,
                message,
            } => write!(
                f,
                "HTML parsing error for {}: {}",
                url, message
            ),
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
/// The path will be "{RS_NET_ARCHIVE_PATH}/{service_name}".
pub fn ensure_archive_path(service_name: &str) -> Result<String, RecordError> {
    let base_path = env::var(RS_NET_ARCHIVE_PATH_ENV_VAR)?; // Uses From<VarError>

    let service_path_str = format!("{}/{}", base_path, service_name);
    let service_path = Path::new(&service_path_str);

    if !service_path.is_dir() {
        fs::create_dir_all(service_path)?; // Uses From<io::Error>
        debug!("Created directory: {}", service_path_str);
    }
    Ok(service_path_str)
}

/// Sanitizes a filename by replacing characters forbidden by common filesystems.
pub fn sanitize_filename(filename: &str) -> String {
    sanitize_filename_crate::sanitize(filename)
}

/// Creates an HTTP error from status code and URL.
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
pub fn check_http_status(response: &reqwest::blocking::Response, url: &str) -> Result<(), RecordError> {
    let status = response.status();

    if status.is_client_error() || status.is_server_error() {
        let status_code = status.as_u16();

        // Handle specific error codes with custom exit behavior
        match status_code {
            403 | 429 => {
                log::error!("HTTP {} error for {}. Access denied or rate limited.", status_code, url);
                log::error!("This application will exit with code 2. Please try again later or check your access permissions.");
                std::process::exit(2);
            }
            _ => {
                return Err(http_error(status_code, url));
            }
        }
    }

    Ok(())
}

/// Creates an HTML parsing error.
pub fn html_parsing_error(url: &str, message: &str) -> RecordError {
    RecordError::HtmlParsingError {
        url: url.to_string(),
        message: message.to_string(),
    }
}

/// Checks for HTML parsing errors and handles site structure changes.
pub fn handle_html_parsing_error(url: &str, error: &str) -> RecordError {
    log::error!("HTML parsing failed for URL: {}", url);
    log::error!("Error: {}", error);
    log::error!("This may indicate a change in the website structure. The application will exit with code 3.");
    log::error!("Please report this issue so the scraper can be updated.");

    std::process::exit(3);
}
