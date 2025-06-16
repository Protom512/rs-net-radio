// use core::panicking::panic;
use log; // 0.4.14
use log::{debug, error, info, warn};
use reqwest; // 0.11.4
use crate::utils::{ensure_archive_path, sanitize_filename, RecordError}; // Added RecordError
use reqwest::blocking::Response;
use reqwest::header::{ORIGIN, USER_AGENT};
use serde::Deserialize;
use serde_json;
// use std::env; // Removed
use std::env::temp_dir;
// use std::fmt::format;

extern crate m3u8_rs;
extern crate tempdir;
use fs_extra;

// use std::fs; // Removed
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
    pc_image_url: Option<String>,
    name: String,
}
pub fn get_api(url: &str) -> Result<Response, RecordError> { // Changed from reqwest::Result
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
        .map_err(RecordError::Reqwest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito;
    use http::StatusCode; // Required for StatusCode::OK

    #[test]
    fn pass_get_api_mocked() { // Renamed
        let server = mockito::mock("GET", "/") // mockito 0.31 syntax, removed mut
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status":"ok"}"#)
            .create();

        match get_api(&mockito::server_url()) {
            Ok(response) => {
                assert_eq!(response.status(), StatusCode::OK);
                assert_eq!(response.json::<serde_json::Value>().unwrap(), serde_json::json!({"status":"ok"}));
            }
            Err(e) => panic!("get_api_mocked failed: {}", e),
        }
        server.assert(); // Verify mock was called (mockito 0.31)
    }
}

impl HibikiVideo {
    fn get_m3u8_url(&self) -> Result<String, RecordError> {
        let url = format!(
            "https://vcms-api.hibiki-radio.jp/api/v1/videos/play_check?video_id={video_id}",
            video_id = self.id
        );
        debug!("{}", url);
        let playlist_response = get_api(&url)?;
        debug!("{:#?}", &playlist_response);
        let playlist_text = playlist_response.text().map_err(RecordError::Reqwest)?;
        let playlist_info: HibikiPlaylistInfo =
            serde_json::from_str(&playlist_text).map_err(RecordError::SerdeJson)?;

        debug!("{:#?}", &playlist_info);
        match playlist_info.token {
            Some(n) => Ok(format!("{}&token={}", playlist_info.playlist_url, n)),
            None => Ok(playlist_info.playlist_url),
        }
    }
}

pub fn record() -> Result<(), RecordError> {
    let page = 1;

    let get_result = get_api(&format!( // Use ?
        "https://vcms-api.hibiki-radio.jp/api/v1/programs?limit=50&page={page}"
    ))?;
    let program_list_text = get_result.text().map_err(RecordError::Reqwest)?;
    let programs: Vec<HibikiJson> = serde_json::from_str(&program_list_text).map_err(RecordError::SerdeJson)?;

    for i in programs { // Changed 'sea' to 'programs'
        debug!("{:?}", i);
        let episode_response = match get_api(&format!("https://vcms-api.hibiki-radio.jp/api/v1/programs/{}", i.access_id)) {
            Ok(n) => n,
            Err(e) => {
                error!("API error for program {} ({}): {}", i.name, i.access_id, e);
                continue;
            }
        };
        let episode_text = match episode_response.text() {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to get text for episode details of {}: {}", i.name, e);
                continue;
            }
        };
        let episode_data: HibikiEpisode = match serde_json::from_str(&episode_text) { // Changed 'sea' to 'episode_data'
            Ok(d) => d,
            Err(e) => {
                error!("Failed to parse episode details for {}: {}", i.name, e);
                continue;
            }
        };

        let episode = match &episode_data.episode { // Changed 'sea' to 'episode_data'
            Some(n) => n,
            None => {
                error!("Not Downloadable. Failed to get Episode Id for {}", i.name);
                continue;
            }
        };

        let latest_id = i.latest_episode_id.ok_or_else(|| RecordError::Other(format!("Missing latest_episode_id for {}", i.name)))?;
        let episode_name = i.latest_episode_name.clone().ok_or_else(|| RecordError::Other(format!("Missing latest_episode_name for {}", i.name)))?;

        if latest_id != episode.id {
            error!(
                "Not Downloadable. Outdated Episode, title={} expected_id={} actual_id={}",
                i.name, // Use i.name directly
                latest_id,
                episode.id
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
        let archive_path = ensure_archive_path("hibiki")?;
        if i.latest_episode_id.is_none() {
            continue;
        }

        let tmpdir = temp_dir().to_str().ok_or(RecordError::TempDir)?.to_string();
        info!("working path: {}", tmpdir);


        let imagefile = format!("{}/{}_thumb.jpg", &tmpdir, &i.name);
        let mut img = std::fs::File::create(&imagefile)?;
        match i.pc_image_url {
            Some(ref n) => {
                let mut m = reqwest::blocking::get(n)?;
                m.copy_to(&mut img)?;
            }
            None => {
                error!("Image not downloadable for {}.", i.name);
                // Optionally, return an error or skip to next iteration
                // For now, just logging and continuing as per original logic for missing images
                continue;
            }
        };

        // create file_name
        // Use episode_name captured above
        let filename = format!("{}_{}.mp4", i.name, episode_name);
        // format characters
        let filename = sanitize_filename(filename.as_str());
        let output_path = format!("{}/{}", archive_path, &filename);
        let working_path = format!("{}/{}", tmpdir, &filename);

        debug!("name:{}\n\tid:{:?}\n", i.name, video.live_flg);
        let url = match video.get_m3u8_url() {
            Ok(u) => u,
            Err(e) => {
                error!("Failed to get m3u8 url for {}: {}", i.name, e);
                continue;
            }
        };

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
            .arg(&imagefile)
            .arg("-i")
            .arg(&url)
            .arg("-vcodec")
            .arg("copy")
            .arg("-acodec")
            .arg("copy")
            .arg("-bsf:a")
            .arg("aac_adtstoasc")
            .arg(&working_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            error!("ffmpeg failed for {}: {}", i.name, stderr);
            return Err(RecordError::CommandFailed{
                command: "ffmpeg".to_string(),
                exit_code: output.status.code(),
                stderr,
            });
        }

        let options = CopyOptions::new();
        fs_extra::file::move_file(&working_path, &output_path, &options)
            .map_err(|e| RecordError::Other(format!("Failed to move file for {}: {}", i.name, e)))?;
    }
    Ok(())
}
