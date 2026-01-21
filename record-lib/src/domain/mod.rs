//! Domain layer for recording services.
//!
//! This module contains core business logic and abstractions,
//! independent of external frameworks and libraries.

pub mod metadata;
pub mod service;
pub mod types;

pub use metadata::RecordingMetadata;
pub use service::RecordService;
pub use types::{OutputPath, ProgramTitle, StreamingUrl};
