use std::{env, fs, path::Path, io};
use log::{debug, error};
use std::fmt;
use std::error::Error as StdError; // Alias to avoid conflict

#[derive(Debug)]
pub enum RecordError {
    Io(std::io::Error),
    EnvVar(std::env::VarError),
    Reqwest(reqwest::Error),
    SerdeJson(serde_json::Error),
    // SerdeXml(serde_xml_rs::Error), // Add if direct usage occurs
    CommandFailed{ command: String, exit_code: Option<i32>, stderr: String },
    TempDir,
    Other(String),
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RecordError::Io(e) => write!(f, "IO error: {}", e),
            RecordError::EnvVar(e) => write!(f, "Environment variable error: {}", e),
            RecordError::Reqwest(e) => write!(f, "Reqwest error: {}", e),
            RecordError::SerdeJson(e) => write!(f, "Serde JSON error: {}", e),
            RecordError::CommandFailed{ command, exit_code, stderr } =>
                write!(f, "Command '{}' failed with code {:?}. Stderr: {}", command, exit_code, stderr),
            RecordError::TempDir => write!(f, "Failed to get temporary directory path"),
            RecordError::Other(s) => write!(f, "Other error: {}", s),
        }
    }
}

impl StdError for RecordError {}

impl From<std::io::Error> for RecordError {
    fn from(err: std::io::Error) -> RecordError { RecordError::Io(err) }
}
impl From<std::env::VarError> for RecordError {
    fn from(err: std::env::VarError) -> RecordError { RecordError::EnvVar(err) }
}
impl From<reqwest::Error> for RecordError {
    fn from(err: reqwest::Error) -> RecordError { RecordError::Reqwest(err) }
}
impl From<serde_json::Error> for RecordError {
    fn from(err: serde_json::Error) -> RecordError { RecordError::SerdeJson(err) }
}

const RS_NET_ARCHIVE_PATH_ENV_VAR: &'static str = "RS_NET_ARCHIVE_PATH";

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
    filename
        .replace('\\', "￥")
        .replace('/', "／")
        .replace(':', "：")
        .replace('*', "＊")
        .replace('?', "？")
        .replace('"', "”")
        .replace('<', "＜")
        .replace('>', "＞")
        .replace('|', "｜")
        .replace('`', "`")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename_basic() {
        assert_eq!(sanitize_filename("Fate/Test: *?\"<>|`"), "Fate／Test： ＊？”＜＞｜`");
    }

    #[test]
    fn test_sanitize_filename_no_forbidden_chars() {
        assert_eq!(sanitize_filename("Normal_Filename_123.mp4"), "Normal_Filename_123.mp4");
    }

    #[test]
    fn test_sanitize_filename_empty() {
        assert_eq!(sanitize_filename(""), "");
    }

    // Note: Testing ensure_archive_path requires filesystem interaction and environment variables,
    // which is more suited for integration tests or requires mocking.
}
