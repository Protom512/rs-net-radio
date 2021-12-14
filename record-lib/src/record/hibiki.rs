// use core::panicking::panic;
use log; // 0.4.14
use log::{debug, error, info, warn};
use reqwest; // 0.11.4
use reqwest::blocking::Response;
use reqwest::header::{ORIGIN, USER_AGENT};
use serde::Deserialize;
use serde_json;
use std::env;
use std::env::temp_dir;
// use std::fmt::format;

extern crate m3u8_rs;
extern crate tempdir;
use fs_extra;

use std::fs;
use std::path::Path;

use fs_extra::file::CopyOptions;
// use nom::InputIter;
use std::process::Command;

#[derive(Deserialize, Debug)]
struct HibikiPlaylistInfo {
    playlist_url: String,
    token: Option<String>,
}

#[derive(Deserialize, Debug)]
struct HibikiVideo {
    id: u32,
    live_flg: bool,
}

#[derive(Deserialize, Debug)]
struct HibikiEpisodeId {
    id: u32,
    video: Option<HibikiVideo>,
}
#[derive(Deserialize, Debug)]
struct HibikiEpisode {
    episode: Option<HibikiEpisodeId>,
}

#[derive(Deserialize, Debug)]
pub struct HibikiJson {
    access_id: String,
    //cast: String,
    latest_episode_id: Option<u32>,
    latest_episode_name: Option<String>,
    name: String,
}
pub fn get_api(url: &str) -> reqwest::Result<Response> {
    let client = reqwest::blocking::Client::new();

    client
        .get(url)
        .header(ORIGIN, "https://hibiki-radio.jp")
        .header(
            USER_AGENT,
            "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; Trident/5.0",
        )
        .header("X-Requested-With", "XMLHttpRequest")
        .send()
}
#[test]
fn pass_get_api() {
    let result = get_api("https://vcms-api.hibiki-radio.jp/api/v1//programs?limit=1")
        .expect("Failed to request on test");
    assert_eq!(result.status(), http::StatusCode::OK)
}

impl HibikiVideo {
    fn get_m3u8_url(&self) -> String {
        let url = format!(
            "https://vcms-api.hibiki-radio.jp/api/v1/videos/play_check?video_id={video_id}",
            video_id = self.id
        );
        debug!("{}", url);
        let playlist = get_api(&url).unwrap();
        // let playlist = match res {
        //     Ok(n) => n,
        //     Err(e) => error!("{}", e),
        // };
        debug!("{:?}", playlist);
        println!("{:#?}", &playlist);
        let playlist_info =
            match serde_json::from_str::<HibikiPlaylistInfo>(&playlist.text().unwrap()) {
                Ok(n) => n,
                Err(e) => panic!("{}", e),
            };
        println!("{:#?}", &playlist_info);
        match playlist_info.token {
            Some(n) => {
                debug!(
                    "{:?}",
                    format!("{}&token={}", playlist_info.playlist_url, n)
                );
                format!("{}&token={}", playlist_info.playlist_url, n)
            }
            None => playlist_info.playlist_url,
        }
    }
}

pub fn format_forbidden_char(filename: &str) -> String {
    // 禁止文字(半角記号)
    // let cannot_used_file_name = "\\/:*?`\"><|";
    // 禁止文字(全角記号)
    // let used_file_name = "￥／：＊？`”＞＜｜";
    //TODO motto smart ni yaritai
    filename
        .replace("\\", "￥")
        .replace("/", "／")
        .replace("\"", "”")
        .replace(":", "：")
        .replace("*", "＊")
        .replace("?", "？")
        .replace("`", "`")
        .replace(">", "＞")
        .replace("<", "＜")
}
#[test]
fn pass_format_char() {
    assert_eq!(format_forbidden_char("Fate/Test"), "Fate／Test")
}
pub fn record() {
    let page = 1;

    let res = get_api(&format!(
        "https://vcms-api.hibiki-radio.jp/api/v1//programs?limit=50&page={}",
        page
    ));
    let get_result = match res {
        Ok(n) => n,
        Err(e) => {
            error!("{}", e);
            panic!("{}", e);
        }
    };

    let sea = match serde_json::from_str::<Vec<HibikiJson>>(&get_result.text().unwrap()) {
        Ok(n) => n,
        Err(e) => {
            error!("{}", e);
            panic!("{}", e);
        }
    };
    for i in sea {
        println!("{:?}", i);
        let result = get_api(&format!(
            "https://vcms-api.hibiki-radio.jp/api/v1/programs/{}",
            i.access_id
        ))
        .unwrap();
        let sea = match serde_json::from_str::<HibikiEpisode>(&result.text().unwrap()) {
            Ok(n) => n,
            Err(e) => {
                error!("{}", e);
                panic!("{}", e);
            }
        };

        let episode = match &sea.episode {
            Some(n) => n,
            None => {
                error!("Not Downloadable. Failed to get Episode Id");
                continue;
            }
        };
        if i.latest_episode_id.unwrap() != episode.id {
            error!(
                "Not Downloadable. Outdated Episode, title={name} expected_id={expected_id} actual_id={actual_id}",
                name = i.latest_episode_name.expect("Failed to get name"),
                expected_id=i.latest_episode_id.unwrap(),
                actual_id=episode.id
            );
            continue;
        }
        let video = match &episode.video {
            Some(n) => n,
            None => {
                error!("Not Downloadable. Failed to get live_flg");
                continue;
            }
        };
        if video.live_flg {
            error!("{} Not Downloadable. Failed to get live_flg", i.name);
            continue;
        }

        // get archive path
        let archive_path = match env::var("RS_NET_ARCHIVE_PATH") {
            Ok(n) => {
                let path = format!("{}/hibiki", n);
                debug!("{:#?}", &path);
                if !Path::new(&path).is_dir() {
                    match fs::create_dir(format!("{}/hibiki", n)) {
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
        if i.latest_episode_id.is_none() {
            continue;
        }
        let tmpdir = match temp_dir().to_str() {
            Some(m) => {
                info!("working path: {}", m);
                m.to_string()
            }
            None => {
                panic!("cannot find tmpdir")
            }
        };

        // create file_name
        let filename = format!("{}_{}.mp4", i.name, i.latest_episode_name.unwrap());
        // format characters
        let filename = format_forbidden_char(filename.as_str());
        let output_path = format!("{}/{}", archive_path, &filename);
        let working_path = format!("{}/{}", tmpdir, &filename);

        info!("name:{}\n\tid:{:?}\n", i.name, video.live_flg);
        let url = video.get_m3u8_url();

        debug!("title: {},url\"{}\"", i.name, url);

        let path = Path::new(&output_path);
        if path.exists() {
            warn!("{} already exists, skipping", &output_path);
            continue;
        }

        let output = Command::new("ffmpeg")
            .arg("-loglevel")
            .arg("warning")
            .arg("-i")
            .arg(&url)
            .arg("-vn")
            .arg("-acodec")
            .arg("copy")
            .arg("-bsf:a")
            .arg("aac_adtstoasc")
            .arg(&working_path)
            .output()
            .expect(" failed to execute ffmpeg");

        let converted: String = String::from_utf8(output.stderr).unwrap();

        if !output.status.success() {
            error!("{}", converted);
        } else {
            let options = CopyOptions::new();
            fs_extra::file::move_file(&working_path, &output_path, &options)
                .expect("Failed to archive file");
        }
    }
}
