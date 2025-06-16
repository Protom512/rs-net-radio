use fs_extra;
use fs_extra::file::CopyOptions;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::env;
use std::env::temp_dir;
use std::fs;
use std::path::Path;
use std::process::Command;

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
    const RS_NET_ARCHIVE_PATH: &'static str = "RS_NET_ARCHIVE_PATH";
    /// Records the episodes of the Onsen program.
    ///
    /// This function iterates through the program's contents (episodes) and downloads
    /// any available streams using ffmpeg.
    pub fn record(&self) {
        let archive_path = match env::var(Self::RS_NET_ARCHIVE_PATH) {
            Ok(n) => {
                let path = format!("{n}/onsen");
                debug!("{:#?}", &path);
                if !Path::new(&path).is_dir() {
                    match fs::create_dir_all(format!("{n}/onsen")) {
                        Ok(m) => debug!("{:?}", m),
                        Err(e) => {
                            error!("{}", e);
                            panic!("{}", e);
                        }
                    };
                }
                path
            }
            Err(e) => panic!("$RS_NET_ARCHIVE_PATH  is not set: {}", e),
        };
        let tmpdir = match temp_dir().to_str() {
            Some(m) => {
                info!("working path: {}", m);
                m.to_string()
            }
            None => {
                panic!("cannot find tmpdir")
            }
        };

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
                        .output()
                        .expect("failed to execute");
                    if !output.status.success() {
                        error!("result:{:?}", output);
                    }
                    //TODO change /tmp/ to archive path
                    //
                    let options = CopyOptions::new();
                    match fs_extra::file::move_file(
                        &output_path,
                        format!("{}/{}", archive_path, &file_name),
                        &options,
                    ) {
                        Ok(n) => n,
                        Err(e) => {
                            error!("{:?}", e);
                            0
                        }
                    };
                }
                None => warn!(
                    "streaming url is null for {},{}",
                    self.title, contents.title
                ),
            };
        }
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
