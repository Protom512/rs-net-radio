// use core::panicking::panic;
use crate::utils::{ensure_archive_path, sanitize_filename, RecordError}; // Added RecordError
use log; // 0.4.14
use log::{debug, error, info, warn};
use reqwest; // 0.11.4
use reqwest::blocking::Response;
use reqwest::header::{ORIGIN, USER_AGENT};
use serde::Deserialize;
use serde_json;

use std::env::temp_dir;
// use std::fmt::format;

extern crate m3u8_rs;
extern crate tempdir;
use fs_extra;

use std::fs; // Removed
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

/// Represents the JSON structure for a Hibiki radio program.
#[derive(Deserialize, Debug)]
pub struct HibikiJson {
    /// The access ID of the program.
    access_id: String,
    //cast: String,
    /// The ID of the latest episode.
    latest_episode_id: Option<u32>,
    /// The name of the latest episode.
    latest_episode_name: Option<String>,
    /// The URL of the program's PC image.
    pc_image_url: Option<String>,
    /// The name of the program.
    name: String,
}

/// Fetches data from the Hibiki API.
///
/// # Arguments
///
/// * `url` - The API endpoint URL.
///
/// # Returns
///
/// A `reqwest::Result` containing the API response.
pub fn get_api(url: &str) -> Result<Response, RecordError> {
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

#[test]
fn test_generate_episode_filename() {
    assert_eq!(
        generate_episode_filename("Program A", Some("Episode 1")),
        "Program A_Episode 1.mp4"
    );
    assert_eq!(
        generate_episode_filename("Program B", None),
        "Program B_UnknownEpisode.mp4"
    );
    assert_eq!(
        generate_episode_filename("Program/C", Some("Episode:2*")),
        "ProgramC_Episode2.mp4"
    );
    assert_eq!(
        generate_episode_filename("Program\\D", Some("Episode?3")),
        "ProgramD_Episode3.mp4"
    );
    assert_eq!(
        generate_episode_filename("Program\"E", Some("Episode`4")),
        "ProgramE_Episode`4.mp4"
    );
    assert_eq!(
        generate_episode_filename("Program>F", Some("Episode<5")),
        "ProgramF_Episode5.mp4"
    );
}

/// Fetches data from a URL and parses it into a specified type.
///
/// # Arguments
///
/// * `url` - The URL to fetch data from.
///
/// # Type Parameters
///
/// * `T` - The type to deserialize the JSON response into. Must implement `serde::Deserialize`.
///
/// # Returns
///
/// A `Result` containing the parsed data of type `T` or an error message string.
fn fetch_and_parse<T: for<'de> Deserialize<'de>>(url: &str) -> Result<T, String> {
    let response = get_api(url).map_err(|e| {
        let err_msg = format!("Failed to get API response from {}: {}", url, e);
        error!("{}", err_msg);
        err_msg
    })?;

    let text = response.text().map_err(|e| {
        let err_msg = format!("Failed to read response text from {}: {}", url, e);
        error!("{}", err_msg);
        err_msg
    })?;

    serde_json::from_str::<T>(&text).map_err(|e| {
        let err_msg = format!("Failed to parse JSON from {}: {}", url, e);
        error!("Error: {}, JSON Text: {}", err_msg, text); // Log the problematic JSON
        err_msg
    })
}

/// Generates a sanitized filename for an episode.
///
/// # Arguments
///
/// * `program_name` - The name of the program.
/// * `episode_name_opt` - An `Option` containing the episode name. Defaults to "UnknownEpisode".
///
/// # Returns
///
/// A `String` representing the sanitized filename (e.g., "Program_Name_Episode_Name.mp4").
fn generate_episode_filename(program_name: &str, episode_name_opt: Option<&str>) -> String {
    let episode_name = episode_name_opt.unwrap_or("UnknownEpisode");
    let raw_filename = format!("{}_{}.mp4", program_name, episode_name);
    sanitize_filename(&raw_filename)
}

fn process_program(program: &HibikiJson, archive_base_path: &str) -> Result<(), String> {
    debug!("{:?}", program);
    let episode_url = format!(
        "https://vcms-api.hibiki-radio.jp/api/v1/programs/{}",
        program.access_id
    );
    let api_episode_detail: HibikiEpisode = fetch_and_parse(&episode_url)?;

    let episode = match &api_episode_detail.episode {
        Some(n) => n,
        None => {
            let err_msg = format!(
                "Not Downloadable. Failed to get Episode Id for program: {}",
                program.name
            );
            error!("{}", err_msg);
            return Err(err_msg);
        }
    };

    if program.latest_episode_id.unwrap_or(0) != episode.id {
        let err_msg = format!(
            "Not Downloadable. Outdated Episode, title={name} expected_id={expected_id} actual_id={actual_id}",
            name = program.latest_episode_name.as_deref().unwrap_or("UNKNOWN_NAME"),
            expected_id = program.latest_episode_id.unwrap_or(0),
            actual_id = episode.id
        );
        error!("{}", err_msg);
        return Err(err_msg);
    }

    let video = match &episode.video {
        Some(n) => n,
        None => {
            let err_msg = format!(
                "Not Downloadable. Failed to get video information for program: {}",
                program.name
            );
            error!("{}", err_msg);
            return Err(err_msg);
        }
    };

    if video.live_flg {
        let err_msg = format!("{} Not Downloadable. Program is live.", program.name);
        error!("{}", err_msg);
        return Err(err_msg);
    }

    // Construct archive path using the provided archive_base_path
    let archive_path = format!("{}/hibiki", archive_base_path);
    debug!("Archive path: {:#?}", &archive_path);
    if !Path::new(&archive_path).is_dir() {
        match fs::create_dir_all(&archive_path) {
            Ok(_) => debug!("Created directory: {}", archive_path),
            Err(e) => {
                let err_msg = format!("Failed to create archive directory {}: {}", archive_path, e);
                error!("{}", err_msg);
                // This might be a panic-worthy situation depending on requirements,
                // but for now, returning Err as per function's contract.
                return Err(err_msg);
            }
        };
    }

    if program.latest_episode_id.is_none() {
        let err_msg = format!(
            "Program {} has no latest_episode_id. Skipping.",
            program.name
        );
        warn!("{}", err_msg);
        return Err(err_msg); // Or Ok(()), depending on whether this is considered an error or just a skippable item.
    }

    let tmpdir = match temp_dir().to_str() {
        Some(m) => {
            info!("working path: {}", m);
            m.to_string()
        }
        None => {
            // This is a more critical system issue.
            // For a library function, returning Err is better than panic.
            let err_msg = "Cannot find tmpdir".to_string();
            error!("{}", err_msg);
            return Err(err_msg);
        }
    };

    let imagefile = format!("{}/{}_thumb.jpg", &tmpdir, sanitize_filename(&program.name));
    let mut img = match std::fs::File::create(&imagefile) {
        Ok(f) => f,
        Err(e) => {
            let err_msg = format!("Failed to create image file {}: {}", imagefile, e);
            error!("{}", err_msg);
            return Err(err_msg);
        }
    };

    match program.pc_image_url {
        Some(ref n) => match reqwest::blocking::get(n) {
            Ok(mut m) => {
                if let Err(e) = m.copy_to(&mut img) {
                    let err_msg = format!(
                        "Failed to download and save image for {}: {}",
                        program.name, e
                    );
                    error!("{}", err_msg);
                    return Err(err_msg);
                }
            }
            Err(e) => {
                debug!("{:#?}", program);
                let err_msg = format!("Failed to download image for {}: {}", program.name, e);
                error!("{}", err_msg);
                return Err(err_msg);
            }
        },
        None => {
            let err_msg = format!("Image not downloadable for program: {}.", program.name);
            error!("{}", err_msg);
            return Err(err_msg);
        }
    };

    // create file_name
    let filename = generate_episode_filename(&program.name, program.latest_episode_name.as_deref());
    let output_path = format!("{}/{}", archive_path, &filename);
    let working_path = format!("{}/{}", tmpdir, &filename);

    debug!("name:{}\n\tid:{:?}\n", program.name, video.live_flg);
    let url = match video.get_m3u8_url() {
        Ok(u) => u,
        Err(e) => {
            error!("Failed to get m3u8 url: {}", e);
            return Err(e.to_string());
        }
    };
    debug!("title: {},url\"{}\"", program.name, url);

    let path = Path::new(&output_path);
    if path.exists() {
        warn!("{} already exists, skipping", &output_path);
        return Ok(()); // Not an error, successfully skipped.
    }

    let ffmpeg_output = Command::new("ffmpeg")
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
        .output();

    match ffmpeg_output {
        Ok(output) => {
            if output.status.success() {
                let options = CopyOptions::new();
                match fs_extra::file::move_file(&working_path, &output_path, &options) {
                    Ok(_) => {
                        info!("Successfully archived {}", output_path);
                        Ok(())
                    }
                    Err(e) => {
                        let err_msg = format!(
                            "Failed to move file from {} to {}: {}",
                            working_path, output_path, e
                        );
                        error!("{}", err_msg);
                        Err(err_msg)
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let err_msg = format!("ffmpeg command failed for {}: {}", program.name, stderr);
                error!("{}", err_msg);
                Err(err_msg)
            }
        }
        Err(e) => {
            let err_msg = format!("Failed to execute ffmpeg for {}: {}", program.name, e);
            error!("{}", err_msg);
            Err(err_msg)
        }
    }
}

static RS_NET_ARCHIVE_PATH: &str = "RS_NET_ARCHIVE_PATH";

/// Records Hibiki radio programs.
///
/// This function fetches the list of programs, checks for new episodes,
/// and downloads them using ffmpeg.
pub fn record() {
    // Get the base archive path from environment variable. This is critical.
    let archive_base_path = ensure_archive_path("hibiki").expect("Failed to get archive base path");

    let page = 1; // Assuming page is fixed at 1 as per original logic
    let programs_url = format!(
        "https://vcms-api.hibiki-radio.jp/api/v1/programs?limit=50&page={}",
        page
    );

    info!("Fetching program list from {}", programs_url);
    let programs: Vec<HibikiJson> = match fetch_and_parse(&programs_url) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to fetch or parse program list: {}", e);
            panic!("Failed to fetch or parse program list: {}", e);
        }
    };

    info!(
        "Fetched {} programs. Starting processing...",
        programs.len()
    );
    for program in programs {
        info!("Processing program: {}", program.name);
        match process_program(&program, &archive_base_path) {
            Ok(()) => info!("Successfully processed program: {}", program.name),
            Err(e) => error!("Failed to process program {}: {}", program.name, e),
        }
    }
    info!("Finished processing all programs.");
}
