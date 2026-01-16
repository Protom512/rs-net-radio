//! Streaming recording functionality with memory management.
//!
//! This module provides functionality for recording streaming audio with
//! efficient memory usage through chunked processing and memory monitoring.

pub mod chunk_processor;
pub mod memory_monitor;

pub use chunk_processor::{ChunkProcessor, CHUNK_SIZE};
pub use memory_monitor::{MemoryMonitor, MemoryStats};

use std::path::Path;
use std::sync::Arc;
use anyhow::{Context, Result};
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

/// Configuration for streaming recording.
#[derive(Clone)]
pub struct StreamingConfig {
    /// Maximum number of parallel downloads.
    pub max_parallel_downloads: usize,
    /// Memory limit in bytes (default: 512MB).
    pub memory_limit_bytes: usize,
    /// Chunk size for streaming downloads (default: 4KB).
    pub chunk_size: usize,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_parallel_downloads: 3,
            memory_limit_bytes: 512 * 1024 * 1024, // 512MB
            chunk_size: CHUNK_SIZE,
        }
    }
}

/// Records a streaming audio source with memory management.
///
/// # Arguments
///
/// * `url` - The URL of the streaming audio source.
/// * `output_path` - The path where the recording will be saved.
/// * `config` - Configuration for the streaming recording.
///
/// # Errors
///
/// Returns an error if the recording fails or memory limit is exceeded.
pub async fn record_streaming(
    url: &str,
    output_path: &Path,
    config: &StreamingConfig,
) -> Result<MemoryStats> {
    info!(
        "Starting streaming recording from {} to {}",
        url,
        output_path.display()
    );

    // Initialize memory monitor
    let memory_monitor = MemoryMonitor::new(config.memory_limit_bytes);
    memory_monitor.start()?;

    // Create chunk processor
    let chunk_processor = ChunkProcessor::new(
        output_path,
        config.chunk_size,
        Arc::new(memory_monitor),
    );

    // Perform the recording
    let stats = chunk_processor
        .process_stream(url)
        .await
        .context("Failed to process streaming audio")?;

    // Check memory usage
    if stats.memory_usage_bytes > config.memory_limit_bytes {
        warn!(
            "Memory usage {} exceeds limit {}",
            stats.memory_usage_bytes, config.memory_limit_bytes
        );
    } else {
        debug!(
            "Memory usage within limit: {} / {}",
            stats.memory_usage_bytes, config.memory_limit_bytes
        );
    }

    info!(
        "Streaming recording completed: {} bytes written in {:.2}s",
        stats.bytes_written,
        stats.duration.as_secs_f64()
    );

    Ok(stats)
}

/// Records multiple streaming sources in parallel with memory management.
///
/// # Arguments
///
/// * `streams` - Vector of (url, `output_path`) tuples.
/// * `config` - Configuration for the streaming recording.
///
/// # Errors
///
/// Returns an error if any recording fails or memory limit is exceeded.
pub async fn record_batch_streaming(
    streams: Vec<(String, std::path::PathBuf)>,
    config: StreamingConfig,
) -> Result<Vec<Result<MemoryStats>>> {
    if streams.is_empty() {
        return Ok(Vec::new());
    }

    info!(
        "Starting batch streaming recording: {} streams",
        streams.len()
    );

    let semaphore = Arc::new(Semaphore::new(config.max_parallel_downloads));
    let mut tasks = tokio::task::JoinSet::new();

    for (url, output_path) in streams {
        let permit = Arc::clone(&semaphore);
        let config = config.clone();

        tasks.spawn(async move {
            let _permit = permit.acquire().await?;
            record_streaming(&url, &output_path, &config).await
        });
    }

    let mut results = Vec::new();

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(r) => results.push(r),
            Err(e) => {
                error!("Task panicked: {}", e);
                results.push(Err(anyhow::anyhow!("Task panicked: {e}")));
            }
        }
    }

    Ok(results)
}
