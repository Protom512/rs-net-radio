use chrono::Timelike;
use chrono::{Datelike, Duration, Local, Utc};
use env_logger::fmt::Color;
use env_logger::Builder;
//use log::LevelFilter;
use log::{error, info, Level};
use std::io::Write;
mod lib;
use crate::lib::record::ag::Ag;
use crate::lib::record::onsen::OnsenProgram;

use tokio_cron_scheduler::{Job, JobScheduler};
#[tokio::main]
async fn main() {
    let mut builder = Builder::new();
    builder.format(|buf, record| {
        let level_color = match record.level() {
            Level::Trace => Color::White,
            Level::Debug => Color::Blue,
            Level::Info => Color::Green,
            Level::Warn => Color::Yellow,
            Level::Error => Color::Red,
        };
        let mut level_style = buf.style();
        level_style.set_color(level_color);
        writeln!(
            buf,
            "{} [{}] - {}",
            Local::now().format("%Y-%m-%dT%H:%M:%S"),
            level_style.value(record.level()),
            level_style.value(record.args())
        )
    });
    builder.filter(None, log::LevelFilter::Debug);
    builder.write_style(env_logger::WriteStyle::Auto);
    builder.init();
    let mut sched = JobScheduler::new();
    let current_time = Local::now();
    let init_schedule = "00 00 5 * * * *";

    // let mut record_sched = JobScheduler::new();
    let job = Job::new(init_schedule, move |_uuid, _l| {
        let json: Vec<OnsenProgram> = OnsenProgram::init();
        for i in &json {
            info!("{}", i.title);
            i.record();
        }

        let mut record_sched = JobScheduler::new();
        let arr: Vec<Ag> = Ag::init();

        for ag in arr {
            let start = ag.start_datetime + Duration::seconds(-15);
            if current_time.timestamp() < start.timestamp() {
                let schedule = format!(
                    "{} {} {} {} {} * {}",
                    start.with_timezone(&Utc).second(),
                    start.with_timezone(&Utc).minute(),
                    start.with_timezone(&Utc).hour(),
                    start.with_timezone(&Utc).day(),
                    start.with_timezone(&Utc).month(),
                    start.with_timezone(&Utc).year()
                );
                let job = Job::new(&schedule, move |_uuid2, _l2| {
                    let status = ag.clone().record().unwrap();
                    if status.success() {
                        info!("ExitStatus:{}", status);
                    } else {
                        error!("{}", status);
                    }
                })
                .unwrap();
                record_sched.add(job).unwrap();
            }
        }
        let _res = tokio::spawn(record_sched.start());
    })
    .unwrap();
    sched.add(job).unwrap();
    let _res = match sched.start().await {
        Ok(m) => m,
        Err(e) => {
            error!("{}", e);
        }
    };
}
