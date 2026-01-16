/// Module for handling hibiki-radio.jp radio recordings.
pub mod hibiki;
/// Module for hibiki-radio.jp HTML scraping and URL extraction.
pub mod hibiki_scraper;
/// Module for handling onsen.ag radio recordings.
pub mod onsen;
/// Module for handling radiko.jp radio recordings.
pub mod radiko;

use chrono::{DateTime, Duration, Local};

use std::process::ExitStatus;

/// A trait for recording internet radio programs.
pub trait Record {
    /// Creates a new recording task.
    ///
    /// # Arguments
    ///
    /// * `title` - The title of the program.
    /// * `start_datetime` - The start date and time of the recording.
    /// * `end_datetime` - The end date and time of the recording.
    fn new(title: &str, start_datetime: &DateTime<Local>, end_datetime: &DateTime<Local>) -> Self;

    /// Starts the recording process.
    ///
    /// # Arguments
    ///
    /// * `output_path` - The path where the recorded file will be saved.
    /// * `duration` - The duration of the recording.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether the recording was successful or an I/O error occurred.
    fn record(self, output_path: String, duration: Duration) -> Result<ExitStatus, std::io::Error>;
}
