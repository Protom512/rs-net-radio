//! Main entry point for the rs-net-radio application.
//!
//! This is a refactored version that uses the new architecture with
//! `BatchRecorder` and `CronManager` for better separation of concerns.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use record_lib::batch::BatchRecorder;
use record_lib::config::repository::ConfigRepository;
use record_lib::config::FileConfigRepository;
use record_lib::domain::service::Program;
use record_lib::scheduler::CronManager;
use record_lib::utils::RecordError;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Command-line arguments for the application.
#[derive(Parser, Debug)]
#[command(name = "rs-net-radio")]
#[command(about = "Internet radio recording application for Japanese services", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

/// Available commands.
#[derive(Subcommand, Debug)]
enum Commands {
    /// Run batch recording from a list of programs
    Batch {
        /// Path to the program list file
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Start cron-based scheduled recording
    Cron {
        /// Path to the cron configuration file
        #[arg(short, long, default_value = "cron.toml")]
        config: PathBuf,
    },
}

/// Mock recording service for demonstration.
/// In production, this would be replaced with actual Hibiki/Onsen/Radiko services.
struct MockRecordService;

#[async_trait::async_trait]
impl record_lib::domain::service::RecordService for MockRecordService {
    async fn record(
        &self,
        url: &str,
        output_path: &std::path::Path,
    ) -> Result<record_lib::domain::metadata::RecordingMetadata, RecordError> {
        info!("Recording from {} to {}", url, output_path.display());

        // Simulate recording
        tokio::time::sleep(Duration::from_secs(1)).await;

        Ok(record_lib::domain::metadata::RecordingMetadata::new(
            "Mock Program".to_string(),
            chrono::Utc::now(),
            chrono::Utc::now(),
            output_path.display().to_string(),
            1024 * 1024,
            128,
        ))
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    init_tracing();

    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        for cause in e.chain().skip(1) {
            eprintln!("  Caused by: {}", cause);
        }
        std::process::exit(1);
    }
}

/// Main execution logic.
async fn run() -> Result<()> {
    let args = Args::parse();

    // Check for required environment variables
    std::env::var("RS_NET_ARCHIVE_PATH").context(
        "Environment variable RS_NET_ARCHIVE_PATH is not set.\n\n\
         Hint: Set it to the root directory where you want to save recordings.\n\
         Example: export RS_NET_ARCHIVE_PATH=/path/to/archive",
    )?;

    match args.command {
        Commands::Batch { input } => {
            run_batch_recording(input).await?;
        }
        Commands::Cron { config } => {
            run_cron_scheduling(config).await?;
        }
    }

    Ok(())
}

/// Initializes the tracing subscriber for structured logging.
fn init_tracing() {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with(fmt::layer())
        .init();
}

/// Runs batch recording from a program list file.
///
/// # Arguments
///
/// * `input_path` - Path to the program list file.
///
/// # Errors
///
/// Returns an error if batch recording fails.
async fn run_batch_recording(input_path: PathBuf) -> Result<()> {
    info!("Starting batch recording from: {}", input_path.display());

    // Load configuration
    let config_repo = FileConfigRepository;
    let config = config_repo
        .load()
        .await
        .context("Failed to load configuration")?;

    // Parse program list
    let programs = parse_program_list(&input_path).context("Failed to parse program list")?;

    // Create batch recorder
    let recorder = BatchRecorder::new(
        config.batch.max_parallel_jobs,
        config.batch.retry_count,
        Duration::from_secs(config.batch.timeout_seconds),
    );

    // Create recording service
    let service = Arc::new(MockRecordService);

    // Execute batch recording
    let summary = recorder
        .record_batch(programs, service)
        .await
        .context("Batch recording failed")?;

    // Print summary
    summary.print_summary();

    Ok(())
}

/// Runs cron-based scheduled recording.
///
/// # Arguments
///
/// * `config_path` - Path to the cron configuration file.
///
/// # Errors
///
/// Returns an error if cron scheduling fails.
async fn run_cron_scheduling(config_path: PathBuf) -> Result<()> {
    info!(
        "Starting cron scheduling with config: {}",
        config_path.display()
    );

    // Load cron configuration
    let cron_config =
        load_cron_config(&config_path).context("Failed to load cron configuration")?;

    // Create cron manager
    let service = Arc::new(MockRecordService);
    let cron_manager = CronManager::new(service)
        .await
        .context("Failed to create cron manager")?;

    // Add schedules
    for schedule in cron_config.schedules {
        cron_manager
            .add_schedule(schedule)
            .await
            .context("Failed to add schedule")?;
    }

    // Start scheduler
    cron_manager
        .start()
        .await
        .context("Failed to start cron manager")?;

    info!("Cron scheduler started, waiting for scheduled tasks...");

    // Wait forever (or until Ctrl+C)
    tokio::signal::ctrl_c()
        .await
        .context("Failed to listen for Ctrl+C")?;

    info!("Shutting down...");

    // Shutdown scheduler
    cron_manager
        .shutdown()
        .await
        .context("Failed to shutdown cron manager")?;

    Ok(())
}

/// Parses a program list file.
///
/// # Arguments
///
/// * `path` - Path to the program list file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
fn parse_program_list(path: &PathBuf) -> Result<Vec<Program>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read program list: {}", path.display()))?;

    let mut programs = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse format: "title|url|output_path"
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() != 3 {
            error!(
                "Invalid format on line {}: {}. Expected format: title|url|output_path",
                line_num + 1,
                line
            );
            continue;
        }

        let title = parts[0].to_string();
        let url = parts[1].to_string();
        let output_path = PathBuf::from(parts[2]);

        programs.push(Program {
            title,
            url,
            output_path,
        });
    }

    Ok(programs)
}

/// Loads cron configuration from a TOML file.
///
/// # Arguments
///
/// * `path` - Path to the cron configuration file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
fn load_cron_config(path: &PathBuf) -> Result<record_lib::config::CronConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read cron config: {}", path.display()))?;

    let config: record_lib::config::CronConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse cron config: {}", path.display()))?;

    Ok(config)
}
