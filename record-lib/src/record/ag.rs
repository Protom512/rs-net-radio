use chrono::{Date, DateTime, Duration, Local};

use log::{debug, error, info};

use std::fmt::Debug;
use std::path::Path;
use std::process::Command;
use std::process::ExitStatus;
use std::{env, fs, str};

#[derive(Clone, Debug, PartialEq)]
pub struct Ag {
    pub title: String,
    pub start_datetime: DateTime<Local>,
    pub end_datetime: DateTime<Local>,
}

impl Ag {
    ///
    ///  # ag+の録画関数
    ///
    pub fn record(self) -> Result<ExitStatus, std::io::Error> {
        let start = self.start_datetime + Duration::seconds(-15);
        let file_name = format!(
            "{}_{}.mp4",
            self.start_datetime.format("%Y%m%d_%H%M%S"),
            self.title
        );
        let output_path = match env::var("RS_NET_ARCHIVE_PATH") {
            Ok(n) => {
                let path = format!("{}/ag", n);
                if !Path::new(&path).is_dir() {
                    match fs::create_dir(format!("{}/ag", n)) {
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
        let path_string = format!("{}/{}", output_path, file_name);
        debug!("{}", path_string);
        info!("Title: {}", self.title);
        info!("StartTime: {}", self.start_datetime);
        let path = Path::new(&path_string);
        path.exists();
        let arg = self.end_datetime - start;
        info!("Duration: {}", arg);

        let agqr_stream_url = "https://fms2.uniqueradio.jp/agqr10/aandg1.m3u8";
        let res = Command::new("streamlink")
            .arg(agqr_stream_url)
            .arg("best")
            .arg("-o")
            .arg(path)
            .arg("-l")
            .arg("info")
            .arg("--hls-duration")
            .arg(format!(
                "{}:{}:{}",
                arg.num_hours(),
                arg.num_minutes() % 60,
                arg.num_seconds() % 60
            ))
            .status();

        info!("EndTime: {}", self.end_datetime);
        return match res {
            Ok(m) => {
                info!("ExitStatus:{}", m);
                Ok(m)
            }
            Err(e) => {
                error!("{}", e);
                Err(e)
            }
        };
    }

    pub fn new(
        title: &str,
        start_datetime: &DateTime<Local>,
        end_datetime: &DateTime<Local>,
    ) -> Ag {
        Ag {
            title: title.to_string(),
            start_datetime: *start_datetime,
            end_datetime: *end_datetime,
        }
    }
    pub fn get_html() -> reqwest::blocking::Response {
        return match reqwest::blocking::get(
            "https://www.joqr.co.jp/qr/agdailyprogram/agdailyprogram.html",
        ) {
            Ok(n) => n,
            Err(e) => {
                error!("{}", e);
                panic!("{}", e);
            }
        };
    }
    pub fn html_parse(get_result: reqwest::blocking::Response) -> Vec<Ag> {
        let body = match get_result.text() {
            Ok(n) => n,
            Err(e) => {
                error!("{}", e);
                panic!("{}", e);
            }
        };

        let selector_fragment =
            scraper::Selector::parse("article.dailyProgram-itemBox.ag ").unwrap();
        let selector = scraper::Selector::parse(" div.dailyProgram-itemContainer >div.js-readmore> div.dailyProgram-itemDetail > p.dailyProgram-itemTitle >a").unwrap();
        let selector_time = scraper::Selector::parse(" div.dailyProgram-itemHeader >h3").unwrap();

        let document = scraper::Html::parse_document(&body);

        for x in &document.errors {
            error!("{}", x)
        }
        // セレクターを用いて要素を取得
        let elements = document.select(&selector_fragment);
        let mut arr = Vec::<Ag>::new();
        let mut datetime_str;
        let local_date: Date<Local> = Local::today();
        for i in elements {
            let mut start_offset_h: Duration = Duration::hours(0);
            let mut start_offset_m: Duration = Duration::minutes(0);
            let mut end_offset_h: Duration = Duration::hours(0);
            let mut end_offset_m: Duration = Duration::minutes(0);
            let mut title = "";
            for j in i.select(&selector) {
                title = j.text().next().unwrap();
            }

            for j in i.select(&selector_time) {
                datetime_str = j.text().next().unwrap();
                let vec_str: Vec<&str> = datetime_str.split(" – ").collect();
                let start_t = vec_str[0];
                let end_t = vec_str[1];
                let start_vec: Vec<&str> = start_t.split(':').collect();
                let start_h: i64 = start_vec[0].parse::<i64>().unwrap();
                let start_m: i64 = start_vec[1].parse::<i64>().unwrap();
                start_offset_h = Duration::hours(start_h);
                start_offset_m = Duration::minutes(start_m);
                let end_vec: Vec<&str> = end_t.split(':').collect();
                let end_h: i64 = end_vec[0].parse::<i64>().unwrap();
                let end_m: i64 = end_vec[1].parse::<i64>().unwrap();
                end_offset_h = Duration::hours(end_h);
                end_offset_m = Duration::minutes(end_m);
            }
            let start_hms = local_date.and_hms(0, 0, 0) + start_offset_h + start_offset_m;
            let end_hms = local_date.and_hms(0, 0, 0) + end_offset_h + end_offset_m;
            arr.push(Ag::new(&title.to_string(), &start_hms, &end_hms));
        }
        arr
    }

    pub fn init() -> Vec<Ag> {
        let get_result = Ag::get_html();
        Ag::html_parse(get_result)
    }
}
