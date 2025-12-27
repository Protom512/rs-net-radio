use crate::utils::{ensure_archive_path, RecordError};
use chrono::{DateTime, Duration, Local, NaiveDate};
use fs_extra;
use fs_extra::file::CopyOptions;
use log::{error, info};
use std::env::temp_dir; // Added RecordError

use std::fmt::Debug;
use std::path::Path;
use std::process::Command;
use std::process::ExitStatus;
use std::str;

const AG_PROGRAM_URL: &str = "https://www.joqr.co.jp/qr/agdailyprogram/agdailyprogram.html";

pub fn get_html_from_url(url: &str) -> Result<reqwest::blocking::Response, RecordError> {
    match reqwest::blocking::get(url) {
        Ok(n) => Ok(n),
        Err(e) => {
            error!("reqwest::blocking::get failed for url {}: {}", url, e);
            Err(RecordError::Reqwest(e))
        }
    }
}

/// Represents an AGQR program recording task.
#[derive(Clone, Debug, PartialEq)]
pub struct Ag {
    /// The title of the program.
    pub title: String,
    /// The start date and time of the program.
    pub start_datetime: DateTime<Local>,
    /// The end date and time of the program.
    pub end_datetime: DateTime<Local>,
}

impl Ag {
    ///
    ///  # ag+の録画関数
    ///
    pub fn record(self) -> Result<ExitStatus, RecordError> {
        let start = self.start_datetime + Duration::seconds(-15);
        let archive_path = ensure_archive_path("ag")?;

        let tmpdir = temp_dir().to_str().ok_or(RecordError::TempDir)?.to_string();
        info!("working path: {}", tmpdir); // Moved info log after ok_or

        let file_name = format!(
            "{}_{}.mp4",
            self.start_datetime.format("%Y%m%d_%H%M%S"),
            self.title
        );
        let working_path = format!("{}/{}", &tmpdir, &file_name);
        info!("Title: {}", self.title);
        info!("StartTime: {}", self.start_datetime);
        let path = Path::new(&working_path);
        if path.exists() {
            "testign file exists, removing it".to_string();
        }
        let arg = self.end_datetime - start;
        info!("Duration: {}", arg);

        let agqr_stream_url = "https://fms2.uniqueradio.jp/agqr10/aandg1.m3u8";
        let status_result = Command::new("streamlink")
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

        let status = status_result?; // Propagates io::Error
        if !status.success() {
            return Err(RecordError::CommandFailed {
                command: "streamlink".to_string(),
                exit_code: status.code(),
                stderr: "Streamlink execution failed, no stderr captured by .status()".to_string(),
            });
        }

        info!("EndTime: {}", self.end_datetime);
        info!("ExitStatus:{}", status);

        let options = CopyOptions::new();
        fs_extra::file::move_file(
            &working_path,
            format!("{}/{}", archive_path, &file_name),
            &options,
        )
        .map_err(|e| RecordError::Other(format!("Failed to move file: {}", e)))?; // Simplified fs_extra error mapping

        Ok(status)
    }

    /// Creates a new `Ag` instance.
    ///
    /// # Arguments
    ///
    /// * `title` - The title of the program.
    /// * `start_datetime` - The start date and time of the program.
    /// * `end_datetime` - The end date and time of the program.
    ///
    /// # Returns
    ///
    /// A new `Ag` instance.
    pub fn new(
        title: &str,
        start_datetime: &DateTime<Local>,
        end_datetime: &DateTime<Local>,
    ) -> Ag {
        Ag {
            title: title.to_string(),
            start_datetime: *start_datetime,
            end_datetime: *end_datetime,
        }
    }

    /// Parses the HTML content of the AGQR daily program schedule page and extracts program information.
    ///
    /// # Arguments
    ///
    /// * `get_result` - A `reqwest::blocking::Response` containing the HTML content.
    ///
    /// # Returns
    ///
    /// A vector of `Ag` instances representing the programs in the schedule.
    ///
    /// # Panics
    ///
    /// Panics if the HTML content cannot be parsed or if expected elements are not found.
    pub fn html_parse(html_body: &str) -> Vec<Ag> {
        let body = html_body.to_string();

        let selector_fragment =
            scraper::Selector::parse("article.dailyProgram-itemBox.ag ").unwrap();
        let selector = scraper::Selector::parse(" div.dailyProgram-itemContainer >div.js-readmore> div.dailyProgram-itemDetail > p.dailyProgram-itemTitle >a").unwrap();
        let selector_time = scraper::Selector::parse(" div.dailyProgram-itemHeader >h3").unwrap();

        let document = scraper::Html::parse_document(&body);

        for x in &document.errors {
            error!("{}", x)
        }
        // セレクターを用いて要素を取得
        let elements = document.select(&selector_fragment);
        let mut arr = Vec::<Ag>::new();
        let mut datetime_str;
        let now: DateTime<Local> = Local::now();
        let local_date: NaiveDate = now.date_naive();
        for i in elements {
            let mut start_offset_h: Duration = Duration::hours(0);
            let mut start_offset_m: Duration = Duration::minutes(0);
            let mut end_offset_h: Duration = Duration::hours(0);
            let mut end_offset_m: Duration = Duration::minutes(0);
            let mut title = "";
            for j in i.select(&selector) {
                title = j.text().next().unwrap();
            }

            for j in i.select(&selector_time) {
                datetime_str = j.text().next().unwrap();
                let vec_str: Vec<&str> = datetime_str.split(" – ").collect();
                let start_t = vec_str[0];
                let end_t = vec_str[1];
                let start_vec: Vec<&str> = start_t.split(':').collect();
                let start_h: i64 = start_vec[0].parse::<i64>().unwrap();
                let start_m: i64 = start_vec[1].parse::<i64>().unwrap();
                start_offset_h = Duration::hours(start_h);
                start_offset_m = Duration::minutes(start_m);
                let end_vec: Vec<&str> = end_t.split(':').collect();
                let end_h: i64 = end_vec[0].parse::<i64>().unwrap();
                let end_m: i64 = end_vec[1].parse::<i64>().unwrap();
                end_offset_h = Duration::hours(end_h);
                end_offset_m = Duration::minutes(end_m);
            }
            let start_hms =
                local_date.and_hms_opt(0, 0, 0).unwrap() + start_offset_h + start_offset_m;
            let end_hms = local_date.and_hms_opt(0, 0, 0).unwrap() + end_offset_h + end_offset_m;
            arr.push(Ag::new(
                title,
                &start_hms.and_local_timezone(Local).unwrap(),
                &end_hms.and_local_timezone(Local).unwrap(),
            ));
        }
        arr
    }

    /// Initializes a list of `Ag` tasks by fetching and parsing the AGQR daily program schedule.
    ///
    /// # Returns
    ///
    /// A vector of `Ag` instances representing the programs in the schedule.
    pub fn init() -> Vec<Ag> {
        match get_html_from_url(AG_PROGRAM_URL) {
            // Call the new function
            Ok(response) => match response.text() {
                Ok(text) => Ag::html_parse(&text),
                Err(e) => {
                    error!("Failed to get text from HTTP response: {}", e);
                    Vec::new()
                }
            },
            Err(e) => {
                error!("Failed to get HTML for A&G: {}", e);
                Vec::new()
            }
        }
    }
}
