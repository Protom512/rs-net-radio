//! Memory monitoring for streaming operations.
//!
//! This module provides memory usage monitoring and limiting functionality
//! for streaming audio operations.

use anyhow::Result;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, warn};

use tokio::time::{interval, Duration};

/// Statistics about memory usage during streaming operations.
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Current memory usage in bytes.
    pub memory_usage_bytes: usize,
    /// Total bytes written to disk.
    pub bytes_written: u64,
    /// Duration of the operation.
    pub duration: std::time::Duration,
    /// Number of chunks processed.
    pub chunks_processed: u32,
}

/// Memory monitor for tracking and limiting memory usage.
///
/// This struct monitors memory usage during streaming operations
/// and can enforce limits to prevent excessive memory consumption.
pub struct MemoryMonitor {
    /// Memory limit in bytes.
    memory_limit: usize,
    /// Current memory usage in bytes.
    current_usage: Arc<AtomicU64>,
    /// Total bytes processed.
    total_bytes: Arc<AtomicU64>,
    /// Number of chunks processed.
    chunk_count: Arc<AtomicU32>,
    /// Whether monitoring is active.
    pub(crate) monitoring_active: Arc<std::sync::atomic::AtomicBool>,
    /// Start time for duration tracking.
    start_time: Arc<std::sync::Mutex<Option<std::time::Instant>>>,
    /// Previous usage snapshot for leak detection.
    previous_usage: Arc<std::sync::Mutex<usize>>,
    /// Snapshot counter for leak detection.
    snapshot_count: Arc<std::sync::atomic::AtomicU32>,
}

impl MemoryMonitor {
    /// Creates a new `MemoryMonitor`.
    ///
    /// # Arguments
    ///
    /// * `memory_limit` - Maximum memory usage in bytes.
    #[must_use]
    pub fn new(memory_limit: usize) -> Self {
        Self {
            memory_limit,
            current_usage: Arc::new(AtomicU64::new(0)),
            total_bytes: Arc::new(AtomicU64::new(0)),
            chunk_count: Arc::new(AtomicU32::new(0)),
            monitoring_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            start_time: Arc::new(std::sync::Mutex::new(None)),
            previous_usage: Arc::new(std::sync::Mutex::new(0)),
            snapshot_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }

    /// Starts monitoring memory usage.
    ///
    /// # Errors
    ///
    /// Returns an error if monitoring is already active.
    pub fn start(&self) -> Result<()> {
        if self.monitoring_active.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!("Memory monitoring is already active"));
        }

        self.monitoring_active.store(true, Ordering::Relaxed);

        if let Ok(mut guard) = self.start_time.lock() {
            *guard = Some(std::time::Instant::now());
        } else {
            return Err(anyhow::anyhow!("Failed to acquire lock for start_time"));
        }

        debug!(
            "Started memory monitoring with limit: {} bytes ({} MB)",
            self.memory_limit,
            self.memory_limit / 1024 / 1024
        );

        Ok(())
    }

    /// Stops monitoring memory usage.
    pub fn stop(&self) {
        self.monitoring_active.store(false, Ordering::Relaxed);

        // Perform cleanup to ensure resources are freed
        self.current_usage.store(0, Ordering::Relaxed);
        self.chunk_count.store(0, Ordering::Relaxed);

        // Reset leak detection state
        if let Ok(mut prev_usage) = self.previous_usage.lock() {
            *prev_usage = 0;
        }
        self.snapshot_count.store(0, Ordering::Relaxed);

        debug!("Stopped memory monitoring and performed cleanup");
    }

    /// Detects potential memory leaks by comparing current usage with previous snapshots.
    ///
    /// This method should be called periodically to check if memory usage
    /// is consistently growing without corresponding resets, which could indicate a leak.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - No leak detected
    /// * `Err(String)` - Potential leak detected with details
    ///
    /// # Errors
    ///
    /// Returns `Err` if a potential memory leak is detected (growth > 50%).
    pub fn detect_potential_leak(&self) -> Result<(), String> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "atomic values are expected to fit in usize on target platforms"
        )]
        let current = self.current_usage.load(Ordering::Relaxed) as usize;

        let mut prev_usage = self
            .previous_usage
            .lock()
            .map_err(|e| format!("Failed to acquire lock for leak detection: {e}"))?;

        let snapshot_num = self.snapshot_count.fetch_add(1, Ordering::Relaxed);

        // On first snapshot, just record current usage
        if snapshot_num == 0 {
            *prev_usage = current;
            return Ok(());
        }

        // Check if usage is growing consistently
        if current > *prev_usage {
            let growth = current - *prev_usage;
            #[expect(
                clippy::cast_precision_loss,
                reason = "f64 precision is acceptable for percentage display"
            )]
            let growth_percentage = if *prev_usage > 0 {
                (growth as f64 / *prev_usage as f64) * 100.0
            } else {
                0.0
            };

            // If growth is more than 50% between snapshots, warn about potential leak
            if growth_percentage > 50.0 {
                warn!(
                    "Potential memory leak detected: usage grew by {} bytes ({:.1}%) since last snapshot",
                    growth, growth_percentage
                );
                return Err(format!(
                    "Memory usage grew by {} bytes ({:.1}%) from {} to {} bytes",
                    growth, growth_percentage, *prev_usage, current
                ));
            }
        }

        // Update previous usage for next comparison
        *prev_usage = current;
        Ok(())
    }

    /// Checks if the current memory usage exceeds the limit.
    ///
    /// # Errors
    ///
    /// Returns an error if the memory limit is exceeded.
    pub fn check_memory_limit(&self) -> Result<()> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "atomic values are expected to fit in usize on target platforms"
        )]
        let current = self.current_usage.load(Ordering::Relaxed) as usize;

        if current > self.memory_limit {
            warn!(
                "Memory limit exceeded: {} bytes > {} bytes",
                current, self.memory_limit
            );
            return Err(anyhow::anyhow!(
                "Memory limit exceeded: {} bytes > {} bytes",
                current,
                self.memory_limit
            ));
        }

        // Warning at 80% of limit
        if current > (self.memory_limit * 4 / 5) {
            #[expect(
                clippy::cast_precision_loss,
                reason = "f64 precision is acceptable for percentage display"
            )]
            let percentage = (current as f64 / self.memory_limit as f64) * 100.0;
            warn!(
                "Memory usage approaching limit: {} bytes ({}% of limit)",
                current, percentage
            );
        }

        Ok(())
    }

    /// Updates the current memory usage.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Number of bytes to add to current usage.
    ///
    /// # Errors
    ///
    /// Returns an error if the update would exceed the memory limit.
    pub fn update_usage(&self, bytes: usize) -> Result<()> {
        self.current_usage
            .fetch_add(bytes as u64, Ordering::Relaxed);
        self.total_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
        self.chunk_count.fetch_add(1, Ordering::Relaxed);

        debug!(
            "Memory usage: {} bytes, chunks: {}",
            self.current_usage.load(Ordering::Relaxed),
            self.chunk_count.load(Ordering::Relaxed)
        );

        self.check_memory_limit()
    }

    /// Resets the current memory usage to zero.
    pub fn reset_usage(&self) {
        self.current_usage.store(0, Ordering::Relaxed);
        self.chunk_count.store(0, Ordering::Relaxed);
        debug!("Reset memory usage to zero");
    }

    /// Gets the current memory usage statistics.
    #[must_use]
    pub fn get_stats(&self) -> MemoryStats {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "atomic values are expected to fit in usize on target platforms"
        )]
        let usage = self.current_usage.load(Ordering::Relaxed) as usize;
        let total = self.total_bytes.load(Ordering::Relaxed);
        let chunks = self.chunk_count.load(Ordering::Relaxed);

        let duration = if let Ok(guard) = self.start_time.lock() {
            if let Some(start) = *guard {
                start.elapsed()
            } else {
                std::time::Duration::from_secs(0)
            }
        } else {
            debug!("Failed to acquire lock for start_time, using zero duration");
            std::time::Duration::from_secs(0)
        };

        MemoryStats {
            memory_usage_bytes: usage,
            bytes_written: total,
            duration,
            chunks_processed: chunks,
        }
    }

    /// Gets the current memory usage in bytes.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "atomic values are expected to fit in usize on target platforms"
    )]
    pub fn current_usage(&self) -> usize {
        self.current_usage.load(Ordering::Relaxed) as usize
    }

    /// Gets the memory limit in bytes.
    #[must_use]
    pub fn memory_limit(&self) -> usize {
        self.memory_limit
    }

    /// Gets the current memory usage as a percentage of the limit.
    #[must_use]
    pub fn usage_percentage(&self) -> f64 {
        if self.memory_limit == 0 {
            return 0.0;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "f64 precision is acceptable for percentage display"
        )]
        let percentage =
            (self.current_usage.load(Ordering::Relaxed) as f64 / self.memory_limit as f64) * 100.0;
        percentage
    }

    /// Starts a background task to periodically log memory usage.
    ///
    /// # Arguments
    ///
    /// * `interval_secs` - Interval between log updates in seconds.
    pub fn start_background_logging(&self, interval_secs: u64) {
        let current_usage = Arc::clone(&self.current_usage);
        let memory_limit = self.memory_limit;
        let monitoring_active = Arc::clone(&self.monitoring_active);

        tokio::spawn(async move {
            let mut timer = interval(Duration::from_secs(interval_secs));

            loop {
                timer.tick().await;

                if !monitoring_active.load(Ordering::Relaxed) {
                    break;
                }

                #[allow(clippy::cast_possible_truncation)]
                let usage = current_usage.load(Ordering::Relaxed) as usize;
                #[allow(clippy::cast_precision_loss)]
                let percentage = if memory_limit > 0 {
                    (usage as f64 / memory_limit as f64) * 100.0
                } else {
                    0.0
                };

                debug!(
                    "Memory usage: {} bytes / {} bytes ({:.1}%)",
                    usage, memory_limit, percentage
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_monitor_creation() {
        let monitor = MemoryMonitor::new(1024 * 1024); // 1MB
        assert_eq!(monitor.memory_limit(), 1024 * 1024);
        assert_eq!(monitor.current_usage(), 0);
        assert!((monitor.usage_percentage() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_memory_monitor_update() {
        let monitor = MemoryMonitor::new(1024);
        monitor.start().unwrap();

        monitor.update_usage(512).unwrap();
        assert_eq!(monitor.current_usage(), 512);
        assert!((monitor.usage_percentage() - 50.0).abs() < 0.1);

        monitor.update_usage(256).unwrap();
        assert_eq!(monitor.current_usage(), 768);
        assert!((monitor.usage_percentage() - 75.0).abs() < 0.1);
    }

    #[test]
    fn test_memory_limit_exceeded() {
        let monitor = MemoryMonitor::new(1000);
        monitor.start().unwrap();

        monitor.update_usage(500).unwrap();
        assert!(monitor.update_usage(600).is_err());
    }

    #[test]
    fn test_memory_monitor_reset() {
        let monitor = MemoryMonitor::new(1024);
        monitor.start().unwrap();

        monitor.update_usage(512).unwrap();
        assert_eq!(monitor.current_usage(), 512);

        monitor.reset_usage();
        assert_eq!(monitor.current_usage(), 0);
    }

    #[test]
    fn test_memory_stats() {
        let monitor = MemoryMonitor::new(1024);
        monitor.start().unwrap();

        monitor.update_usage(512).unwrap();
        monitor.update_usage(256).unwrap();

        let stats = monitor.get_stats();
        assert_eq!(stats.memory_usage_bytes, 768);
        assert_eq!(stats.bytes_written, 768);
        assert_eq!(stats.chunks_processed, 2);
    }

    #[test]
    fn test_512mb_limit_enforcement() {
        let monitor = MemoryMonitor::new(512 * 1024 * 1024); // 512MB
        monitor.start().unwrap();

        // Should allow usage up to limit
        monitor.update_usage(512 * 1024 * 1024).unwrap();
        assert_eq!(monitor.current_usage(), 512 * 1024 * 1024);
    }

    #[test]
    fn test_warning_threshold_at_80_percent() {
        let monitor = MemoryMonitor::new(1000);
        monitor.start().unwrap();

        // At 80% threshold, should still succeed but log warning
        let result = monitor.update_usage(800); // 80% of limit
        assert!(result.is_ok());

        // Slightly over 80% should also succeed with warning
        let result = monitor.update_usage(50); // 850 total (85%)
        assert!(result.is_ok());
    }

    #[test]
    fn test_memory_leak_detection() {
        let monitor = MemoryMonitor::new(1024);
        monitor.start().unwrap();

        // Simulate normal operation
        monitor.update_usage(100).unwrap();
        monitor.reset_usage();

        // Simulate potential leak - usage keeps growing without resets
        monitor.update_usage(200).unwrap();
        let stats1 = monitor.get_stats();

        monitor.update_usage(300).unwrap();
        let stats2 = monitor.get_stats();

        // Usage should be increasing
        assert!(stats2.memory_usage_bytes > stats1.memory_usage_bytes);
    }

    #[test]
    fn test_detect_potential_leak() {
        let monitor = MemoryMonitor::new(1024);
        monitor.start().unwrap();

        // First snapshot should always succeed
        assert!(monitor.detect_potential_leak().is_ok());

        // Add moderate usage - should not trigger leak detection
        monitor.update_usage(100).unwrap();
        assert!(monitor.detect_potential_leak().is_ok());

        // Add significant usage to trigger leak detection (>50% growth)
        monitor.update_usage(200).unwrap(); // Total: 300 bytes
        let result = monitor.detect_potential_leak();

        // This should detect the rapid growth
        assert!(result.is_err());
    }

    #[test]
    fn test_stop_performs_cleanup() {
        let monitor = MemoryMonitor::new(1024);
        monitor.start().unwrap();

        // Add some usage
        monitor.update_usage(500).unwrap();
        assert_eq!(monitor.current_usage(), 500);

        // Stop monitoring
        monitor.stop();

        // Verify cleanup was performed
        assert_eq!(monitor.current_usage(), 0);
        assert!(!monitor
            .monitoring_active
            .load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn test_resource_cleanup_on_limit_exceeded() {
        let monitor = MemoryMonitor::new(1000);
        monitor.start().unwrap();

        monitor.update_usage(500).unwrap();

        // This should fail and ensure resources are tracked
        let result = monitor.update_usage(600);
        assert!(result.is_err());

        // Verify monitoring state is still consistent
        let stats = monitor.get_stats();
        assert_eq!(stats.chunks_processed, 2);
    }

    #[test]
    fn test_concurrent_usage_tracking() {
        use std::thread;

        let monitor = std::sync::Arc::new(MemoryMonitor::new(10_000));
        monitor.start().unwrap();

        let mut handles = vec![];

        // Spawn multiple threads to test concurrent access
        for _ in 0..5 {
            let monitor_clone = std::sync::Arc::clone(&monitor);
            let handle = thread::spawn(move || {
                for _ in 0..10 {
                    let _ = monitor_clone.update_usage(100);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Total should be 5000 (5 threads * 10 iterations * 100 bytes)
        assert_eq!(monitor.current_usage(), 5000);
    }

    #[tokio::test]
    async fn test_background_logging_stops_when_monitoring_stops() {
        let monitor = MemoryMonitor::new(1024);
        monitor.start().unwrap();

        // Start background logging with short interval
        monitor.start_background_logging(1);

        // Let it run briefly
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Stop monitoring
        monitor.stop();

        // Background task should exit gracefully
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify monitoring is stopped
        assert!(!monitor
            .monitoring_active
            .load(std::sync::atomic::Ordering::Relaxed));
    }
}
