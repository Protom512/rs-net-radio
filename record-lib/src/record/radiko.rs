use log::error;
use reqwest::blocking::{Client, Response};
struct Radiko {}
impl Radiko {
    pub fn get_program_dom(ch: &str) -> Response {
        let client = Client::new();
        let url = format!(
            "http://radiko.jp/v2/api/program/station/weekly?station_id={ch}",
            ch = ch
        );
        match client.get(url).send() {
            Ok(m) => m,

            Err(e) => {
                error!("{}", e);
                panic!("{}", e);
            }
        }
    }
}
