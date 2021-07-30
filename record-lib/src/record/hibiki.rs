use crate::lib::record::Record;
use chrono::{Date, DateTime, Datelike, Duration, Local, Timelike};
use log::{debug, error, info};
use std::path::Path;
use std::process::{Command, ExitStatus};
#[derive(Clone, Debug)]
pub struct Hibiki {
    pub title: String,
    pub start_datetime: DateTime<Local>,
    pub end_datetime: DateTime<Local>,
}

impl Record for Hibiki {
    fn new(
        title: &str,
        start_datetime: &DateTime<Local>,
        end_datetime: &DateTime<Local>,
    ) -> Hibiki {
        Hibiki {
            title: title.to_string(),
            start_datetime: *start_datetime,
            end_datetime: *end_datetime,
        }
    }

    fn record(self, output_path: String, duration: Duration) -> Result<ExitStatus, std::io::Error> {
        //unimplemented!();
        let start = self.start_datetime + Duration::seconds(-15);
        let file_name = format!(
            "{}_{}.mp4",
            self.start_datetime.format("%Y%m%d_%H%M%S"),
            self.title
        );
        let path_string = format!("{}/{}", output_path, file_name);
        debug!("{}", path_string);
        info!("Title: {}", self.title);
        info!("StartTime: {}", self.start_datetime);
        let path = Path::new(&path_string);
        path.exists();
        let arg = self.end_datetime - start;
        info!("Duration: {}", arg);

        let agqr_stream_url = "https://fms2.uniqueradio.jp/agqr10/aandg1.m3u8";
        let res = Command::new("streamlink")
            .arg(agqr_stream_url)
            .arg("best")
            .arg("-o")
            .arg(path)
            .arg("-l")
            .arg("info")
            .arg("--hls-duration")
            .arg(format!(
                "{}:{}:{}",
                arg.num_hours(),
                arg.num_minutes() % 60,
                arg.num_seconds() % 60
            ))
            .status();
        res
    }
}
