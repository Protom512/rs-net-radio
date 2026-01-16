//! This library provides functionalities to record internet radio streams
//! from various Japanese radio services, including Radiko, AGQR, Onsen, and Hibiki Radio.
pub mod batch;
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
