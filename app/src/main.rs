use chrono::Timelike;
use chrono::{offset::TimeZone, DateTime, Datelike, Duration, Local, Utc};
use env_logger::fmt::Color;
use env_logger::Builder;
use log::{debug, error, info, Level};
use std::error::Error; //use log::LevelFilter;
use std::io::Write;
extern crate record_lib;
use record_lib::record::ag::Ag;
use record_lib::record::onsen::OnsenProgram;

use record_lib::record::hibiki::record;
use record_lib::record::radiko::RecordRadiko;
use tokio_cron_scheduler::{Job, JobScheduler};

fn job_radiko(init_schedule: &str, ch: &'static str) -> Result<Job, Box<dyn Error>> {
    info!("running job_radiko");
    debug!("{}", &init_schedule);
    let current_time = Local::now();
    Job::new(init_schedule, move |_uuid, _l| {
        let mut record_sched = JobScheduler::new();
        let arr = RecordRadiko::init(ch);

        for radiko in arr {
            if current_time.timestamp() < radiko.ft.timestamp() {
                let schedule = format!(
                    "{} {} {} {} {} * {}",
                    radiko.ft.with_timezone(&Utc).second(),
                    radiko.ft.with_timezone(&Utc).minute(),
                    radiko.ft.with_timezone(&Utc).hour(),
                    radiko.ft.with_timezone(&Utc).day(),
                    radiko.ft.with_timezone(&Utc).month(),
                    radiko.ft.with_timezone(&Utc).year()
                );
                let job = Job::new(&schedule, move |_uuid2, _l2| {
                    let status = radiko.download();
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
}
fn job_ag(init_schedule: &str) -> Result<Job, Box<dyn Error>> {
    info!("running job_ag");
    debug!("{}", &init_schedule);
    let current_time = Local::now();
    Job::new(init_schedule, move |_uuid, _l| {
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
}

fn job_onsen(init_schedule: &str) -> Result<Job, Box<dyn Error>> {
    info!("running job_onsen");
    debug!("{}", &init_schedule);
    Job::new(init_schedule, move |_uuid, _l| {
        let json: Vec<OnsenProgram> = OnsenProgram::init();
        for i in &json {
            info!("{}", i.title);
            i.record();
        }
    })
}

fn job_hibiki(init_schedule: &str) -> Result<Job, Box<dyn Error>> {
    info!("running job_hibiki");
    debug!("{}", &init_schedule);
    Job::new(init_schedule, move |_uuid, _l| {
        record();
    })
}

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
            "{} [{}] {}:{} - {}",
            Local::now().format("%Y-%m-%dT%H:%M:%S"),
            level_style.value(record.level()),
            level_style.value(&record.file().unwrap_or("____unknown")[4..]),
            level_style.value(&record.line().unwrap_or(0)),
            level_style.value(record.args())
        )
    });
    builder.filter(None, log::LevelFilter::Info);
    builder.write_style(env_logger::WriteStyle::Auto);
    builder.init();
    let mut sched = JobScheduler::new();
    let current_time = Local::now();

    let init_schedule = "00 00 20 * * * *";

    let init_today = Local::today();
    let init_string = format!(
        "{}/{}/{} 04:00:00",
        init_today.year(),
        init_today.month(),
        init_today.day()
    );

    let init_dt: DateTime<Local> = Local
        .datetime_from_str(&init_string, "%Y/%m/%d %H:%M:%S")
        .expect("Failed to parse datetime");

    if current_time.timestamp() > init_dt.timestamp() {
        let current_shot = current_time + Duration::seconds(3);
        let schedule = format!(
            "{} {} {} {} {} * {}",
            current_shot.with_timezone(&Utc).second(),
            current_shot.with_timezone(&Utc).minute(),
            current_shot.with_timezone(&Utc).hour(),
            current_shot.with_timezone(&Utc).day(),
            current_shot.with_timezone(&Utc).month(),
            current_shot.with_timezone(&Utc).year()
        );

        let job = job_ag(&schedule).unwrap();
        sched.add(job).expect("Failed to Add job to cron");
    }
    let job = job_ag(init_schedule).unwrap();
    sched.add(job).expect("Failed to Add job to cron");
    // let mut record_sched = JobScheduler::new();

    if current_time.timestamp() > init_dt.timestamp() {
        let current_shot = current_time + Duration::seconds(3);
        let schedule = format!(
            "{} {} {} {} {} * {}",
            current_shot.with_timezone(&Utc).second(),
            current_shot.with_timezone(&Utc).minute(),
            current_shot.with_timezone(&Utc).hour(),
            current_shot.with_timezone(&Utc).day(),
            current_shot.with_timezone(&Utc).month(),
            current_shot.with_timezone(&Utc).year()
        );

        let job = job_onsen(&schedule).expect("Failed to create Job");
        sched.add(job).expect("Failed to Add job to cron");
    }

    let job = job_onsen(init_schedule).expect("Failed to create Job");
    sched.add(job).expect("Failed to Add job to cron");
    //radiko

    if current_time.timestamp() > init_dt.timestamp() {
        let current_shot = current_time + Duration::seconds(3);
        let schedule = format!(
            "{} {} {} {} {} * {}",
            current_shot.with_timezone(&Utc).second(),
            current_shot.with_timezone(&Utc).minute(),
            current_shot.with_timezone(&Utc).hour(),
            current_shot.with_timezone(&Utc).day(),
            current_shot.with_timezone(&Utc).month(),
            current_shot.with_timezone(&Utc).year()
        );

        let job = job_radiko(&schedule, "QRR").expect("Failed to create Job");
        sched.add(job).expect("Failed to Add job to cron");
        let job = job_radiko(&schedule, "LFR").expect("Failed to create Job");
        sched.add(job).expect("Failed to Add job to cron");
    }

    let job = job_radiko(init_schedule, "QRR").expect("Failed to create Job");
    sched.add(job).expect("Failed to Add job to cron");
    let job = job_radiko(init_schedule, "LFR").expect("Failed to create Job");
    sched.add(job).expect("Failed to Add job to cron");

    //10時でおんせｎと重ならないように

    let init_schedule = "00 00 01 * * * *";

    if current_time.timestamp() > init_dt.timestamp() {
        let current_shot = current_time + Duration::seconds(3);
        let schedule = format!(
            "{} {} {} {} {} * {}",
            current_shot.with_timezone(&Utc).second(),
            current_shot.with_timezone(&Utc).minute(),
            current_shot.with_timezone(&Utc).hour(),
            current_shot.with_timezone(&Utc).day(),
            current_shot.with_timezone(&Utc).month(),
            current_shot.with_timezone(&Utc).year()
        );

        let job = job_hibiki(&schedule).expect("Failed to create Job");
        sched.add(job).expect("Failed to Add job to cron");
    }

    let job = job_hibiki(init_schedule).expect("Failed to create Job");
    sched.add(job).expect("Failed to Add job to cron");

    match sched.start().await {
        Ok(m) => m,
        Err(e) => {
            error!("{}", e);
        }
    };
}
