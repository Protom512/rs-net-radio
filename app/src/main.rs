//! Main entry point for the rs-net-radio application.
//!
//! This version uses the AppFacade pattern to reduce coupling
//! between the main entry point and internal modules.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use record_lib::application::AppFacade;
use record_lib::config::CronConfig;
use record_lib::domain::metadata::RecordingMetadata;
use record_lib::domain::service::RecordService;
use record_lib::utils::RecordError;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;
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
impl RecordService for MockRecordService {
    async fn record(
        &self,
        url: &str,
        output_path: &std::path::Path,
    ) -> Result<RecordingMetadata, RecordError> {
        info!("Recording from {} to {}", url, output_path.display());

        // Simulate recording
        tokio::time::sleep(Duration::from_secs(1)).await;

        Ok(RecordingMetadata::new(
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
async fn main() -> Result<()> {
    // Initialize tracing
    init_tracing();

    let args = Args::parse();

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

/// Runs batch recording from a program list file using AppFacade.
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

    // Create application facade
    let facade = AppFacade::new().await?;

    // Create recording service
    let service = Arc::new(MockRecordService);

    // Execute batch recording through facade
    let summary = facade.run_batch(input_path, service).await?;

    // Print summary
    summary.print_summary();

    Ok(())
}

/// Runs cron-based scheduled recording using AppFacade.
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

    // Create application facade
    let facade = AppFacade::new().await?;

    // Create cron manager through facade
    let service = Arc::new(MockRecordService);
    let cron_manager = facade.create_cron_manager(service).await?;

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

/// Loads cron configuration from a TOML file.
///
/// # Arguments
///
/// * `path` - Path to the cron configuration file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
fn load_cron_config(path: &PathBuf) -> Result<CronConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read cron config: {}", path.display()))?;

    let config: CronConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse cron config: {}", path.display()))?;

    Ok(config)
}
