// use core::panicking::panic;
#![expect(
    clippy::missing_errors_doc,
    reason = "async functions have complex error cases documented at module level"
)]
#![expect(
    clippy::manual_let_else,
    reason = "manual let-else pattern improves code readability in this context"
)]

use crate::utils::sanitize_filename;
use crate::utils::{ensure_archive_path, RecordError}; // Added RecordError
use crate::{FfmpegCommand, FfmpegInput, FfmpegOutput};
use log; // 0.4.14
use log::{debug, error, info, warn};
use reqwest; // 0.11.4
use reqwest::blocking::Response;
use reqwest::header::USER_AGENT;
use serde::Deserialize;
use serde_json;
use std::env::temp_dir;

extern crate m3u8_rs;
extern crate tempdir;
use fs_extra;

use std::path::Path;

use fs_extra::file::CopyOptions;

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
    let client = crate::http::blocking_client();

    // Use modern User-Agent matching current Chrome browser
    let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                     (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36";

    let response = client
        .get(url)
        .header("Origin", "https://hibiki-radio.jp")
        .header("Referer", "https://hibiki-radio.jp/")
        .header(USER_AGENT, user_agent)
        .header("X-Requested-With", "XMLHttpRequest")
        .header("sec-ch-ua", r#""Not(A:Brand";v="8", "Chromium";v="144""#)
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", r#""Windows""#)
        .send()
        .map_err(RecordError::Reqwest)?;

    // Check for HTTP errors, especially 403 and 429
    crate::utils::check_http_status(&response, url)?;

    Ok(response)
}

impl HibikiVideo {
    fn get_m3u8_url(&self) -> Result<String, RecordError> {
        let url = format!(
            "https://vcms-api.hibiki-radio.jp/api/v1/videos/play_check?video_id={video_id}",
            video_id = self.id
        );
        debug!("{url}");
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
        let err_msg = format!("Failed to get API response from {url}: {e}");
        error!("{err_msg}");
        err_msg
    })?;

    let text = response.text().map_err(|e| {
        let err_msg = format!("Failed to read response text from {url}: {e}");
        error!("{err_msg}");
        err_msg
    })?;

    serde_json::from_str::<T>(&text).map_err(|e| {
        let err_msg = format!("Failed to parse JSON from {url}: {e}");
        error!("Error: {err_msg}, JSON Text: {text}"); // Log the problematic JSON
        err_msg
    })
}

/// Generates a sanitized filename for an episode.
///
/// # Arguments
///
/// * `program_name` - The name of the program.
/// * `episode_name_opt` - An `Option` containing the episode name. Defaults to "`UnknownEpisode`".
///
/// # Returns
///
/// A `String` representing the sanitized filename (e.g., "`Program_Name_Episode_Name.mp4`").
fn generate_episode_filename(program_name: &str, episode_name_opt: Option<&str>) -> String {
    let episode_name = episode_name_opt.unwrap_or("UnknownEpisode");
    let raw_filename = format!("{program_name}_{episode_name}.mp4");
    sanitize_filename(&raw_filename)
}

#[expect(
    clippy::too_many_lines,
    reason = "complex recording logic requires sequential processing; refactoring would reduce clarity"
)]
fn process_program(program: &HibikiJson, archive_base_path: &str) -> Result<(), String> {
    debug!("{program:?}");
    let episode_url = format!(
        "https://vcms-api.hibiki-radio.jp/api/v1/programs/{}",
        program.access_id
    );
    let api_episode_detail: HibikiEpisode = fetch_and_parse(&episode_url)?;

    let episode = if let Some(n) = &api_episode_detail.episode {
        n
    } else {
        let err_msg = format!(
            "Not Downloadable. Failed to get Episode Id for program: {}",
            program.name
        );
        error!("{err_msg}");
        return Err(err_msg);
    };

    if program.latest_episode_id.unwrap_or(0) != episode.id {
        let err_msg = format!(
            "Not Downloadable. Outdated Episode, title={name} expected_id={expected_id} actual_id={actual_id}",
            name = program.latest_episode_name.as_deref().unwrap_or("UNKNOWN_NAME"),
            expected_id = program.latest_episode_id.unwrap_or(0),
            actual_id = episode.id
        );
        error!("{err_msg}");
        return Err(err_msg);
    }

    let video = if let Some(n) = &episode.video {
        n
    } else {
        let err_msg = format!(
            "Not Downloadable. Failed to get video information for program: {}",
            program.name
        );
        error!("{err_msg}");
        return Err(err_msg);
    };

    if video.live_flg {
        let err_msg = format!("{} Not Downloadable. Program is live.", program.name);
        error!("{err_msg}");
        return Err(err_msg);
    }

    // archive_base_path already includes "/hibiki" from ensure_archive_path()
    debug!("Archive path: {:#?}", &archive_base_path);

    if program.latest_episode_id.is_none() {
        let err_msg = format!(
            "Program {} has no latest_episode_id. Skipping.",
            program.name
        );
        warn!("{err_msg}");
        return Err(err_msg); // Or Ok(()), depending on whether this is considered an error or just a skippable item.
    }

    let tmpdir = if let Some(m) = temp_dir().to_str() {
        info!("working path: {m}");
        m.to_string()
    } else {
        // This is a more critical system issue.
        // For a library function, returning Err is better than panic.
        let err_msg = "Cannot find tmpdir".to_string();
        error!("{err_msg}");
        return Err(err_msg);
    };

    let imagefile = format!("{}/{}_thumb.jpg", &tmpdir, sanitize_filename(&program.name));
    let mut img = match std::fs::File::create(&imagefile) {
        Ok(f) => f,
        Err(e) => {
            let err_msg = format!("Failed to create image file {imagefile}: {e}");
            error!("{err_msg}");
            return Err(err_msg);
        }
    };

    if let Some(ref n) = program.pc_image_url {
        let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                         (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36";
        let client = crate::http::blocking_client();
        match client.get(n).header("User-Agent", user_agent).send() {
            Ok(mut response) => {
                if !response.status().is_success() {
                    let err_msg = format!(
                        "Failed to download image for {}: HTTP {}",
                        program.name,
                        response.status()
                    );
                    error!("{err_msg}");
                    return Err(err_msg);
                }
                if let Err(e) = response.copy_to(&mut img) {
                    let err_msg = format!("Failed to save image for {}: {}", program.name, e);
                    error!("{err_msg}");
                    return Err(err_msg);
                }
            }
            Err(e) => {
                debug!("{program:#?}");
                let err_msg = format!("Failed to download image for {}: {}", program.name, e);
                error!("{err_msg}");
                return Err(err_msg);
            }
        }
    } else {
        let err_msg = format!("Image not downloadable for program: {}.", program.name);
        error!("{err_msg}");
        return Err(err_msg);
    }

    // create file_name
    let filename = generate_episode_filename(&program.name, program.latest_episode_name.as_deref());
    let output_path = format!("{}/{}", archive_base_path, &filename);
    let working_path = format!("{}/{}", tmpdir, &filename);

    debug!("name:{}\n\tid:{:?}\n", program.name, video.live_flg);
    let url = match video.get_m3u8_url() {
        Ok(u) => u,
        Err(e) => {
            error!("Failed to get m3u8 url: {e}");
            return Err(e.to_string());
        }
    };
    debug!("title: {},url\"{}\"", program.name, url);

    let path = Path::new(&output_path);
    if path.exists() {
        warn!("{} already exists, skipping", &output_path);
        return Ok(()); // Not an error, successfully skipped.
    }

    // Prepare HTTP headers for ffmpeg to authenticate requests
    let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                     (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36";

    let thumbnail_input = FfmpegInput::file(&imagefile);
    let stream_input = FfmpegInput::http(&url)
        .header(format!("User-Agent: {user_agent}\r\n"))
        .header("Referer: https://hibiki-radio.jp/\r\n")
        .header("Origin: https://hibiki-radio.jp\r\n")
        .user_agent(user_agent);

    FfmpegCommand::new()
        .log_level("warning")
        .inputs(vec![thumbnail_input, stream_input])
        .video_codec("copy")
        .audio_codec("copy")
        .bitstream_filter("aac_adtstoasc")
        .output(&working_path)
        .run()
        .and_then(FfmpegOutput::into_result)
        .map_err(|e| {
            let err_msg = format!("ffmpeg command failed for {}: {}", program.name, e);
            error!("{err_msg}");
            err_msg
        })?;

    let options = CopyOptions::new();
    fs_extra::file::move_file(&working_path, &output_path, &options).map_err(|e| {
        let err_msg = format!("Failed to move file from {working_path} to {output_path}: {e}");
        error!("{err_msg}");
        err_msg
    })?;

    info!("Successfully archived {output_path}");
    Ok(())
}

/// Records Hibiki radio programs.
///
/// This function fetches the list of programs, checks for new episodes,
/// and downloads them using ffmpeg.
pub fn record() -> Result<(), RecordError> {
    // Get the base archive path from environment variable. This is critical.
    let archive_base_path = ensure_archive_path("hibiki").map_err(|e| {
        error!("Failed to get archive base path: {e}");
        e
    })?;

    let page = 1; // Assuming page is fixed at 1 as per original logic
    let programs_url =
        format!("https://vcms-api.hibiki-radio.jp/api/v1/programs?limit=50&page={page}");

    info!("Fetching program list from {programs_url}");
    let programs: Vec<HibikiJson> = fetch_and_parse(&programs_url).map_err(|e| {
        let msg = format!("Failed to fetch or parse program list: {e}");
        error!("{msg}");
        RecordError::Other(msg)
    })?;

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
    Ok(())
}

/// 非同期で単一プログラムを処理
async fn process_program_async(
    client: &reqwest::Client,
    program: &HibikiJson,
    archive_base_path: &str,
) -> Result<(), String> {
    debug!("{program:?}");
    let episode_url = format!(
        "https://vcms-api.hibiki-radio.jp/api/v1/programs/{}",
        program.access_id
    );

    let api_response = fetch_episode_async(client, &episode_url).await?;
    let episode = validate_episode(&api_response, program)?;

    let video = validate_video(episode, program)?;
    let tmpdir = std::env::temp_dir()
        .to_str()
        .ok_or_else(|| "Cannot find tmpdir".to_string())?
        .to_string();
    info!("working path: {tmpdir}");

    download_thumbnail_async(client, program, &tmpdir).await?;

    let filename = generate_episode_filename(&program.name, program.latest_episode_name.as_deref());
    let output_path = format!("{archive_base_path}/{filename}");
    let working_path = format!("{tmpdir}/{filename}");

    let path = Path::new(&output_path);
    if path.exists() {
        warn!("{output_path} already exists, skipping");
        return Ok(());
    }

    let url = get_streaming_url_async(client, video).await?;
    debug!("title: {},url\"{}\"", program.name, url);

    record_program_async(
        &url,
        &tmpdir,
        &sanitize_filename(&program.name),
        &working_path,
    )
    .await?;
    move_to_final_path(&working_path, &output_path)?;

    info!("Successfully archived {output_path}");
    Ok(())
}

/// Fetches episode details asynchronously.
async fn fetch_episode_async(
    client: &reqwest::Client,
    episode_url: &str,
) -> Result<HibikiEpisode, String> {
    let api_response = client
        .get(episode_url)
        .header("Origin", "https://hibiki-radio.jp")
        .header("Referer", "https://hibiki-radio.jp/")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .header("X-Requested-With", "XMLHttpRequest")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch episode: {e}"))?;

    let episode_text = api_response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {e}"))?;
    serde_json::from_str(&episode_text).map_err(|e| format!("Failed to parse episode JSON: {e}"))
}

/// Validates episode information.
fn validate_episode<'a>(
    api_episode_detail: &'a HibikiEpisode,
    program: &HibikiJson,
) -> Result<&'a HibikiEpisodeId, String> {
    let episode = if let Some(n) = &api_episode_detail.episode {
        n
    } else {
        let err_msg = format!(
            "Not Downloadable. Failed to get Episode Id for program: {}",
            program.name
        );
        error!("{err_msg}");
        return Err(err_msg);
    };

    if program.latest_episode_id.unwrap_or(0) != episode.id {
        let err_msg = format!(
            "Not Downloadable. Outdated Episode, title={name} expected_id={expected_id} actual_id={actual_id}",
            name = program.latest_episode_name.as_deref().unwrap_or("UNKNOWN_NAME"),
            expected_id = program.latest_episode_id.unwrap_or(0),
            actual_id = episode.id
        );
        error!("{err_msg}");
        return Err(err_msg);
    }

    Ok(episode)
}

/// Validates video information.
fn validate_video<'a>(
    episode: &'a HibikiEpisodeId,
    program: &HibikiJson,
) -> Result<&'a HibikiVideo, String> {
    let video = if let Some(n) = &episode.video {
        n
    } else {
        let err_msg = format!(
            "Not Downloadable. Failed to get video information for program: {}",
            program.name
        );
        error!("{err_msg}");
        return Err(err_msg);
    };

    if video.live_flg {
        let err_msg = format!("{} Not Downloadable. Program is live.", program.name);
        error!("{err_msg}");
        return Err(err_msg);
    }

    Ok(video)
}

/// Downloads thumbnail image asynchronously.
async fn download_thumbnail_async(
    client: &reqwest::Client,
    program: &HibikiJson,
    tmpdir: &str,
) -> Result<(), String> {
    let imagefile = format!("{tmpdir}/{}_thumb.jpg", sanitize_filename(&program.name));
    let mut img = std::fs::File::create(&imagefile)
        .map_err(|e| format!("Failed to create image file {imagefile}: {e}"))?;

    if let Some(ref n) = program.pc_image_url {
        let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                         (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36";
        let mut response = client
            .get(n)
            .header("User-Agent", user_agent)
            .send()
            .await
            .map_err(|e| format!("Failed to download image for {}: {e}", program.name))?;

        if !response.status().is_success() {
            let err_msg = format!(
                "Failed to download image for {}: HTTP {}",
                program.name,
                response.status()
            );
            error!("{err_msg}");
            return Err(err_msg);
        }

        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            std::io::Write::write_all(&mut img, &chunk)
                .map_err(|e| format!("Failed to write image: {e}"))?;
        }
    } else {
        let err_msg = format!("Image not downloadable for program: {}.", program.name);
        error!("{err_msg}");
        return Err(err_msg);
    }

    Ok(())
}

/// Gets streaming URL asynchronously.
async fn get_streaming_url_async(
    client: &reqwest::Client,
    video: &HibikiVideo,
) -> Result<String, String> {
    let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                     (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36";

    let playlist_url = format!(
        "https://vcms-api.hibiki-radio.jp/api/v1/videos/play_check?video_id={video_id}",
        video_id = video.id
    );
    let playlist_response = client
        .get(&playlist_url)
        .header("Origin", "https://hibiki-radio.jp")
        .header("Referer", "https://hibiki-radio.jp/")
        .header("User-Agent", user_agent)
        .header("X-Requested-With", "XMLHttpRequest")
        .send()
        .await
        .map_err(|e| format!("Failed to get playlist: {e}"))?;

    let playlist_text = playlist_response
        .text()
        .await
        .map_err(|e| format!("Failed to read playlist: {e}"))?;
    let playlist_info: HibikiPlaylistInfo = serde_json::from_str(&playlist_text)
        .map_err(|e| format!("Failed to parse playlist JSON: {e}"))?;

    Ok(match playlist_info.token {
        Some(n) => format!("{}&token={}", playlist_info.playlist_url, n),
        None => playlist_info.playlist_url,
    })
}

/// Records program asynchronously using ffmpeg.
async fn record_program_async(
    url: &str,
    tmpdir: &str,
    program_name_sanitized: &str,
    working_path: &str,
) -> Result<(), String> {
    let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                     (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36";

    let imagefile = format!("{tmpdir}/{program_name_sanitized}_thumb.jpg");

    let thumbnail_input = crate::FfmpegInput::file(&imagefile);
    let stream_input = crate::FfmpegInput::http(url)
        .header(format!("User-Agent: {user_agent}\r\n"))
        .header("Referer: https://hibiki-radio.jp/\r\n")
        .header("Origin: https://hibiki-radio.jp\r\n")
        .user_agent(user_agent);

    crate::FfmpegCommand::new()
        .log_level("warning")
        .inputs(vec![thumbnail_input, stream_input])
        .video_codec("copy")
        .audio_codec("copy")
        .bitstream_filter("aac_adtstoasc")
        .output(working_path)
        .run_async()
        .await
        .and_then(FfmpegOutput::into_result)
        .map_err(|e| {
            let err_msg = format!("ffmpeg command failed: {e}");
            error!("{err_msg}");
            err_msg
        })?;

    Ok(())
}

/// Moves recording to final path.
fn move_to_final_path(working_path: &str, output_path: &str) -> Result<(), String> {
    let options = fs_extra::file::CopyOptions::new();
    fs_extra::file::move_file(working_path, output_path, &options).map_err(|e| {
        let err_msg = format!("Failed to move file from {working_path} to {output_path}: {e}");
        error!("{err_msg}");
        err_msg
    })?;
    Ok(())
}

/// Hibiki radio programsを並列で録画する非同期関数
///
/// # 使用例
/// ```no_run
/// use record_lib::record::hibiki;
///
/// #[tokio::main]
/// async fn main() {
///     if let Err(e) = hibiki::record_parallel().await {
///         eprintln!("hibiki recording failed: {e}");
///     }
/// }
/// ```
///
/// # Errors
///
/// Returns `RecordError` if the archive path cannot be obtained or the program
/// list cannot be fetched/parsed. Per-program failures are logged and skipped.
///
/// # Panics
///
/// Panics if semaphore permit acquisition fails (unwraps internally).
pub async fn record_parallel() -> Result<(), RecordError> {
    let archive_base_path = ensure_archive_path("hibiki").map_err(|e| {
        error!("Failed to get archive base path: {e}");
        e
    })?;

    let page = 1;
    let programs_url =
        format!("https://vcms-api.hibiki-radio.jp/api/v1/programs?limit=50&page={page}");

    info!("Fetching program list from {programs_url}");

    // Use the shared client to benefit from connection pooling.
    let client = crate::http::async_client();

    // 非同期でプログラムリストを取得
    let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                     (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36";
    let response = client
        .get(&programs_url)
        .header("Origin", "https://hibiki-radio.jp")
        .header("Referer", "https://hibiki-radio.jp/")
        .header("User-Agent", user_agent)
        .header("X-Requested-With", "XMLHttpRequest")
        .send()
        .await;

    let programs: Vec<HibikiJson> = match response {
        Ok(resp) => {
            if !resp.status().is_success() {
                let msg = format!("Failed to fetch program list: HTTP {}", resp.status());
                error!("{msg}");
                return Err(RecordError::Other(msg));
            }
            let text = match resp.text().await {
                Ok(t) => t,
                Err(e) => {
                    let msg = format!("Failed to read response: {e}");
                    error!("{msg}");
                    return Err(RecordError::Other(msg));
                }
            };
            match serde_json::from_str::<Vec<HibikiJson>>(&text) {
                Ok(p) => p,
                Err(e) => {
                    let msg = format!("Failed to parse program list JSON: {e}");
                    error!("{msg}");
                    return Err(RecordError::Other(msg));
                }
            }
        }
        Err(e) => {
            let msg = format!("Failed to fetch program list: {e}");
            error!("{msg}");
            return Err(RecordError::Other(msg));
        }
    };

    info!(
        "Fetched {} programs. Starting parallel processing...",
        programs.len()
    );

    // 並列処理の最大数を制限（同時接続数を抑えるため）
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(3));
    let mut tasks = Vec::new();

    for program in programs {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let archive_path = archive_base_path.clone();
        let client = client.clone();

        let task = tokio::spawn(async move {
            let _permit = permit; // タスク完了時に permit を解放
            info!("Processing program: {}", program.name);
            match process_program_async(&client, &program, &archive_path).await {
                Ok(()) => {
                    info!("Successfully processed program: {}", program.name);
                }
                Err(e) => error!("Failed to process program {}: {}", program.name, e),
            }
        });

        tasks.push(task);
    }

    // すべてのタスクの完了を待つ
    for task in tasks {
        let _ = task.await;
    }

    info!("Finished processing all programs.");
    Ok(())
}
