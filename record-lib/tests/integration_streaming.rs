//! Integration tests for streaming recording memory management.
//!
//! These tests verify:
//! - Memory usage monitoring during streaming
//! - 4KB chunk processing verification
//! - Long recording memory limit enforcement
//! - Memory leak detection
//! - Concurrent streaming operations

use futures_util::io::Cursor;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

use record_lib::streaming::chunk_processor::{ChunkProcessor, CHUNK_SIZE};
use record_lib::streaming::memory_monitor::MemoryMonitor;

/// Helper function to create a temporary test file path
fn temp_test_path(name: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("test_streaming_{name}.mp3"));
    path
}

/// Helper function to create test data of a specific size
fn create_test_data(size: usize) -> Vec<u8> {
    vec![42u8; size]
}

#[tokio::test]
async fn test_memory_monitor_initialization() {
    let monitor = MemoryMonitor::new(1024 * 1024); // 1MB limit

    assert_eq!(monitor.memory_limit(), 1024 * 1024);
    assert_eq!(monitor.current_usage(), 0);
    assert!((monitor.usage_percentage() - 0.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn test_memory_monitor_tracking() {
    let monitor = MemoryMonitor::new(1024);
    monitor.start().expect("Failed to start monitoring");

    // Simulate memory usage
    monitor.update_usage(512).expect("Failed to update usage");
    assert_eq!(monitor.current_usage(), 512);
    assert!((monitor.usage_percentage() - 50.0).abs() < 0.1);

    monitor.update_usage(256).expect("Failed to update usage");
    assert_eq!(monitor.current_usage(), 768);
    assert!((monitor.usage_percentage() - 75.0).abs() < 0.1);

    monitor.stop();
}

#[tokio::test]
async fn test_memory_limit_enforcement() {
    let monitor = MemoryMonitor::new(1000);
    monitor.start().expect("Failed to start monitoring");

    // Should allow usage up to limit
    monitor.update_usage(1000).expect("Failed to update usage");
    assert_eq!(monitor.current_usage(), 1000);

    // Should reject usage that exceeds limit
    let result = monitor.update_usage(1);
    assert!(result.is_err(), "Should reject usage exceeding limit");

    monitor.stop();
}

#[tokio::test]
async fn test_memory_limit_at_512mb() {
    let monitor = MemoryMonitor::new(512 * 1024 * 1024); // 512MB
    monitor.start().expect("Failed to start monitoring");

    // Should allow usage up to 512MB
    monitor
        .update_usage(512 * 1024 * 1024)
        .expect("Failed to update usage");
    assert_eq!(monitor.current_usage(), 512 * 1024 * 1024);

    // Should reject usage that exceeds 512MB
    let result = monitor.update_usage(1);
    assert!(result.is_err(), "Should reject usage exceeding 512MB limit");

    monitor.stop();
}

#[tokio::test]
async fn test_memory_warning_threshold() {
    let monitor = MemoryMonitor::new(1000);
    monitor.start().expect("Failed to start monitoring");

    // At 80% threshold, should still succeed but log warning
    let result = monitor.update_usage(800); // 80% of limit
    assert!(result.is_ok(), "Should allow usage at 80% threshold");

    // Slightly over 80% should also succeed with warning
    let result = monitor.update_usage(50); // 850 total (85%)
    assert!(result.is_ok(), "Should allow usage slightly over 80%");

    monitor.stop();
}

#[tokio::test]
async fn test_memory_leak_detection() {
    let monitor = MemoryMonitor::new(1024);
    monitor.start().expect("Failed to start monitoring");

    // First snapshot should always succeed
    assert!(monitor.detect_potential_leak().is_ok());

    // Add moderate usage - should not trigger leak detection
    monitor.update_usage(100).expect("Failed to update usage");
    assert!(monitor.detect_potential_leak().is_ok());

    // Add significant usage to trigger leak detection (>50% growth)
    monitor.update_usage(200).expect("Failed to update usage"); // Total: 300 bytes
    let result = monitor.detect_potential_leak();

    // This should detect the rapid growth
    assert!(result.is_err(), "Should detect potential memory leak");

    monitor.stop();
}

#[tokio::test]
async fn test_4kb_chunk_processing() {
    let output_path = temp_test_path("4kb_chunks");
    let memory_monitor = Arc::new(MemoryMonitor::new(512 * 1024 * 1024)); // 512MB limit
    let processor = ChunkProcessor::new(&output_path, CHUNK_SIZE, memory_monitor);

    // Create test data that's exactly 3 chunks (12KB)
    let test_data = create_test_data(CHUNK_SIZE * 3);
    let reader = Cursor::new(test_data);

    let result = processor.process_reader(reader).await;

    assert!(
        result.is_ok(),
        "Failed to process 4KB chunks: {:?}",
        result.err()
    );

    let stats = result.unwrap();
    assert_eq!(stats.bytes_written, (CHUNK_SIZE * 3) as u64);
    assert_eq!(stats.chunks_processed, 3);

    // Verify file was written correctly
    let written_data = std::fs::read(&output_path).expect("Failed to read output file");
    assert_eq!(written_data.len(), CHUNK_SIZE * 3);

    // Cleanup
    let _ = std::fs::remove_file(&output_path);
}

#[tokio::test]
async fn test_chunk_processing_with_partial_last_chunk() {
    let output_path = temp_test_path("partial_chunk");
    let memory_monitor = Arc::new(MemoryMonitor::new(512 * 1024 * 1024));
    let processor = ChunkProcessor::new(&output_path, CHUNK_SIZE, memory_monitor);

    // Create test data with partial last chunk (10KB + 1KB)
    let test_data = create_test_data(CHUNK_SIZE * 2 + 1024);
    let reader = Cursor::new(test_data);

    let result = processor.process_reader(reader).await;

    assert!(
        result.is_ok(),
        "Failed to process chunks with partial last chunk"
    );

    let stats = result.unwrap();
    assert_eq!(stats.bytes_written, (CHUNK_SIZE * 2 + 1024) as u64);
    assert_eq!(stats.chunks_processed, 3); // 2 full chunks + 1 partial

    // Cleanup
    let _ = std::fs::remove_file(&output_path);
}

#[tokio::test]
async fn test_chunk_processing_memory_limit() {
    let output_path = temp_test_path("memory_limit");
    // Live-memory cap smaller than a single chunk: the first chunk read must
    // exceed the cap and abort. Cumulative throughput no longer trips the limit
    // (see test_long_stream_does_not_abort_on_cumulative_bytes).
    let memory_monitor = Arc::new(MemoryMonitor::new(CHUNK_SIZE / 2));
    let processor = ChunkProcessor::new(&output_path, CHUNK_SIZE, memory_monitor);

    // Create test data that exceeds the per-chunk live-memory cap.
    let test_data = create_test_data(CHUNK_SIZE * 10);
    let reader = Cursor::new(test_data);

    let result = processor.process_reader(reader).await;

    assert!(
        result.is_err(),
        "Should fail when a single chunk exceeds the live-memory limit"
    );

    // Cleanup
    let _ = std::fs::remove_file(&output_path);
}

#[tokio::test]
async fn test_long_recording_memory_efficiency() {
    let output_path = temp_test_path("long_recording");
    let memory_monitor = Arc::new(MemoryMonitor::new(512 * 1024 * 1024)); // 512MB limit
    memory_monitor.start().expect("Failed to start monitoring");

    let processor = ChunkProcessor::new(&output_path, CHUNK_SIZE, memory_monitor.clone());

    // Simulate a long recording (10MB of data)
    let test_data = create_test_data(10 * 1024 * 1024);
    let reader = Cursor::new(test_data);

    let result = processor.process_reader(reader).await;

    assert!(result.is_ok(), "Failed to process long recording");

    let stats = result.unwrap();
    assert_eq!(stats.bytes_written, (10 * 1024 * 1024) as u64);

    // Verify memory usage stayed within limits
    let final_stats = memory_monitor.get_stats();
    assert!(
        final_stats.memory_usage_bytes <= 512 * 1024 * 1024,
        "Memory usage {} exceeds 512MB limit",
        final_stats.memory_usage_bytes
    );

    // Cleanup
    let _ = std::fs::remove_file(&output_path);
    memory_monitor.stop();
}

#[tokio::test]
async fn test_concurrent_chunk_processing() {
    let output_path1 = temp_test_path("concurrent1");
    let output_path2 = temp_test_path("concurrent2");

    let memory_monitor1 = Arc::new(MemoryMonitor::new(512 * 1024 * 1024));
    let memory_monitor2 = Arc::new(MemoryMonitor::new(512 * 1024 * 1024));

    let processor1 = ChunkProcessor::new(&output_path1, CHUNK_SIZE, memory_monitor1);
    let processor2 = ChunkProcessor::new(&output_path2, CHUNK_SIZE, memory_monitor2);

    let test_data1 = create_test_data(CHUNK_SIZE * 5);
    let test_data2 = create_test_data(CHUNK_SIZE * 5);

    let reader1 = Cursor::new(test_data1);
    let reader2 = Cursor::new(test_data2);

    // Process concurrently
    let handle1 = tokio::spawn(async move { processor1.process_reader(reader1).await });
    let handle2 = tokio::spawn(async move { processor2.process_reader(reader2).await });

    let result1 = handle1.await.expect("Task 1 panicked");
    let result2 = handle2.await.expect("Task 2 panicked");

    assert!(result1.is_ok(), "Concurrent task 1 failed");
    assert!(result2.is_ok(), "Concurrent task 2 failed");

    let stats1 = result1.unwrap();
    let stats2 = result2.unwrap();

    assert_eq!(stats1.bytes_written, (CHUNK_SIZE * 5) as u64);
    assert_eq!(stats2.bytes_written, (CHUNK_SIZE * 5) as u64);

    // Cleanup
    let _ = std::fs::remove_file(&output_path1);
    let _ = std::fs::remove_file(&output_path2);
}

#[tokio::test]
async fn test_memory_monitor_cleanup() {
    let monitor = MemoryMonitor::new(1024);
    monitor.start().expect("Failed to start monitoring");

    // Add some usage
    monitor.update_usage(500).expect("Failed to update usage");
    assert_eq!(monitor.current_usage(), 500);

    // Stop monitoring and verify cleanup
    monitor.stop();
    assert_eq!(monitor.current_usage(), 0);
}

#[tokio::test]
async fn test_memory_monitor_concurrent_updates() {
    use std::sync::Arc;

    let monitor = Arc::new(MemoryMonitor::new(10_000));
    monitor.start().expect("Failed to start monitoring");

    let mut handles = vec![];

    // Spawn multiple tasks to test concurrent updates
    for _ in 0..5 {
        let monitor_clone = Arc::clone(&monitor);
        let handle = tokio::spawn(async move {
            for _ in 0..10 {
                let _ = monitor_clone.update_usage(100);
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.expect("Task panicked");
    }

    // Total should be 5000 (5 tasks * 10 iterations * 100 bytes)
    assert_eq!(monitor.current_usage(), 5000);

    monitor.stop();
}

#[tokio::test]
async fn test_chunk_processing_timeout() {
    let output_path = temp_test_path("timeout");
    let memory_monitor = Arc::new(MemoryMonitor::new(512 * 1024 * 1024));
    let processor = ChunkProcessor::new(&output_path, CHUNK_SIZE, memory_monitor);

    // Create test data
    let test_data = create_test_data(CHUNK_SIZE);
    let reader = Cursor::new(test_data);

    // Set a timeout of 5 seconds
    let result = timeout(Duration::from_secs(5), processor.process_reader(reader)).await;

    assert!(result.is_ok(), "Processing should complete within timeout");
    assert!(result.unwrap().is_ok(), "Processing should succeed");

    // Cleanup
    let _ = std::fs::remove_file(&output_path);
}

#[tokio::test]
async fn test_memory_stats_accuracy() {
    let monitor = MemoryMonitor::new(1024);
    monitor.start().expect("Failed to start monitoring");

    // Add some usage
    monitor.update_usage(512).expect("Failed to update usage");
    monitor.update_usage(256).expect("Failed to update usage");

    let stats = monitor.get_stats();
    assert_eq!(stats.memory_usage_bytes, 768);
    assert_eq!(stats.bytes_written, 768);
    assert_eq!(stats.chunks_processed, 2);

    monitor.stop();
}

#[tokio::test]
async fn test_chunk_processing_with_empty_data() {
    let output_path = temp_test_path("empty");
    let memory_monitor = Arc::new(MemoryMonitor::new(512 * 1024 * 1024));
    let processor = ChunkProcessor::new(&output_path, CHUNK_SIZE, memory_monitor);

    // Create empty test data
    let test_data = vec![0u8; 0];
    let reader = Cursor::new(test_data);

    let result = processor.process_reader(reader).await;

    assert!(result.is_ok(), "Should handle empty data gracefully");

    let stats = result.unwrap();
    assert_eq!(stats.bytes_written, 0);
    assert_eq!(stats.chunks_processed, 0);

    // Cleanup
    let _ = std::fs::remove_file(&output_path);
}

#[tokio::test]
async fn test_memory_monitor_reset() {
    let monitor = MemoryMonitor::new(1024);
    monitor.start().expect("Failed to start monitoring");

    monitor.update_usage(512).expect("Failed to update usage");
    assert_eq!(monitor.current_usage(), 512);

    monitor.reset_usage();
    assert_eq!(monitor.current_usage(), 0);

    monitor.stop();
}

#[tokio::test]
async fn test_background_logging_stops_gracefully() {
    let monitor = MemoryMonitor::new(1024);
    monitor.start().expect("Failed to start monitoring");

    // Start background logging with short interval
    monitor.start_background_logging(1);

    // Let it run briefly
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Stop monitoring
    monitor.stop();

    // Background task should exit gracefully
    tokio::time::sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_chunk_size_validation() {
    let output_path = temp_test_path("size_validation");
    let memory_monitor = Arc::new(MemoryMonitor::new(512 * 1024 * 1024));

    // Test with different chunk sizes
    for chunk_size in [1024, 2048, 4096, 8192] {
        let processor = ChunkProcessor::new(&output_path, chunk_size, memory_monitor.clone());

        let test_data = create_test_data(chunk_size * 2);
        let reader = Cursor::new(test_data);

        let result = processor.process_reader(reader).await;
        assert!(result.is_ok(), "Failed with chunk size {}", chunk_size);

        let stats = result.unwrap();
        assert_eq!(
            stats.chunks_processed, 2,
            "Chunk count mismatch for size {chunk_size}",
        );
    }

    // Cleanup
    let _ = std::fs::remove_file(&output_path);
}

#[tokio::test]
async fn test_memory_monitor_stats_duration() {
    let monitor = MemoryMonitor::new(1024);
    monitor.start().expect("Failed to start monitoring");

    // Add some usage
    monitor.update_usage(512).expect("Failed to update usage");

    // Get stats immediately
    let stats1 = monitor.get_stats();
    assert!(stats1.duration.as_secs_f64() >= 0.0);

    // Wait a bit and get stats again
    tokio::time::sleep(Duration::from_millis(100)).await;
    let stats2 = monitor.get_stats();

    // Duration should have increased
    assert!(stats2.duration >= stats1.duration);

    monitor.stop();
}
