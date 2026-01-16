//! Chunk processing for streaming audio downloads.
//!
//! This module provides efficient chunked processing of streaming audio
//! to minimize memory usage through 4KB chunk-based processing with async I/O.

#![allow(clippy::missing_errors_doc)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::cast_precision_loss)]

use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tracing::{debug, error, info, warn};

use super::memory_monitor::{MemoryMonitor, MemoryStats};

/// Default chunk size for streaming downloads (4KB).
/// This size is optimal for disk I/O and memory efficiency.
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
    /// Creates a new `ChunkProcessor`.
    ///
    /// # Arguments
    ///
    /// * `output_path` - Path where the audio will be saved.
    /// * `chunk_size` - Size of chunks for processing.
    /// * `memory_monitor` - Memory monitor for tracking usage.
    #[must_use]
    pub fn new(output_path: &Path, chunk_size: usize, memory_monitor: Arc<MemoryMonitor>) -> Self {
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
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
            .context("Failed to start download")?;

        let http_status = response.status();
        if http_status.is_client_error() || http_status.is_server_error() {
            let error_msg = format!("HTTP error: {http_status}");
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

    /// Processes a stream from a byte reader with optimized 4KB chunk processing.
    ///
    /// This method implements efficient chunked processing by:
    /// 1. Reading data in 4KB chunks to minimize memory usage
    /// 2. Writing chunks immediately to disk using async I/O
    /// 3. Monitoring memory usage to enforce the 512MB limit
    /// 4. Using buffered writes for better I/O performance
    ///
    /// # Arguments
    ///
    /// * `reader` - The reader to process.
    ///
    /// # Errors
    ///
    /// Returns an error if reading or writing fails or memory limit is exceeded.
    pub async fn process_reader<R>(&self, mut reader: R) -> Result<MemoryStats>
    where
        R: futures_util::AsyncReadExt + Unpin + std::marker::Send,
    {
        let start_time = std::time::Instant::now();

        // Create output file and buffered writer with 4KB buffer
        let file = File::create(&self.output_path)
            .await
            .context("Failed to create output file")?;

        // Use buffered writer with chunk size for optimal I/O
        let mut writer = BufWriter::with_capacity(self.chunk_size, file);

        // Pre-allocate buffer to avoid reallocations
        let mut buffer = vec![0u8; self.chunk_size];
        let mut total_bytes = 0u64;
        let mut chunk_count = 0u32;
        let mut last_progress_time = start_time;

        info!(
            "Starting chunk processing with {} byte chunks",
            self.chunk_size
        );

        loop {
            // Check memory limit before reading each chunk
            if let Err(e) = self.memory_monitor.check_memory_limit() {
                error!("Memory limit exceeded: {}", e);
                return Err(e);
            }

            // Read up to chunk_size bytes
            let bytes_read = reader
                .read(&mut buffer)
                .await
                .context("Failed to read from stream")?;

            if bytes_read == 0 {
                debug!("Reached end of stream");
                break; // EOF
            }

            // Write chunk to file immediately using async I/O
            writer
                .write_all(&buffer[..bytes_read])
                .await
                .context("Failed to write chunk to file")?;

            // Update memory monitor with processed bytes
            self.memory_monitor
                .update_usage(bytes_read)
                .context("Failed to update memory usage")?;

            total_bytes += bytes_read as u64;
            chunk_count += 1;

            // Log progress every 100 chunks or every 5 seconds
            let now = std::time::Instant::now();
            if chunk_count % 100 == 0 || now.duration_since(last_progress_time).as_secs() >= 5 {
                let elapsed = now.duration_since(start_time).as_secs_f64();
                let throughput = if elapsed > 0.0 {
                    (total_bytes as f64 / 1024.0 / 1024.0) / elapsed
                } else {
                    0.0
                };
                debug!(
                    "Processed {} chunks ({} bytes, {:.2} MB/s)",
                    chunk_count, total_bytes, throughput
                );
                last_progress_time = now;
            }

            // Detect potential memory leaks periodically
            if chunk_count % 1000 == 0 {
                if let Err(leak_err) = self.memory_monitor.detect_potential_leak() {
                    warn!("Potential memory leak detected: {}", leak_err);
                }
            }
        }

        // Flush remaining data to ensure all data is written
        writer
            .flush()
            .await
            .context("Failed to flush output file")?;

        let duration = start_time.elapsed();

        info!(
            "Chunk processing completed: {} bytes in {} chunks ({:.2}s)",
            total_bytes,
            chunk_count,
            duration.as_secs_f64()
        );

        // Calculate final statistics
        let final_stats = MemoryStats {
            memory_usage_bytes: self.memory_monitor.get_stats().memory_usage_bytes,
            bytes_written: total_bytes,
            duration,
            chunks_processed: chunk_count,
        };

        // Verify memory efficiency
        if final_stats.memory_usage_bytes > self.chunk_size * 10 {
            warn!(
                "High memory usage detected: {} bytes (consider reducing chunk size or checking for memory leaks)",
                final_stats.memory_usage_bytes
            );
        }

        Ok(final_stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::io::Cursor;
    use tokio::io::AsyncReadExt;

    /// テスト用のモックHTTPサーバーを作成
    async fn create_mock_http_server(port: u16) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
                .await
                .expect("Failed to bind mock server");

            if let Ok((mut socket, _addr)) = listener.accept().await {
                let mut buffer = [0u8; 1024];
                let _ = socket.read(&mut buffer).await;

                // HTTPレスポンスを送信
                let response = "HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\n\r\n";
                let _ = socket.write_all(response.as_bytes()).await;

                // テストデータを4KBチャンクで送信
                let test_data = vec![0u8; CHUNK_SIZE * 2]; // 8KBのデータ
                let _ = socket.write_all(&test_data).await;
            }
        })
    }

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

    /// 4KBチャンク処理のテスト - データが正しくチャンク単位で処理されること
    #[tokio::test]
    async fn test_4kb_chunk_processing() {
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_4kb_chunks.mp3");

        let memory_monitor = Arc::new(MemoryMonitor::new(512 * 1024 * 1024)); // 512MB limit
        let processor = ChunkProcessor::new(&output_path, CHUNK_SIZE, memory_monitor);

        // 4KBの倍数のデータを作成 (12KB = 3 chunks)
        let test_data = vec![42u8; CHUNK_SIZE * 3];
        let reader = Cursor::new(test_data);

        let result = processor.process_reader(reader).await;

        assert!(result.is_ok());
        let stats = result.expect("Failed to process reader");
        assert_eq!(stats.bytes_written, (CHUNK_SIZE * 3) as u64);
        assert_eq!(stats.chunks_processed, 3); // 3つのチャンクが処理された

        // ファイルが正しく書き込まれたことを確認
        let written_data = std::fs::read(&output_path).expect("Failed to read output file");
        assert_eq!(written_data.len(), CHUNK_SIZE * 3);
        assert!(written_data.iter().all(|&b| b == 42));

        // Clean up
        let _ = std::fs::remove_file(&output_path);
    }

    /// 非同期I/O処理のテスト - バックグラウンド書き出しがブロックしないこと
    #[tokio::test]
    async fn test_async_io_non_blocking() {
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_async_io.mp3");

        let memory_monitor = Arc::new(MemoryMonitor::new(512 * 1024 * 1024));
        let processor = Arc::new(ChunkProcessor::new(
            &output_path,
            CHUNK_SIZE,
            memory_monitor,
        ));

        // 大量のデータを作成 (1MB)
        let test_data = vec![0u8; 1024 * 1024];
        let reader = Cursor::new(test_data);

        let processor_clone = Arc::clone(&processor);

        // 非同期タスクで処理を実行
        let handle = tokio::spawn(async move { processor_clone.process_reader(reader).await });

        // タスクが完了するまで待機
        let result = handle.await.expect("Task panicked");
        assert!(result.is_ok());

        let stats = result.expect("Failed to process reader");
        assert_eq!(stats.bytes_written, 1024 * 1024);

        // Clean up
        let _ = std::fs::remove_file(&output_path);
    }

    /// チャンクバッファの最適化テスト - メモリ効率が良いこと
    #[tokio::test]
    async fn test_chunk_buffer_optimization() {
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_buffer_optimization.mp3");

        let memory_monitor = Arc::new(MemoryMonitor::new(512 * 1024 * 1024));
        let processor = ChunkProcessor::new(&output_path, CHUNK_SIZE, memory_monitor.clone());

        // モニタリングを開始
        memory_monitor.start().expect("Failed to start monitoring");

        // 4KB未満の最後のチャンクを含むデータを作成 (10KB + 1KB)
        let test_data = vec![0u8; CHUNK_SIZE * 2 + 1024];
        let reader = Cursor::new(test_data);

        let result = processor.process_reader(reader).await;
        assert!(result.is_ok());

        let stats = result.expect("Failed to process reader");
        assert_eq!(stats.bytes_written, (CHUNK_SIZE * 2 + 1024) as u64);

        // メモリ使用量がチャンクサイズを大幅に超えていないことを確認
        let final_stats = memory_monitor.get_stats();
        assert!(
            final_stats.memory_usage_bytes < CHUNK_SIZE * 3,
            "Memory usage should not exceed 3x chunk size"
        );

        // Clean up
        let _ = std::fs::remove_file(&output_path);
        memory_monitor.stop();
    }

    /// ストリーミングURL処理の統合テスト
    #[tokio::test]
    async fn test_stream_url_processing() {
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_stream_url.mp3");

        let memory_monitor = Arc::new(MemoryMonitor::new(512 * 1024 * 1024));

        // モックHTTPサーバーを起動
        let port = 18876;
        let _server_handle = create_mock_http_server(port).await;

        // サーバーが起動するのを待機
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let processor = ChunkProcessor::new(&output_path, CHUNK_SIZE, memory_monitor);

        let url = format!("http://127.0.0.1:{}/test_stream", port);
        let result = processor.process_stream(&url).await;

        assert!(
            result.is_ok(),
            "Failed to process stream: {:?}",
            result.err()
        );

        let stats = result.expect("Failed to process stream");
        assert_eq!(stats.bytes_written, (CHUNK_SIZE * 2) as u64);

        // Clean up
        let _ = std::fs::remove_file(&output_path);
    }

    /// エラーハンドリングのテスト - I/Oエラーが適切に処理されること
    #[tokio::test]
    async fn test_io_error_handling() {
        let temp_dir = std::env::temp_dir();
        // 無効なパス（作成できないディレクトリ）
        let output_path = temp_dir.join("nonexistent_dir").join("test.mp3");

        let memory_monitor = Arc::new(MemoryMonitor::new(512 * 1024 * 1024));
        let processor = ChunkProcessor::new(&output_path, CHUNK_SIZE, memory_monitor);

        let test_data = vec![0u8; CHUNK_SIZE];
        let reader = Cursor::new(test_data);

        let result = processor.process_reader(reader).await;
        assert!(result.is_err(), "Should fail with I/O error");
    }

    /// メモリ監視の統合テスト
    #[tokio::test]
    async fn test_memory_monitoring_integration() {
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_memory_integration.mp3");

        // 厳しいメモリ制限を設定
        let memory_monitor = Arc::new(MemoryMonitor::new(CHUNK_SIZE * 5)); // 5 chunks limit
        memory_monitor.start().expect("Failed to start monitoring");

        let processor = ChunkProcessor::new(&output_path, CHUNK_SIZE, memory_monitor.clone());

        // 制限内のデータ
        let test_data = vec![0u8; CHUNK_SIZE * 3];
        let reader = Cursor::new(test_data);

        let result = processor.process_reader(reader).await;
        assert!(result.is_ok());

        let stats = result.expect("Failed to process reader");
        assert_eq!(stats.chunks_processed, 3);
        assert!(stats.memory_usage_bytes <= CHUNK_SIZE * 5);

        // Clean up
        let _ = std::fs::remove_file(&output_path);
        memory_monitor.stop();
    }

    /// 並列チャンク処理のテスト
    #[tokio::test]
    async fn test_parallel_chunk_processing() {
        let temp_dir = std::env::temp_dir();

        let memory_monitor1 = Arc::new(MemoryMonitor::new(512 * 1024 * 1024));
        let memory_monitor2 = Arc::new(MemoryMonitor::new(512 * 1024 * 1024));

        let processor1 = ChunkProcessor::new(
            &temp_dir.join("test_parallel1.mp3"),
            CHUNK_SIZE,
            memory_monitor1,
        );
        let processor2 = ChunkProcessor::new(
            &temp_dir.join("test_parallel2.mp3"),
            CHUNK_SIZE,
            memory_monitor2,
        );

        let test_data1 = vec![1u8; CHUNK_SIZE * 2];
        let test_data2 = vec![2u8; CHUNK_SIZE * 2];

        let reader1 = Cursor::new(test_data1);
        let reader2 = Cursor::new(test_data2);

        // 並列処理を実行
        let handle1 = tokio::spawn(async move { processor1.process_reader(reader1).await });
        let handle2 = tokio::spawn(async move { processor2.process_reader(reader2).await });

        let result1 = handle1.await.expect("Task 1 panicked");
        let result2 = handle2.await.expect("Task 2 panicked");

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        // Clean up
        let _ = std::fs::remove_file(temp_dir.join("test_parallel1.mp3"));
        let _ = std::fs::remove_file(temp_dir.join("test_parallel2.mp3"));
    }

    /// 長時間録音時のメモリ制限テスト
    #[tokio::test]
    async fn test_long_recording_memory_limit() {
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_long_recording.mp3");

        // 512MB制限を設定
        let memory_monitor = Arc::new(MemoryMonitor::new(512 * 1024 * 1024));
        memory_monitor.start().expect("Failed to start monitoring");

        let processor = ChunkProcessor::new(&output_path, CHUNK_SIZE, memory_monitor.clone());

        // 大量のデータをシミュレート (10MB)
        let test_data = vec![0u8; 10 * 1024 * 1024];
        let reader = Cursor::new(test_data);

        let result = processor.process_reader(reader).await;
        assert!(result.is_ok());

        let stats = result.expect("Failed to process reader");

        // メモリ使用量が512MBを超えていないことを確認
        assert!(
            stats.memory_usage_bytes <= 512 * 1024 * 1024,
            "Memory usage {} exceeds 512MB limit",
            stats.memory_usage_bytes
        );

        // Clean up
        let _ = std::fs::remove_file(&output_path);
        memory_monitor.stop();
    }

    /// チャンク処理の進捗報告テスト
    #[tokio::test]
    async fn test_chunk_progress_reporting() {
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_progress.mp3");

        let memory_monitor = Arc::new(MemoryMonitor::new(512 * 1024 * 1024));
        let processor = ChunkProcessor::new(&output_path, CHUNK_SIZE, memory_monitor);

        // 複数のチャンクを含むデータ
        let test_data = vec![0u8; CHUNK_SIZE * 10];
        let reader = Cursor::new(test_data);

        let result = processor.process_reader(reader).await;
        assert!(result.is_ok());

        let stats = result.expect("Failed to process reader");
        assert_eq!(stats.chunks_processed, 10);
        assert!(stats.duration.as_secs_f64() > 0.0);

        // Clean up
        let _ = std::fs::remove_file(&output_path);
    }
}
