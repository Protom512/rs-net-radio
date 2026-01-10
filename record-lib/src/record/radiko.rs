use crate::utils::{ensure_archive_path, sanitize_filename, RecordError}; // Added RecordError
                                                                         // Assuming this is a custom module for base64 encoding
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone};
use log::{debug, error, info};
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};

use fs_extra::file::CopyOptions;
use serde_xml_rs::from_str;
use std::borrow::Cow;
use std::env::temp_dir;
use std::process::{Command, ExitStatus};

/// Represents the overall Radiko data structure.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Radiko<'a> {
    ttl: u32,
    srvtime: u32,
    #[serde(borrow)]
    /// Holds information about the stations.
    pub stations: Stations<'a>,
}

/// Represents a collection of stations.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Stations<'a> {
    #[serde(borrow)]
    /// Holds information about a single station.
    pub station: Station<'a>,
}

/// Represents a single station.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Station<'a> {
    id: ID,
    #[serde(default)]
    name: Cow<'a, str>,
    #[serde(borrow)]
    /// Holds the program schedule for the station.
    pub scd: Scd<'a>,
}
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum ID {
    QRR,
    LFR,
    BAYFM78,
}

/// Represents the program schedule for a station.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Scd<'a> {
    #[serde(borrow)]
    /// Holds a list of programs.
    pub progs: Vec<Progs<'a>>,
}

/// Represents a collection of programs.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Progs<'a> {
    #[serde(borrow)]
    #[serde(rename = "$value")]
    /// Holds a list of program sets, which can be either a date or a program.
    pub list: Vec<Progset<'a>>,
}

/// Represents either a date or a program in the schedule.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Progset<'a> {
    /// Represents a date in the schedule.
    Date(ProgDate),
    /// Represents a program in the schedule.
    #[serde(borrow)]
    Prog(Program<'a>),
}

/// Represents a date in the program schedule.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ProgDate {
    #[serde(rename = "$value")]
    value: u32,
}

/// Represents a single program.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Program<'a> {
    /// Start time of the program (YYYYMMDDHHMMSS).
    pub ft: Cow<'a, str>,
    /// End time of the program (YYYYMMDDHHMMSS).
    pub to: Cow<'a, str>,
    /// Start time of the program (HHMM).
    pub ftl: Cow<'a, str>,
    /// End time of the program (HHMM).
    pub tol: Cow<'a, str>,
    /// Duration of the program in seconds.
    pub dur: u32,
    /// Title of the program.
    pub title: Cow<'a, str>,
    // pfm: Option<&'a str>,
}

/// Represents the streaming URL for a channel.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ChStreamingUrl {
    // #[serde(borrow)]
    #[serde(rename = "$value")]
    list: Vec<Urlset>,
}

/// Represents a set of URLs, typically for streaming.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Urlset {
    // #[serde(borrow)]
    /// Represents a streaming URL.
    Url(StreamingUrl),
}

/// Represents a streaming URL with area restriction information.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct StreamingUrl {
    /// Indicates if the stream is area-free (0 for false, 1 for true).
    pub areafree: u8,
    /// The URL for creating the playlist.
    pub playlist_create_url: String,
    // media_url_path: String,
    // playlist_url_path: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RecordRadiko {
    pub title: String, // Made public for access in main.rs closure
    pub ft: DateTime<Local>,
    dur: u32,
    url: String,
}
impl RecordRadiko {
    /// Builds a list of recording tasks for the specified channel.
    ///
    /// Iterates the channel's program schedule and constructs `RecordRadiko` entries for each valid program that has a successfully parsed start time and an associated streaming URL. Programs that fail time parsing or are considered invalid are skipped.
    ///
    /// # Arguments
    ///
    /// * `ch` - Channel identifier (for example, `"QRR"` or `"LFR"`).
    ///
    /// # Returns
    ///
    /// A `Vec<RecordRadiko>` with one entry per valid program, each containing the program title, start time, duration, and stream URL.
    ///
    /// # Examples
    ///
    /// ```
    /// let tasks = RecordRadiko::init("QRR");
    /// assert!(tasks.iter().all(|t| !t.title.is_empty()));
    /// ```
    pub fn init(ch: &str) -> Vec<Self> {
        let radiko = Radiko::init(ch);
        let streaming_url = ChStreamingUrl::init(ch);
        let url = streaming_url.get_streaming_url();
        debug!("{}", &url);

        // let current_time = Local::now();
        let mut hoge = Vec::<RecordRadiko>::new();
        for i in radiko.stations.station.scd.progs {
            for j in i.list {
                match j {
                    Progset::Prog(n) => {
                        if n.validate_program() {
                            let ft = match n.parse_time() {
                                Ok(m) => m,
                                Err(e) => {
                                    error!("{}", e);
                                    continue;
                                }
                            };
                            let rad = RecordRadiko {
                                title: n.title.as_ref().to_string(),
                                ft,
                                dur: n.dur,
                                url: url.clone(),
                            };
                            hoge.push(rad);
                        }
                    }
                    Progset::Date(date) => debug!("{:#?}", date),
                }
                // info!("{:#?}", j);
            }
        }
        debug!("{:#?}", hoge);
        hoge
    }

    /// Downloads the program stream to the archive directory using Radiko authentication and ffmpeg.
    ///
    /// Performs Radiko auth steps, derives the partial key from the service key, requests a streaming
    /// URL, invokes `ffmpeg` to record for the program duration into a temporary file, then moves the
    /// file into the archive directory with a sanitized filename.
    ///
    /// # Errors
    ///
    /// Returns `RecordError` when authentication headers are missing or invalid, when key offset/length
    /// are out of bounds, when temporary directory or archive path resolution fails, when `ffmpeg` fails
    /// to produce a successful exit status, or when moving the recorded file into the archive fails.
    ///
    /// # Returns
    ///
    /// The `ExitStatus` produced by the `ffmpeg` process on success.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use chrono::Local;
    /// let rec = RecordRadiko {
    ///     title: "Example Show".to_string(),
    ///     ft: Local::now(),
    ///     dur: 60,
    ///     url: "https://example.com/stream.m3u8".to_string(),
    /// };
    /// let status = rec.download().expect("download failed");
    /// assert!(status.success());
    /// ```
    pub fn download(&self) -> Result<ExitStatus, RecordError> {
        // Changed signature
        let resp = RecordRadiko::auth1()?; // Propagate error from auth1

        let header_str = resp.headers();

        let radiko_authkey_value = String::from("bcd151073c03b352e1ef2fd66c32209da9ca0afa");

        let authtoken = header_str
            .get("x-radiko-authtoken")
            .ok_or_else(|| RecordError::Other("Missing X-Radiko-Authtoken header".to_string()))?
            .to_str()
            .map_err(|e| RecordError::Other(format!("Invalid X-Radiko-Authtoken header: {}", e)))?;

        let key_length_str = header_str
            .get("x-radiko-keylength")
            .ok_or_else(|| RecordError::Other("Missing X-Radiko-Keylength header".to_string()))?
            .to_str()
            .map_err(|e| RecordError::Other(format!("Invalid X-Radiko-Keylength header: {}", e)))?;
        let key_length: usize = key_length_str.parse().map_err(|e| {
            RecordError::Other(format!("Failed to parse X-Radiko-Keylength: {}", e))
        })?;

        let keyoffset_str = header_str
            .get("x-radiko-keyoffset")
            .ok_or_else(|| RecordError::Other("Missing X-Radiko-Keyoffset header".to_string()))?
            .to_str()
            .map_err(|e| RecordError::Other(format!("Invalid X-Radiko-Keyoffset header: {}", e)))?;
        let keyoffset: usize = keyoffset_str.parse().map_err(|e| {
            RecordError::Other(format!("Failed to parse X-Radiko-Keyoffset: {}", e))
        })?;

        if keyoffset + key_length > radiko_authkey_value.len() {
            return Err(RecordError::Other(format!(
                "Invalid slice: offset {} + length {} exceeds {}",
                keyoffset,
                key_length,
                radiko_authkey_value.len()
            )));
        }

        let partial_key = general_purpose::STANDARD
            .encode(&radiko_authkey_value[keyoffset..keyoffset + key_length]);

        let _resp_auth2 = RecordRadiko::auth2(authtoken, partial_key)?;
        debug!("{:#?}\n", &_resp_auth2.text()?); // Propagate error from text()

        // get archive path
        let archive_path = ensure_archive_path("radiko")?;

        let tmpdir = temp_dir().to_str().ok_or(RecordError::TempDir)?.to_string();
        info!("working path: {}", tmpdir);

        // create file_name
        let filename = format!("{}_{}.mp4", self.ft.format("%Y%m%d%H%M%S"), self.title);
        // format characters
        let filename = sanitize_filename(filename.as_str());
        let output_path = format!("{}/{}", archive_path, &filename);
        let working_path = format!("{}/{}", tmpdir, &filename);

        let output = Command::new("ffmpeg")
            .arg("-loglevel")
            .arg("debug")
            .arg("-fflags")
            .arg("+discardcorrupt")
            .arg("-headers")
            .arg(format!("X-Radiko-Authtoken: {authtoken}"))
            .arg("-y")
            .arg("-i")
            .arg(&self.url)
            .arg("-acodec")
            .arg("copy")
            .arg("-vn")
            .arg("-bsf:a")
            .arg("aac_adtstoasc")
            .arg("-t")
            .arg(self.dur.to_string())
            .arg(&working_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            error!("ffmpeg failed for {}: {}", self.title, stderr);
            return Err(RecordError::CommandFailed {
                command: "ffmpeg".to_string(),
                exit_code: output.status.code(),
                stderr,
            });
        }

        let options = CopyOptions::new();
        fs_extra::file::move_file(&working_path, output_path, &options).map_err(|e| {
            RecordError::Other(format!("Failed to move file for {}: {}", self.title, e))
        })?;

        Ok(output.status)
    }

    fn auth1() -> Result<Response, RecordError> {
        // Changed signature
        let client = Client::new();
        let url = "https://radiko.jp/v2/api/auth1";
        Ok(client
            .get(url)
            .header("pragma", "no-cache")
            .header("X-Radiko-App", "pc_html5")
            .header("X-Radiko-App-Version", "0.0.1")
            .header("X-Radiko-User", "test-stream")
            .header("X-Radiko-Device", "pc")
            .send()?) // Propagate error
    }
    fn auth2(token: &str, partial_key: String) -> Result<Response, RecordError> {
        // Changed signature
        let client = Client::new();
        let url = "https://radiko.jp/v2/api/auth2";
        Ok(client
            .get(url)
            .header("pragma", "no-cache")
            .header("X-Radiko-User", "test-stream")
            .header("X-Radiko-Device", "pc")
            .header("X-Radiko-AuthToken", token)
            .header("X-Radiko-PartialKey", partial_key)
            .send()?) // Propagate error
    }
}

impl Radiko<'_> {
    /// Initializes Radiko data for a given channel.
    ///
    /// # Arguments
    ///
    /// * `ch` - The channel ID (e.g., "QRR", "LFR").
    ///
    /// # Returns
    ///
    /// A `Radiko` struct containing station and program information.
    pub fn init(ch: &str) -> Self {
        let m = get_program_dom(ch);
        let radiko: Radiko = match from_str(match &m.text() {
            Ok(l) => l,
            Err(e) => {
                panic!("{:#?}", e);
            }
        }) {
            Ok(n) => n,
            Err(e) => {
                error!("{:#?}", e);
                panic!("{:#?}", e)
            }
        };
        radiko
    }
}

impl ChStreamingUrl {
    /// Gets the streaming URL from the `ChStreamingUrl` struct.
    ///
    /// It iterates through the list of URLs and returns the first area-free M3U8 playlist URL.
    ///
    /// # Returns
    ///
    /// The streaming URL as a string.
    ///
    /// # Panics
    ///
    /// Panics if no suitable streaming URL is found.
    pub fn get_streaming_url(&self) -> String {
        debug!("{:#?}", self.list);
        for i in &self.list[0..1] {
            match i {
                Urlset::Url(n) => {
                    if n.areafree == 0 {
                        let string = &n.playlist_create_url;
                        if string.contains("m3u8") {
                            return string.to_string();
                        }
                    }
                }
            }
        }
        panic!("something went wrong");
    }

    /// Initializes `ChStreamingUrl` for a given channel.
    ///
    /// # Arguments
    ///
    /// * `ch` - The channel ID (e.g., "QRR", "LFR").
    ///
    /// # Returns
    ///
    /// A `ChStreamingUrl` struct containing streaming URL information.
    pub fn init(ch: &str) -> ChStreamingUrl {
        let client = Client::new();
        let url = format!("http://radiko.jp/v2/station/stream_smh_multi/{ch}.xml");
        debug!("{:#?}", &url);
        match client.get(url).send() {
            Ok(m) => {
                let streamingurl: ChStreamingUrl = match from_str(match &m.text() {
                    Ok(n) => n,
                    Err(e) => panic!("{}", e),
                }) {
                    Ok(l) => l,
                    Err(e) => {
                        panic!("{:#?}", e)
                    }
                };
                streamingurl
            }
            Err(e) => {
                error!("{}", e);
                panic!("{}", e);
            }
        }
    }
}

/// Fetches the program DOM for a given channel.
///
/// # Arguments
///
/// * `ch` - The channel ID (e.g., "QRR", "LFR").
///
/// # Returns
///
/// A `reqwest::blocking::Response` containing the program DOM.
pub fn get_program_dom(ch: &str) -> Response {
    let client = Client::new();
    let url = format!("http://radiko.jp/v2/api/program/station/weekly?station_id={ch}");
    info!("{:#?}", &url);
    match client.get(url).send() {
        Ok(m) => m,
        Err(e) => {
            error!("{}", e);
            panic!("{}", e);
        }
    }
}
impl Program<'_> {
    /// Convert the program's `ft` timestamp (format `YYYYMMDDHHMMSS`) into a `DateTime<Local>`.
    ///
    /// Returns `Ok(DateTime<Local>)` when parsing and local-time resolution succeed,
    /// or `Err(String)` with a brief error message if parsing fails or the local time is ambiguous/invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// let prog = Program { ft: "20240101123000".into(), to: "".into(), ftl: "".into(), tol: "".into(), dur: 0, title: "".into() };
    /// let dt = prog.parse_time().unwrap();
    /// assert_eq!(dt.format("%Y%m%d%H%M%S").to_string(), "20240101123000");
    /// ```
    fn parse_time(&self) -> Result<DateTime<Local>, String> {
        match NaiveDateTime::parse_from_str(self.ft.as_ref(), "%Y%m%d%H%M%S") {
            Ok(naive_dt) => {
                // Localタイムゾーンに変換
                match Local.from_local_datetime(&naive_dt).single() {
                    Some(local_dt) => Ok(local_dt),
                    None => Err("Ambiguous or invalid local time".to_string()),
                }
            }
            Err(e) => Err(format!("Parse error: {}", e)),
        }
    }
    /// Determines whether the program should be recorded based on its title.
    ///
    /// The program is considered invalid for recording when its title is empty or
    /// contains the phrases "放送休止" or "番組休止".
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    ///
    /// let good = Program {
    ///     ft: Cow::Borrowed("20250101000000"),
    ///     to: Cow::Borrowed("20250101010000"),
    ///     ftl: Cow::Borrowed("0000"),
    ///     tol: Cow::Borrowed("0100"),
    ///     dur: 3600,
    ///     title: Cow::Borrowed("Morning Show"),
    /// };
    /// assert!(good.validate_program());
    ///
    /// let bad = Program {
    ///     title: Cow::Borrowed("番組休止"),
    ///     ft: Cow::Borrowed("20250101000000"),
    ///     to: Cow::Borrowed("20250101010000"),
    ///     ftl: Cow::Borrowed("0000"),
    ///     tol: Cow::Borrowed("0100"),
    ///     dur: 3600,
    /// };
    /// assert!(!bad.validate_program());
    /// ```
    ///
    /// @returns `true` if the program title is not empty and does not contain "放送休止" or "番組休止", `false` otherwise.
    fn validate_program(&self) -> bool {
        if self.title.is_empty()
            || self.title.contains("放送休止")
            || self.title.contains("番組休止")
        {
            return false;
        }
        true
    }
}
#[test]
fn pass_auth1() {
    assert_eq!(
        RecordRadiko::auth1().unwrap().status(),
        http::StatusCode::OK
    )
}

/// Verifies that Radiko's two-step authentication succeeds when given a correctly-derived partial key.
///
/// Calls `auth1` to obtain the authentication headers, extracts the token, key offset, and key length,
/// builds the partial key from a known auth key using the offset/length, then calls `auth2` and asserts
/// the response status is 200 OK.
///
/// # Examples
///
/// ```
/// let resp = RecordRadiko::auth1().unwrap();
/// let headers = resp.headers();
/// let radiko_authkey_value = String::from("bcd151073c03b352e1ef2fd66c32209da9ca0afa");
/// let authtoken = headers.get("x-radiko-authtoken").unwrap().to_str().unwrap();
/// let key_length: u8 = headers.get("x-radiko-keylength").unwrap().to_str().unwrap().parse().unwrap();
/// let keyoffset: usize = headers.get("x-radiko-keyoffset").unwrap().to_str().unwrap().parse().unwrap();
/// let partial_key = general_purpose::STANDARD.encode(&radiko_authkey_value[keyoffset..(keyoffset + key_length as usize)]);
/// assert_eq!(RecordRadiko::auth2(authtoken, partial_key).unwrap().status(), http::StatusCode::OK);
/// ```
#[test]
fn pass_auth2() {
    let resp = RecordRadiko::auth1().unwrap();
    let header_str = resp.headers();

    let radiko_authkey_value = String::from("bcd151073c03b352e1ef2fd66c32209da9ca0afa");

    let authtoken = header_str
        .get("x-radiko-authtoken")
        .expect("Failed to get auth-token")
        .to_str()
        .unwrap();
    let key_length: u8 = header_str
        .get("x-radiko-keylength")
        .expect("Failed to get keylength")
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    let keyoffset: usize = header_str
        .get("x-radiko-keyoffset")
        .expect("Failed to get keyoffset")
        .to_str()
        .unwrap()
        .parse()
        .expect("Failed to parse to integer");
    let partial_key = general_purpose::STANDARD
        .encode(&radiko_authkey_value[keyoffset..(keyoffset + key_length as usize)]);
    assert_eq!(
        RecordRadiko::auth2(authtoken, partial_key)
            .unwrap()
            .status(),
        http::StatusCode::OK
    )
}

#[test]
fn false_validate_program_bangumi_kyushi() {
    let prog = Program {
        ft: Cow::from("20211122060000"),
        to: Cow::from("20211122070000"),
        ftl: Cow::from("0600"),
        tol: Cow::from("0700"),
        dur: 3600,
        title: Cow::from("番組休止"),
    };
    assert!(!(prog.validate_program()))
}
#[test]
fn false_validate_program_housou_kyushi() {
    let prog = Program {
        ft: Cow::from("20211122060000"),
        to: Cow::from("20211122070000"),
        ftl: Cow::from("0600"),
        tol: Cow::from("0700"),
        dur: 3600,
        title: Cow::from("放送休止"),
    };
    assert!(!(prog.validate_program()))
}

impl ProgDate {
    /// Parse the stored YYYYMMDD integer into a `NaiveDate`.
    ///
    /// Panics if the value cannot be parsed as a valid date.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::NaiveDate;
    /// let pd = ProgDate { value: 20230102 };
    /// let d = pd._parse_date();
    /// assert_eq!(d, NaiveDate::from_ymd(2023, 1, 2));
    /// ```
    fn _parse_date(&self) -> NaiveDate {
        match NaiveDate::parse_from_str(&self.value.to_string(), "%Y%m%d") {
            Ok(m) => m,
            Err(e) => panic!("{:#?}", e),
        }
    }
}
#[test]
fn test_parse_date() {
    let progdate = ProgDate { value: 20211125 };

    assert_eq!(
        progdate._parse_date(),
        NaiveDate::parse_from_str("20211125", "%Y%m%d").unwrap()
    )
}