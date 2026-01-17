//! This library provides functionalities to record internet radio streams
//! from various Japanese radio services, including Radiko, AGQR, Onsen, and Hibiki Radio.
pub mod batch;
pub mod common;
pub mod config;
pub mod domain;
pub mod error;
pub mod logging;
pub mod record;
pub mod scheduler;
pub mod streaming;
pub mod utils;

// Re-export commonly used types
pub use error::{RecordingError, Result};
// Re-export common FFmpeg utilities
pub use common::{FfmpegCommand, FfmpegInput, FfmpegOutput};
