use chrono::Timelike;
use chrono::{offset::TimeZone, DateTime, Datelike, Duration, Local, Utc};
use env_logger::Builder;
use log::{debug, error, info};
use std::error::Error; //use log::LevelFilter;
use std::io::Write;
extern crate record_lib;
use record_lib::record::onsen::OnsenProgram;

use record_lib::record::hibiki::record;
use record_lib::record::radiko::RecordRadiko;
use tokio_cron_scheduler::{Job, JobScheduler};

fn job_radiko(init_schedule: &str, ch: &'static str) -> Result<Job, Box<dyn Error>> {
    info!("running job_radiko");
    debug!("{}", &init_schedule);
    let current_time = Local::now();
    Job::new(init_schedule, move |_uuid, _l| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let record_sched = JobScheduler::new().await.unwrap(); // Removed mut
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
                    let radiko_clone = radiko.clone(); // Clone for the closure
                    let job = Job::new(schedule.as_str(), move |_uuid2, _l2| {
                        // Used .as_str()
                        info!("Executing Radiko record for: {}", radiko_clone.title);
                        match radiko_clone.download() {
                            Ok(status) => {
                                if status.success() {
                                    info!(
                                        "Radiko Record successful for {}: {}",
                                        radiko_clone.title, status
                                    );
                                } else {
                                    error!(
                                        "Radiko Record command failed for {}: {}",
                                        radiko_clone.title, status
                                    );
                                }
                            }
                            Err(e) => {
                                error!(
                                    "Radiko Record execution error for {}: {}",
                                    radiko_clone.title, e
                                );
                            }
                        }
                    })
                    .unwrap();
                    record_sched.add(job).await.unwrap(); // Added .await and unwrap
                }
            }
            let _res = record_sched.start().await;
        });
    })
    .map_err(Box::from)
}
fn job_onsen(init_schedule: &str) -> Result<Job, Box<dyn Error>> {
    info!("running job_onsen");
    debug!("{}", &init_schedule);
    Job::new(init_schedule, move |_uuid, _l| {
        let json: Vec<OnsenProgram> = OnsenProgram::init();
        for onsen_program in &json {
            info!("Executing Onsen record for: {}", onsen_program.title);
            match onsen_program.record() {
                Ok(()) => {
                    info!("Onsen Record successful for {}", onsen_program.title);
                }
                Err(e) => {
                    error!(
                        "Onsen Record execution error for {}: {}",
                        onsen_program.title, e
                    );
                }
            }
        }
    })
    .map_err(Box::from) // Added error mapping
}

fn job_hibiki(init_schedule: &str) -> Result<Job, Box<dyn Error>> {
    info!("running job_hibiki");
    debug!("{}", &init_schedule);
    Job::new(init_schedule, move |_uuid, _l| {
        info!("Executing Hibiki record job");
        record();
    })
    .map_err(Box::from) // Added error mapping
}

#[tokio::main]
async fn main() {
    let mut builder = Builder::new();
    builder.format(|buf, record| {
        /// Returns the default style for the given log level.
        /// This style includes color and formatting attributes that will be used to display log messages.
        /// The style is determined by the log level (e.g., Error, Warn, Info, Debug, Trace).
        let style = buf.default_level_style(record.level());
        writeln!(
            buf,
            "[{}] [{}:{}] {}",
            record.level(),
            record
                .file()
                .unwrap_or("____unknown")
                .get(4..)
                .unwrap_or("unknown"),
            record.line().unwrap_or(0),
            record.args()
        )
    });
    builder.filter(None, log::LevelFilter::Info);
    builder.write_style(env_logger::WriteStyle::Auto);
    builder.init();
    let sched = JobScheduler::new().await.unwrap(); // Removed mut
    let current_time = Local::now();

    let init_schedule_str = "00 00 20 * * * *";

    let init_today = Local::now();
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
    }

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

        let job = job_onsen(schedule.as_str()).expect("Failed to create Job");
        sched.add(job).await.expect("Failed to Add job to cron"); // Added .await
    }

    let job = job_onsen(init_schedule_str).expect("Failed to create Job");
    sched.add(job).await.expect("Failed to Add job to cron"); // Added .await
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

        let job = job_radiko(schedule.as_str(), "QRR").expect("Failed to create Job");
        sched.add(job).await.expect("Failed to Add job to cron"); // Added .await
        let job = job_radiko(schedule.as_str(), "LFR").expect("Failed to create Job");
        sched.add(job).await.expect("Failed to Add job to cron"); // Added .await
    }

    let job = job_radiko(init_schedule_str, "QRR").expect("Failed to create Job");
    sched.add(job).await.expect("Failed to Add job to cron"); // Added .await
    let job = job_radiko(init_schedule_str, "LFR").expect("Failed to create Job");
    sched.add(job).await.expect("Failed to Add job to cron"); // Added .await

    //10時でおんせｎと重ならないように

    let init_hibiki_schedule_str = "00 00 01 * * * *";

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

        let job = job_hibiki(schedule.as_str()).expect("Failed to create Job");
        sched.add(job).await.expect("Failed to Add job to cron"); // Added .await
    }

    let job = job_hibiki(init_hibiki_schedule_str).expect("Failed to create Job");
    sched.add(job).await.expect("Failed to Add job to cron"); // Added .await

    match sched.start().await {
        Ok(m) => m,
        Err(e) => {
            error!("{}", e);
        }
    };
}
