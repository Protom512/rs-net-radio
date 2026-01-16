//! Cron-based scheduling for automated recording.
//!
//! This module provides the `CronManager` which handles scheduled recording tasks
//! using tokio-cron-scheduler.

pub mod cron_manager;

pub use cron_manager::CronManager;
