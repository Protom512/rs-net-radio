pub mod ag;
pub mod hibiki;
pub mod onsen;
pub mod radiko;

use chrono::{DateTime, Duration, Local};

use std::process::ExitStatus;
pub trait Record {
    fn new(title: &str, start_datetime: &DateTime<Local>, end_datetime: &DateTime<Local>) -> Self;
    fn record(self, output_path: String, duration: Duration) -> Result<ExitStatus, std::io::Error>;
}
