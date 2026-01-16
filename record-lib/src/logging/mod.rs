//! Structured logging with error.log persistence.
//!
//! This module provides comprehensive logging functionality with
//! automatic error logging to error.log file with rotation.

#![allow(clippy::missing_panics_doc)]

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use tracing::{debug, info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Configuration for the logging system.
#[derive(Clone)]
pub struct LoggingConfig {
    /// Path to the error log file.
    pub error_log_path: String,
    /// Maximum size of error log file in bytes before rotation.
    pub max_log_size: usize,
    /// Number of backup log files to keep.
    pub num_backup_files: usize,
    /// Whether to include timestamps in logs.
    pub include_timestamps: bool,
    /// Whether to include module paths in logs.
    pub include_module_paths: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            error_log_path: "error.log".to_string(),
            max_log_size: 10 * 1024 * 1024, // 10MB
            num_backup_files: 5,
            include_timestamps: true,
            include_module_paths: true,
        }
    }
}

/// Initializes the logging system with console and file output.
///
/// # Arguments
///
/// * `config` - Configuration for the logging system.
///
/// # Errors
///
/// Returns an error if initialization fails.
pub fn init_logging(config: &LoggingConfig) -> anyhow::Result<()> {
    // Create error log file if it doesn't exist
    if !Path::new(&config.error_log_path).exists() {
        std::fs::File::create(&config.error_log_path)?;
    }

    // Check log file size and rotate if necessary
    check_and_rotate_log(
        &config.error_log_path,
        config.max_log_size,
        config.num_backup_files,
    )?;

    // Set up environment filter from RUST_LOG or default to INFO
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Set up console layer
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(config.include_module_paths)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    // Set up file layer for error.log
    let config_clone = config.clone();
    let error_log_path = config_clone.error_log_path.clone();

    // Validate that we can create/open the log file
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&error_log_path)
        .map_err(|e| {
            anyhow::anyhow!("Failed to open error log file '{}': {}", error_log_path, e)
        })?;

    let file_layer = fmt::layer()
        .with_writer(move || {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&error_log_path)
                .expect("Failed to open error log file")
        })
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false);

    // Initialize subscriber
    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    info!("Logging system initialized");
    debug!("Error log file: {}", config.error_log_path);
    debug!("Max log size: {} bytes", config.max_log_size);

    Ok(())
}

/// Checks and rotates the log file if it exceeds the maximum size.
///
/// # Arguments
///
/// * `log_path` - Path to the log file.
/// * `max_size` - Maximum size in bytes before rotation.
/// * `num_backups` - Number of backup files to keep.
///
/// # Errors
///
/// Returns an error if file operations fail.
fn check_and_rotate_log(
    log_path: &str,
    max_size: usize,
    num_backups: usize,
) -> std::io::Result<()> {
    let path = Path::new(log_path);

    if !path.exists() {
        return Ok(());
    }

    let metadata = std::fs::metadata(path)?;
    #[allow(clippy::cast_possible_truncation)]
    let file_size = metadata.len() as usize;

    if file_size < max_size {
        return Ok(());
    }

    warn!(
        "Log file size {} exceeds maximum {}, rotating...",
        file_size, max_size
    );

    // Rotate existing backup files
    for i in (1..num_backups).rev() {
        let old_backup = format!("{log_path}.{i}");
        let new_backup = format!("{}.{}", log_path, i + 1);

        let old_path = Path::new(&old_backup);
        let new_path = Path::new(&new_backup);

        if old_path.exists() {
            std::fs::rename(old_path, new_path)?;
        }
    }

    // Move current log to .1
    let backup_path = format!("{log_path}.1");
    std::fs::rename(log_path, &backup_path)?;

    // Create new log file
    std::fs::File::create(log_path)?;

    info!("Log rotation completed");

    Ok(())
}

/// Logs an error to the error log file with full context.
///
/// # Arguments
///
/// * `error` - The error to log.
/// * `context` - Additional context information.
///
/// # Errors
///
/// Returns an error if writing to the log file fails.
pub fn log_error_with_context<E>(error: &E, context: &str) -> std::io::Result<()>
where
    E: std::error::Error,
{
    let log_path = "error.log";
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");

    writeln!(file, "[{timestamp}] ERROR: {context} - {error}")?;

    // Log error chain
    let mut source = error.source();
    let mut depth = 1;
    while let Some(cause) = source {
        writeln!(
            file,
            "{}  Caused by: {}: {}",
            "  ".repeat(depth),
            depth,
            cause
        )?;
        source = cause.source();
        depth += 1;
    }

    // Log stack trace if available (requires 'backtrace' feature)
    #[cfg(feature = "backtrace")]
    {
        use std::backtrace::BacktraceStatus;
        let backtrace = std::backtrace::Backtrace::capture();
        if backtrace.status() == BacktraceStatus::Captured {
            writeln!(file, "  Stack trace:\n{}", backtrace)?;
        }
    }

    file.flush()?;

    Ok(())
}

/// Logs a warning to the error log file.
///
/// # Arguments
///
/// * `message` - The warning message to log.
///
/// # Errors
///
/// Returns an error if writing to the log file fails.
pub fn log_warning(message: &str) -> std::io::Result<()> {
    let log_path = "error.log";
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    writeln!(file, "[{timestamp}] WARN: {message}")?;

    file.flush()?;

    Ok(())
}

/// Logs an info message to the error log file.
///
/// # Arguments
///
/// * `message` - The info message to log.
///
/// # Errors
///
/// Returns an error if writing to the log file fails.
pub fn log_info(message: &str) -> std::io::Result<()> {
    let log_path = "error.log";
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    writeln!(file, "[{timestamp}] INFO: {message}")?;

    file.flush()?;

    Ok(())
}

/// A custom error type that includes file and line information.
#[derive(Debug)]
pub struct LocatedError<E> {
    /// The underlying error.
    pub error: E,
    /// The file where the error occurred.
    pub file: &'static str,
    /// The line where the error occurred.
    pub line: u32,
    /// The column where the error occurred.
    pub column: u32,
}

impl<E: std::error::Error> std::error::Error for LocatedError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.error.source()
    }
}

impl<E: std::fmt::Display> std::fmt::Display for LocatedError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} ({}:{})", self.error, self.file, self.line)
    }
}

/// Creates a located error at the current position.
#[macro_export]
macro_rules! located_error {
    ($error:expr) => {
        $crate::logging::LocatedError {
            error: $error,
            file: file!(),
            line: line!(),
            column: column!(),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.error_log_path, "error.log");
        assert_eq!(config.max_log_size, 10 * 1024 * 1024);
        assert_eq!(config.num_backup_files, 5);
    }

    #[test]
    fn test_log_error_context() -> std::io::Result<()> {
        let error = std::io::Error::new(std::io::ErrorKind::NotFound, "test file not found");
        log_error_with_context(&error, "Test context")?;
        Ok(())
    }

    #[test]
    fn test_located_error() {
        let io_error = std::io::Error::other("test error");
        let located = located_error!(io_error);

        assert_eq!(located.file, file!());
        assert!(located.line > 0);
    }
}
