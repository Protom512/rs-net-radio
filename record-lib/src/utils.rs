use std::{env, fs, path::Path, io};
use log::{debug, error}; // Added log import

const RS_NET_ARCHIVE_PATH_ENV_VAR: &'static str = "RS_NET_ARCHIVE_PATH";

/// Ensures the archive path for a given service exists and returns it.
/// The path will be "{RS_NET_ARCHIVE_PATH}/{service_name}".
pub fn ensure_archive_path(service_name: &str) -> Result<String, io::Error> {
    let base_path_var = env::var(RS_NET_ARCHIVE_PATH_ENV_VAR);

    let base_path = match base_path_var {
        Ok(path) => path,
        Err(e) => {
            error!("Environment variable {} is not set: {}", RS_NET_ARCHIVE_PATH_ENV_VAR, e);
            // Return an error that can be handled by the caller
            return Err(io::Error::new(io::ErrorKind::NotFound,
                format!("{} is not set: {}", RS_NET_ARCHIVE_PATH_ENV_VAR, e)));
        }
    };

    let service_path_str = format!("{}/{}", base_path, service_name);
    let service_path = Path::new(&service_path_str);

    if !service_path.is_dir() {
        fs::create_dir_all(service_path).map_err(|e| {
            error!("Failed to create directory {}: {}", service_path_str, e);
            e
        })?;
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
        .replace('"', "”") // Corrected from `\”` to `"`
        .replace('<', "＜")
        .replace('>', "＞")
        .replace('|', "｜") // Added pipe character based on typical sanitization
        .replace('`', "`") // Keep existing backtick replacement
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
    // For this subtask, we'll focus on the sanitization tests.
}
