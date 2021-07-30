use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::env;
use std::env::temp_dir;
use std::fs;
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
    pub fn record(&self) {
        let archive_path = match env::var("RS_NET_ARCHIVE_PATH") {
            Ok(n) => {
                let path = format!("{}/onsen", n);
                if !Path::new(&path).is_dir() {
                    match fs::create_dir(format!("{}/onsen", n)) {
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
                info!("save path: {}", m);
                m.to_string()
            }
            None => {
                panic!("cannot find tmpdir")
            }
        };

        for contents in &self.contents {
            let _res = match &contents.streaming_url {
                Some(n) => {
                    let file_name = format!(
                        "{}_{}.mp4",
                        &self.title.as_str().replace(" ", "_").replace("　", "_"),
                        &contents.title.as_str().replace(" ", "_")
                    );
                    let output_path = format!("{}/{}", tmpdir, &file_name);
                    debug!("{}", file_name);
                    debug!("{:?}", n);
                    let output = Command::new("ffmpeg")
                        .arg("-loglevel")
                        .arg("warning")
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
                    match std::fs::rename(&output_path, format!("{}/{}", archive_path, &file_name))
                    {
                        Ok(n) => n,
                        Err(e) => error!("{:?}", e),
                    };
                }
                None => error!(
                    "streaming url is null for {},{}",
                    self.title, contents.title
                ),
            };
        }
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
