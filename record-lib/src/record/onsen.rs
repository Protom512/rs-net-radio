use fs_extra;
use fs_extra::file::CopyOptions;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use crate::utils::{ensure_archive_path, RecordError}; // Added RecordError
use std::env::temp_dir;
// use std::fs; // Removed
use std::path::Path;
use std::process::Command;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OnsenProgramContents {
    id: u32,
    title: String,
    latest: bool,
    premium: bool,
    deliver_date: Option<String>,
    streaming_url: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OnsenPerformer {
    id: u32,
    name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OnsenProgram {
    // "category_list",
    // #[serde(borrow)]
    contents: Vec<OnsenProgramContents>,
    // #[serde(borrow)]
    performers: Vec<OnsenPerformer>, // "copyright",
    // "delivery_day_of_week",
    // "delivery_interval",
    // "directory_name",
    // "display",
    id: u32,
    // "image",
    // "list",
    // "new",
    // "performers",
    // "related_infos",
    // "related_links",
    // "related_programs",
    // "show_contents_count",
    // "sponsor_name",
    pub title: String,
    // "updated"
}
impl OnsenProgram {
    pub fn record(&self) -> Result<(), RecordError> { // Changed signature
        let archive_path = ensure_archive_path("onsen")?;

        let tmpdir = temp_dir().to_str().ok_or(RecordError::TempDir)?.to_string();
        info!("working path: {}", tmpdir);

        for contents in &self.contents {
            match &contents.streaming_url {
                Some(n) => {
                    let file_name = format!(
                        "{}_{}.mp4",
                        &self.title.as_str().replace([' ', '　', '/'], "_"),
                        &contents.title.as_str().replace([' ', '/'], "_")
                    );
                    let output_path = format!("{}/{}", tmpdir, &file_name);
                    let archive_file = format!("{}/{}", &archive_path, &file_name);
                    let path = Path::new(&archive_file);
                    if path.exists() {
                        warn!("{} already exists, skipping", &archive_file);
                        continue;
                    }
                    let output = Command::new("ffmpeg")
                        .arg("-loglevel")
                        .arg("warning")
                        .arg("-headers")
                        .arg("Origin: https://www.onsen.ag")
                        .arg("-headers")
                        .arg("Referer: https://www.onsen.ag/")
                        .arg("-y")
                        .arg("-i")
                        .arg(n)
                        .arg("-vcodec")
                        .arg("libx264")
                        .arg("-acodec")
                        .arg("copy")
                        .arg("-bsf:a")
                        .arg("aac_adtstoasc")
                        .arg(&output_path)
                        .output()?;

                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        error!("ffmpeg failed for {} - {}: {}", self.title, contents.title, stderr);
                        // Continue to next content on failure, or return Err?
                        // For now, mimicking original by continuing, but logging error.
                        // If this should halt, use:
                        // return Err(RecordError::CommandFailed {
                        // command: "ffmpeg".to_string(),
                        // exit_code: output.status.code(),
                        // stderr,
                        // });
                        continue;
                    }

                    let options = CopyOptions::new();
                    fs_extra::file::move_file(
                        &output_path,
                        format!("{}/{}", archive_path, &file_name),
                        &options,
                    ).map_err(|e| RecordError::Other(format!("Failed to move file for {} - {}: {}", self.title, contents.title, e)))?;
                }
                None => warn!(
                    "streaming url is null for {},{}",
                    self.title, contents.title
                ),
            };
        }
        Ok(())
    }
    pub fn init() -> Vec<OnsenProgram> {
        let client = reqwest::blocking::Client::new();
        match client.get("https://www.onsen.ag/web_api/programs").send() {
            Ok(m) => match m.json::<Vec<OnsenProgram>>() {
                Ok(n) => n,

                Err(e) => {
                    error!("{}", e);
                    panic!("{}", e);
                }
            },
            Err(e) => {
                error!("{}", e);
                panic!("{}", e);
            }
        }
    }
}
