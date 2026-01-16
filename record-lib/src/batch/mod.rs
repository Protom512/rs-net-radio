//! Batch recording functionality.
//!
//! This module provides the `BatchRecorder` which handles parallel recording
//! of multiple programs with progress tracking.

pub mod recorder;

pub use recorder::{BatchRecorder, BatchSummary};
