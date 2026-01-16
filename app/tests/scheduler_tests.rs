use std::time::Duration;
use tokio::time::{timeout, Duration as TokioDuration};
use tokio_cron_scheduler::{Job, JobScheduler};
// Removed unused tokio::sync::oneshot
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_one_shot_job() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create channel. 'mut rx' is needed to call recv()
    let (tx, mut rx) = mpsc::channel::<()>(1);

    let sched = JobScheduler::new().await?;
    let tx_cloned = tx.clone();

    let job = Job::new_one_shot(Duration::from_millis(200), move |_uuid, _l| {
        // 2. Use try_send in the sync closure context
        let _ = tx_cloned.try_send(());
    })?;

    sched.add(job).await?;
    sched.start().await?;

    // 3. Properly wait for the signal with a timeout
    // timeout() returns Result<T, Elapsed>. rx.recv() returns Option<T>.
    timeout(TokioDuration::from_secs(5), rx.recv())
        .await
        .map_err(|_| "Test timed out: Job did not run in time")? // Error if 5s pass
        .ok_or("Channel closed: Sender dropped without sending")?; // Error if sender died

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parse_cron_with_year_field() -> Result<(), Box<dyn std::error::Error>> {
    let _job = Job::new("0 0 0 1 1 * 2026", |_u, _l| {
        // nop
    })?;
    Ok(())
}
