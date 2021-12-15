use chrono::{Date, DateTime, Local};
use env_logger;
use log::{debug, error, info, warn};
use record_lib::record::radiko;
use record_lib::record::radiko::{Progset, Urlset};
use std::borrow::Borrow;
#[derive(Debug)]
struct Hoge<'a> {
    title: String,
    ft: DateTime<Local>,
    dur: u32,
    url: &'a str,
}
fn main() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    let radiko = radiko::Radiko::init("QRR");
    let streaming_url = radiko::ChStreamingUrl::init("QRR");
    let url = streaming_url.get_streaming_url();
    debug!("{}", &url);

    // let current_time = Local::now();
    let mut hoge = Vec::<Hoge>::new();
    for i in radiko.stations.station.scd.progs {
        for j in i.list {
            match j {
                // Prog(prog) => info!("{:#}", prog),
                Progset::Prog(n) => {
                    hoge.push(Hoge {
                        title: n.title.as_ref().to_string(),
                        ft: n.parse_time(),
                        dur: n.dur,
                        url: &url,
                    })
                    // n.download()
                    // info!("{}", &n.title);
                }
                Progset::Date(date) => error!("geee: {:#?} ", date),
            }
            // info!("{:#?}", j);
        }
    }
    debug!("{:#?}", hoge);
}
