//! Chunk processing for streaming audio downloads.
//!
//! This module provides efficient chunked processing of streaming audio
//! to minimize memory usage.

use std::path::Path;
use std::sync::Arc;
use anyhow::{Context, Result};
use futures_util::AsyncReadExt;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tracing::{debug, error, info};

use super::memory_monitor::{MemoryMonitor, MemoryStats};

/// Default chunk size for streaming downloads (4KB).
pub const CHUNK_SIZE: usize = 4096;

/// Chunk processor for streaming audio downloads.
///
/// This struct handles the downloading and writing of streaming audio
/// in fixed-size chunks to minimize memory usage.
pub struct ChunkProcessor {
    /// Output file path.
    output_path: std::path::PathBuf,
    /// Chunk size for processing.
    chunk_size: usize,
    /// Memory monitor for tracking usage.
    memory_monitor: Arc<MemoryMonitor>,
}

impl ChunkProcessor {
    /// Creates a new ChunkProcessor.
    ///
    /// # Arguments
    ///
    /// * `output_path` - Path where the audio will be saved.
    /// * `chunk_size` - Size of chunks for processing.
    /// * `memory_monitor` - Memory monitor for tracking usage.
    pub fn new(
        output_path: &Path,
        chunk_size: usize,
        memory_monitor: Arc<MemoryMonitor>,
    ) -> Self {
        Self {
            output_path: output_path.to_path_buf(),
            chunk_size,
            memory_monitor,
        }
    }

    /// Processes a streaming audio source.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL of the streaming audio source.
    ///
    /// # Errors
    ///
    /// Returns an error if the download or writing fails.
    pub async fn process_stream(&self, url: &str) -> Result<MemoryStats> {
        let start_time = std::time::Instant::now();

        // Create HTTP client with timeout
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .context("Failed to create HTTP client")?;

        // Start the download
        info!("Starting download from {}", url);
        let response = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await
            .context("Failed to start download")?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let error_msg = format!("HTTP error: {}", status);
            error!("{}", error_msg);
            return Err(anyhow::anyhow!(error_msg));
        }

        let total_bytes = response.content_length().unwrap_or(0);
        info!("Total bytes to download: {}", total_bytes);

        // Create output file and buffered writer
        let file = File::create(&self.output_path)
            .await
            .context("Failed to create output file")?;
        let mut writer = BufWriter::with_capacity(self.chunk_size, file);

        // Download and write in chunks
        let mut downloaded_bytes = 0u64;
        let mut stream = response.bytes_stream();

        use futures_util::stream::StreamExt;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to download chunk")?;

            // Check memory limit before processing chunk
            if let Err(e) = self.memory_monitor.check_memory_limit() {
                error!("Memory limit exceeded: {}", e);
                return Err(e);
            }

            // Write chunk to file
            writer
                .write_all(&chunk)
                .await
                .context("Failed to write chunk to file")?;

            downloaded_bytes += chunk.len() as u64;

            // Update memory monitor
            self.memory_monitor
                .update_usage(chunk.len())
                .context("Failed to update memory usage")?;

            debug!(
                "Downloaded {} / {} bytes ({:.1}%)",
                downloaded_bytes,
                total_bytes,
                if total_bytes > 0 {
                    (downloaded_bytes as f64 / total_bytes as f64) * 100.0
                } else {
                    0.0
                }
            );
        }

        // Flush remaining data
        writer
            .flush()
            .await
            .context("Failed to flush output file")?;

        let duration = start_time.elapsed();

        info!(
            "Download completed: {} bytes in {:.2}s ({:.2} MB/s)",
            downloaded_bytes,
            duration.as_secs_f64(),
            if duration.as_secs_f64() > 0.0 {
                (downloaded_bytes as f64 / 1024.0 / 1024.0) / duration.as_secs_f64()
            } else {
                0.0
            }
        );

        // Get final memory stats
        let stats = self.memory_monitor.get_stats();
        Ok(MemoryStats {
            memory_usage_bytes: stats.memory_usage_bytes,
            bytes_written: downloaded_bytes,
            duration,
            chunks_processed: stats.chunks_processed,
        })
    }

    /// Processes a stream from a byte reader.
    ///
    /// # Arguments
    ///
    /// * `reader` - The reader to process.
    ///
    /// # Errors
    ///
    /// Returns an error if reading or writing fails.
    pub async fn process_reader<R>(
        &self,
        mut reader: R,
    ) -> Result<MemoryStats>
    where
        R: futures_util::AsyncReadExt + Unpin + std::marker::Send,
    {
        let start_time = std::time::Instant::now();

        // Create output file and buffered writer
        let file = File::create(&self.output_path)
            .await
            .context("Failed to create output file")?;
        let mut writer = BufWriter::with_capacity(self.chunk_size, file);

        let mut buffer = vec![0u8; self.chunk_size];
        let mut total_bytes = 0u64;
        let mut chunk_count = 0u32;

        loop {
            // Check memory limit before reading
            if let Err(e) = self.memory_monitor.check_memory_limit() {
                error!("Memory limit exceeded: {}", e);
                return Err(e);
            }

            let bytes_read = reader
                .read(&mut buffer)
                .await
                .context("Failed to read from stream")?;

            if bytes_read == 0 {
                break; // EOF
            }

            // Write chunk to file
            writer
                .write_all(&buffer[..bytes_read])
                .await
                .context("Failed to write chunk to file")?;

            // Update memory monitor
            self.memory_monitor
                .update_usage(bytes_read)
                .context("Failed to update memory usage")?;

            total_bytes += bytes_read as u64;
            chunk_count += 1;

            debug!("Processed chunk {}: {} bytes", chunk_count, bytes_read);
        }

        // Flush remaining data
        writer
            .flush()
            .await
            .context("Failed to flush output file")?;

        let duration = start_time.elapsed();

        info!(
            "Processing completed: {} bytes in {} chunks, {:.2}s",
            total_bytes, chunk_count, duration.as_secs_f64()
        );

        Ok(MemoryStats {
            memory_usage_bytes: self.memory_monitor.get_stats().memory_usage_bytes,
            bytes_written: total_bytes,
            duration,
            chunks_processed: chunk_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn test_chunk_processor() {
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_output.mp3");

        let memory_monitor = Arc::new(MemoryMonitor::new(1024 * 1024)); // 1MB limit
        let processor = ChunkProcessor::new(&output_path, CHUNK_SIZE, memory_monitor);

        // Create test data
        let test_data = vec![0u8; 8192]; // 8KB of data
        let reader = Cursor::new(test_data);

        let result = processor.process_reader(reader).await;

        assert!(result.is_ok());
        let stats = result.expect("Failed to process reader");
        assert_eq!(stats.bytes_written, 8192);

        // Clean up
        let _ = std::fs::remove_file(&output_path);
    }

    #[tokio::test]
    async fn test_memory_limit_exceeded() {
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_output_limit.mp3");

        let memory_monitor = Arc::new(MemoryMonitor::new(100)); // Very low limit
        let processor = ChunkProcessor::new(&output_path, CHUNK_SIZE, memory_monitor);

        // Create test data that exceeds limit
        let test_data = vec![0u8; 8192]; // 8KB of data
        let reader = Cursor::new(test_data);

        let result = processor.process_reader(reader).await;

        assert!(result.is_err());

        // Clean up
        let _ = std::fs::remove_file(&output_path);
    }
}
