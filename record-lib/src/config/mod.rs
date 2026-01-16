//! Configuration management for the recording system.
//!
//! This module provides configuration structures and repository pattern
//! for managing application settings from TOML files.

pub mod manager;
pub mod repository;

pub use manager::{Config, CronConfig, Schedule};
pub use repository::{ConfigRepository, FileConfigRepository};
