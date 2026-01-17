use crate::utils::{ensure_archive_path, RecordError}; // Added RecordError
use crate::{FfmpegCommand, FfmpegInput};
use fs_extra;
use fs_extra::file::CopyOptions;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::env::temp_dir;
use std::path::Path;

/// Represents the contents of an Onsen program episode.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OnsenProgramContents {
    /// The ID of the program content.
    id: u32,
    /// The title of the program content (episode).
    title: String,
    /// Indicates if this is the latest episode.
    latest: bool,
    /// Indicates if this is a premium episode.
    premium: bool,
    /// The delivery date of the episode.
    deliver_date: Option<String>,
    /// The streaming URL for the episode.
    streaming_url: Option<String>,
}

/// Represents a performer on an Onsen program.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OnsenPerformer {
    /// The ID of the performer.
    id: u32,
    /// The name of the performer.
    name: String,
}

/// Represents an Onsen radio program.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OnsenProgram {
    // "category_list",
    // #[serde(borrow)]
    /// A list of contents (episodes) for the program.
    contents: Vec<OnsenProgramContents>,
    // #[serde(borrow)]
    /// A list of performers for the program.
    performers: Vec<OnsenPerformer>, // "copyright",
    // "delivery_day_of_week",
    // "delivery_interval",
    // "directory_name",
    // "display",
    /// The ID of the program.
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
    /// The title of the program.
    pub title: String,
    // "updated"
}
impl OnsenProgram {
    /// Records the episodes of the Onsen program.
    ///
    /// This function iterates through the program's contents (episodes) and downloads
    /// any available streams using ffmpeg.
    ///
    /// # Errors
    ///
    /// Returns `RecordError` if:
    /// - The archive path cannot be created
    /// - The temporary directory cannot be found
    /// - Stream download or recording fails
    pub fn record(&self) -> Result<(), RecordError> {
        // Changed signature
        let archive_path = ensure_archive_path("onsen")?;

        let tmpdir = temp_dir().to_str().ok_or(RecordError::TempDir)?.to_string();
        info!("working path: {tmpdir}");

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

                    let stream_input = FfmpegInput::http(n)
                        .header("Origin: https://www.onsen.ag\r\n")
                        .header("Referer: https://www.onsen.ag/\r\n");

                    let result = FfmpegCommand::new()
                        .log_level("warning")
                        .input(stream_input)
                        .video_codec("libx264")
                        .audio_codec("copy")
                        .bitstream_filter("aac_adtstoasc")
                        .overwrite(true)
                        .output(&output_path)
                        .run();

                    match result {
                        Ok(output) if output.is_success() => {
                            let options = CopyOptions::new();
                            fs_extra::file::move_file(
                                &output_path,
                                format!("{}/{}", archive_path, &file_name),
                                &options,
                            )
                            .map_err(|e| {
                                RecordError::Other(format!(
                                    "Failed to move file for {} - {}: {}",
                                    self.title, contents.title, e
                                ))
                            })?;
                        }
                        Ok(output) => {
                            error!(
                                "ffmpeg failed for {} - {}: {}",
                                self.title,
                                contents.title,
                                output.stderr()
                            );
                        }
                        Err(e) => {
                            error!(
                                "ffmpeg failed for {} - {}: {}",
                                self.title, contents.title, e
                            );
                        }
                    }
                }
                None => warn!(
                    "streaming url is null for {},{}",
                    self.title, contents.title
                ),
            }
        }
        Ok(())
    }

    /// Initializes a list of `OnsenProgram` tasks by fetching data from the Onsen API.
    ///
    /// # Returns
    ///
    /// A vector of `OnsenProgram` instances.
    ///
    /// # Panics
    ///
    /// Panics if the API request or JSON parsing fails.
    #[must_use]
    pub fn init() -> Vec<OnsenProgram> {
        let client = reqwest::blocking::Client::new();
        match client.get("https://www.onsen.ag/web_api/programs").send() {
            Ok(m) => match m.json::<Vec<OnsenProgram>>() {
                Ok(n) => n,

                Err(e) => {
                    error!("{e}");
                    panic!("{}", e);
                }
            },
            Err(e) => {
                error!("{e}");
                panic!("{}", e);
            }
        }
    }
}
