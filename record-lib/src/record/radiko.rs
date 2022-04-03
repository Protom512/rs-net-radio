use chrono;
use chrono::{DateTime, Local, NaiveDate, TimeZone};
// use http::Uri;
use log::{debug, error, info};
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
// use serde_json::to_string;

use fs_extra::file::CopyOptions;
use serde_xml_rs::from_str;
use std::borrow::Cow;
use std::env::temp_dir;
use std::path::Path;
use std::process::{Command, ExitStatus};
use std::{env, fs};
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
#[derive(Debug, PartialEq)]
pub struct RecordRadiko {
    title: String,
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

    pub fn download(&self) -> ExitStatus {
        let resp = RecordRadiko::auth1();
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
        // let end = u8::try_from(keyoffset + key_length).expect("Failed to convert to u8");
        let partial_key = base64::encode(
            &radiko_authkey_value[keyoffset as usize..(keyoffset + key_length as usize) as usize],
        );
        let resp = RecordRadiko::auth2(authtoken, partial_key);
        debug!("{:#?}\n", &resp.text().expect("Failed to get resp body"));

        // get archive path
        let archive_path = match env::var("RS_NET_ARCHIVE_PATH") {
            Ok(n) => {
                let path = format!("{}/radiko", n,);
                debug!("{:#?}", &path);
                if !Path::new(&path).is_dir() {
                    match fs::create_dir(&path) {
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

        // create file_name
        let filename = format!("{}_{}.mp4", self.ft.format("%Y%m%d%H%M%S"), self.title);
        // format characters
        let filename = RecordRadiko::format_forbidden_char(filename.as_str());
        let output_path = format!("{}/{}", archive_path, &filename);
        let working_path = format!("{}/{}", tmpdir, &filename);

        let output = Command::new("ffmpeg")
            .arg("-loglevel")
            .arg("debug")
            .arg("-fflags")
            .arg("+discardcorrupt")
            .arg("-headers")
            .arg(format!(
                "X-Radiko-Authtoken: {authtoken}",
                authtoken = authtoken
            ))
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
            .output()
            .expect("Failed to execute ffmpeg");
        let converted: String = String::from_utf8(output.stderr).unwrap();

        if output.status.success() {
            let options = CopyOptions::new();
            fs_extra::file::move_file(&working_path, &output_path, &options)
                .expect("Failed to archive file");
        } else {
            error!("{}", converted);
        }
        output.status
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
    fn auth1() -> Response {
        let client = Client::new();
        let url = "https://radiko.jp/v2/api/auth1";
        match client
            .get(url)
            .header("pragma", "no-cache")
            .header("X-Radiko-App", "pc_html5")
            .header("X-Radiko-App-Version", "0.0.1")
            .header("X-Radiko-User", "test-stream")
            .header("X-Radiko-Device", "pc")
            .send()
        {
            Ok(n) => {
                //debug!("{:#?}", n);
                n
            }
            Err(e) => {
                panic!("{}", e);
            }
        }
        /*

                {
            "server": "nginx",
            "date": "Fri, 10 Dec 2021 03:35:40 GMT",
            "content-type": "text/plain",
            "transfer-encoding": "chunked",
            "connection": "keep-alive",
            "x-radiko-apptype": "pc",
            "x-radiko-apptype2": "pc",
            "x-radiko-authtoken": "O_rlqtaPquyAH6sAIBGopg",
            "x-radiko-authwait": "0",
            "x-radiko-delay": "15",
            "x-radiko-keylength": "16",
            "x-radiko-keyoffset": "0",
            "access-control-expose-headers": "X-Radiko-AuthToken, X-Radiko-Partialkey, X-Radiko-AppType, X-Radiko-AuthWait, X-Radiko-Delay, X-Radiko-KeyLength, X-Radiko-KeyOffset, X-Radiko-SubStation",
            "access-control-allow-credentials": "true",
        }


                */
    }
    fn auth2(token: &str, partial_key: String) -> Response {
        let client = Client::new();
        let url = "https://radiko.jp/v2/api/auth2";
        match client
            .get(url)
            .header("pragma", "no-cache")
            .header("X-Radiko-User", " test-stream")
            .header("X-Radiko-Device", "pc")
            .header("X-Radiko-AuthToken", token)
            .header("X-Radiko-PartialKey", partial_key)
            .send()
        {
            Ok(n) => {
                //debug!("{:#?}", n);
                n
            }
            Err(e) => {
                panic!("{}", e);
            }
        }
    }
}

impl Radiko<'_> {
    pub fn init(ch: &str) -> Self {
        let m = get_program_dom(ch);
        let radiko: Radiko = match from_str(match &m.text() {
            Ok(l) => {
                // debug!("{:#?}", l);
                l
            }
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

        //url.to_string()
    }
    pub fn init(ch: &str) -> ChStreamingUrl {
        let client = Client::new();
        let url = format!(
            "http://radiko.jp/v2/station/stream_smh_multi/{channel}.xml",
            channel = ch
        );
        //    stream_url=`xmllint --xpath "/urls/url[@areafree='0'][1]/playlist_create_url/text()" ${channel}.xml`

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
    let url = format!(
        "http://radiko.jp/v2/api/program/station/weekly?station_id={ch}",
        ch = ch
    );
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
        return match Local.datetime_from_str((&self.ft).as_ref(), "%Y%m%d%H%M%S") {
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
    assert_eq!(RecordRadiko::auth1().status(), http::StatusCode::OK)
}

#[test]
fn pass_auth2() {
    let resp = RecordRadiko::auth1();
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
    // let end = u8::try_from(keyoffset + key_length).expect("Failed to convert to u8");
    let partial_key = base64::encode(
        &radiko_authkey_value[keyoffset as usize..(keyoffset + key_length as usize) as usize],
    );
    assert_eq!(
        RecordRadiko::auth2(authtoken, partial_key).status(),
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
        // pfm: None,
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
        // pfm: None,
    };
    assert!(!(prog.validate_program()))
}

impl ProgDate {
    fn parse_date(&self) -> NaiveDate {
        return match NaiveDate::parse_from_str(&*self.value.to_string(), "%Y%m%d") {
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
        NaiveDate::parse_from_str(&*"20211125".to_string(), "%Y%m%d").unwrap()
    )
}
