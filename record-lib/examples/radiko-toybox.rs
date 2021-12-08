use chrono;
use chrono::{Date, DateTime, Local, NaiveDate, TimeZone};
use http::Uri;
use log::{error, info};
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use serde_xml_rs::from_str;
use std::fs::File;

use xml::EventReader;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Radiko {
    ttl: u32,
    srvtime: u32,
    stations: Stations,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Stations {
    station: Station,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Station {
    id: ID,
    name: String,
    scd: Scd,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum ID {
    QRR,
    LFR,
    BAYFM78,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Scd {
    progs: Vec<Progs>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Progs {
    #[serde(rename = "$value")]
    list: Vec<Progset>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum Progset {
    Date(ProgDate),
    Prog(Program),
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ProgDate {
    #[serde(rename = "$value")]
    value: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Program {
    ft: String,
    to: String,
    ftl: String,
    tol: String,
    dur: u32,
    title: String,
    pfm: Option<String>,
}

impl Radiko {}
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
impl Program {
    fn parse_time(self) -> DateTime<Local> {
        return match Local.datetime_from_str(&self.ft, "%Y%m%d%H%M%S") {
            Ok(m) => m,
            Err(e) => panic!("{:#?}", e),
        };
    }
    fn validate_program(self) -> bool {
        if self.title.is_empty() {
            return false;
        } else if self.title.contains("放送休止") {
            return false;
        } else if self.title.contains("番組休止") {
            return false;
        }
        true
    }

    fn dl_swf(&self) {
        let swf_url: String = "http://radiko.jp/apps/js/flash/myplayer-release.swf".to_string();

        let uri = match swf_url.parse::<Uri>() {
            Ok(m) => m,
            Err(error) => {
                error!("{:#?}", error);
                panic!("{:#?}", error);
            }
        };
        let use_ssl = { uri.scheme_str() == Some("https") };
        let client = Client::new();

        // client.get(uri.)
        let mut resp = match client.get(swf_url).send() {
            Ok(mut m) => m,
            Err(error) => {
                error!("{:#?}", error);
                panic!("{:#?}", error);
            }
        };

        match resp.status() {
            http::StatusCode::OK => {
                let work_dir = "/tmp";
                let path = format!("{}/swf", &work_dir);
                log::debug!("output path will be {}", &path);
                // Write contents to disk.
                let mut f = File::create(&path).expect("Unable to create file");
                // copy(&mut resp, &mut f).expect("Unable to copy data");
            }
            _ => {
                error!("hibiki;Failed to download swf");
                panic!();
            }
        }
        // self.auth();
    }

    fn auth() {
        //
        //curl -s \
        //  --header "pragma: no-cache" \
        //  --header "X-Radiko-App: pc_html5" \
        //  --header "X-Radiko-App-Version: 0.0.1" \
        //  --header "X-Radiko-User: test-stream" \
        //  --header "X-Radiko-Device: pc" \
        //  --dump-header auth1_fms_${pid} \
        //  -o /dev/null \
        // https://radiko.jp/v2/api/auth1
        unimplemented!()
    }
}
#[test]

fn false_validate_program_bangumi_kyushi() {
    let prog = Program {
        ft: "20211122060000".to_string(),
        to: "20211122070000".to_string(),
        ftl: "0600".to_string(),
        tol: "0700".to_string(),
        dur: 3600,
        title: "番組休止".to_string(),
        pfm: None,
    };
    assert_eq!(prog.validate_program(), false)
}
#[test]

fn false_validate_program_housou_kyushi() {
    let prog = Program {
        ft: "20211122060000".to_string(),
        to: "20211122070000".to_string(),
        ftl: "0600".to_string(),
        tol: "0700".to_string(),
        dur: 3600,
        title: "放送休止".to_string(),
        pfm: None,
    };
    assert_eq!(prog.validate_program(), false)
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
#[test]
#[should_panic]
fn panic_parse_date() {
    let progdate = ProgDate { value: 2021112511 };
    progdate.parse_date();
}
fn main() {
    let m = get_program_dom("BAYFM78");

    let qrr: Radiko = match from_str(match &m.text() {
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

    for i in qrr.stations.station.scd.progs {
        for j in i.list {
            match j {
                // Prog(prog) => info!("{:#}", prog),
                Progset::Prog(n) => {
                    // info!("{}", &n.parse_time());
                    info!("{}", &n.title);
                    n.dl_swf();
                }
                Progset::Date(date) => error!("geee: {:#?} ", date),
            }
            // info!("{:#?}", j);
        }
    }
}
