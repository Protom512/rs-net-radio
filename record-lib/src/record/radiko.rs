use chrono;
use chrono::{DateTime, Local, NaiveDate, TimeZone};
// use http::Uri;
use crate::utils::{ensure_archive_path, sanitize_filename, RecordError}; // Added RecordError
use log::{debug, error, info};
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
// use serde_json::to_string;

use fs_extra::file::CopyOptions;
use serde_xml_rs::from_str;
use std::borrow::Cow;
use std::env::temp_dir;
// use std::path::Path; // Removed
use std::process::{Command, ExitStatus};
// use std::{env, fs}; // Removed
//
// #[macro_use]
// extern crate serde_derive;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Radiko<'a> {
    ttl: u32,
    srvtime: u32,
    #[serde(borrow)]
    pub stations: Stations<'a>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Stations<'a> {
    #[serde(borrow)]
    pub station: Station<'a>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Station<'a> {
    id: ID,
    #[serde(default)]
    name: Cow<'a, str>,
    #[serde(borrow)]
    pub scd: Scd<'a>,
}
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum ID {
    QRR,
    LFR,
    BAYFM78,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Scd<'a> {
    #[serde(borrow)]
    pub progs: Vec<Progs<'a>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Progs<'a> {
    #[serde(borrow)]
    #[serde(rename = "$value")]
    pub list: Vec<Progset<'a>>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Progset<'a> {
    Date(ProgDate),
    #[serde(borrow)]
    Prog(Program<'a>),
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ProgDate {
    #[serde(rename = "$value")]
    value: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Program<'a> {
    pub ft: Cow<'a, str>,
    pub to: Cow<'a, str>,
    pub ftl: Cow<'a, str>,
    pub tol: Cow<'a, str>,
    pub dur: u32,
    pub title: Cow<'a, str>,
    // pfm: Option<&'a str>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ChStreamingUrl {
    // #[serde(borrow)]
    #[serde(rename = "$value")]
    list: Vec<Urlset>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Urlset {
    // #[serde(borrow)]
    Url(StreamingUrl),
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct StreamingUrl {
    pub areafree: u8,
    pub playlist_create_url: String,
    // media_url_path: String,
    // playlist_url_path: String,
}
#[derive(Debug, PartialEq, Clone)] // Added Clone
pub struct RecordRadiko {
    pub title: String, // Made public for access in main.rs closure
    pub ft: DateTime<Local>,
    dur: u32,
    url: String,
}
impl RecordRadiko {
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
                            let rad = RecordRadiko {
                                title: n.title.as_ref().to_string(),
                                ft: n.parse_time(),
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
        let key_length: u8 = key_length_str.parse().map_err(|e| {
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

        let partial_key =
            base64::encode(&radiko_authkey_value[keyoffset..(keyoffset + key_length as usize)]);
        let _resp_auth2 = RecordRadiko::auth2(authtoken, partial_key)?; // Propagate error from auth2
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
        fs_extra::file::move_file(&working_path, &output_path, &options).map_err(|e| {
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
            .header("X-Radiko-User", " test-stream")
            .header("X-Radiko-Device", "pc")
            .header("X-Radiko-AuthToken", token)
            .header("X-Radiko-PartialKey", partial_key)
            .send()?) // Propagate error
    }
}

impl Radiko<'_> {
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
    pub fn parse_time(&self) -> DateTime<Local> {
        return match Local.datetime_from_str(self.ft.as_ref(), "%Y%m%d%H%M%S") {
            Ok(m) => m,
            Err(e) => panic!("{:#?}", e),
        };
    }
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
    let partial_key =
        base64::encode(&radiko_authkey_value[keyoffset..(keyoffset + key_length as usize)]);
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
    fn parse_date(&self) -> NaiveDate {
        return match NaiveDate::parse_from_str(&self.value.to_string(), "%Y%m%d") {
            Ok(m) => m,
            Err(e) => panic!("{:#?}", e),
        };
    }
}
#[test]
fn test_parse_date() {
    let progdate = ProgDate { value: 20211125 };

    assert_eq!(
        progdate.parse_date(),
        NaiveDate::parse_from_str(&"20211125".to_string(), "%Y%m%d").unwrap()
    )
}
