use crate::utils::RecordError;
use log::{debug, error, info};
use std::process::ExitStatus;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// 外部プロセスの実行結果を保持する構造体
pub struct ProcessResult {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

/// Execute an external command, capturing its exit status, stdout, and stderr.
///
/// Runs the given command with the provided arguments and returns a `ProcessResult` containing the process exit status and captured output. If `timeout_duration` is provided and the command does not complete within that duration, the function attempts to kill the child process and returns `RecordError::Other` with a timeout message. If the process exits with a non-success status, the function returns `RecordError::CommandFailed` containing the command name, exit code, and captured stderr.
///
/// # Parameters
///
/// - `cmd_name`: The command executable name to run.
/// - `args`: Arguments to pass to the command.
/// - `timeout_duration`: Optional timeout after which the child process will be killed and a timeout error returned.
///
/// # Returns
///
/// `Ok(ProcessResult)` on successful execution (zero or success exit status), or an appropriate `RecordError` on I/O errors, timeouts, or non-successful exit statuses.
///
/// # Examples
///
/// ```
/// # use std::time::Duration;
/// # use tokio_test::block_on;
/// # async fn try_execute() -> Result<(), Box<dyn std::error::Error>> {
/// let res = crate::execute_command("echo", &["hello"], Some(Duration::from_secs(5))).await?;
/// assert!(res.stdout.contains("hello"));
/// # Ok(()) }
/// # let _ = block_on(try_execute());
/// ```
pub async fn execute_command(
    cmd_name: &str,
    args: &[&str],
    timeout_duration: Option<Duration>,
) -> Result<ProcessResult, RecordError> {
    info!("Executing command: {} {}", cmd_name, args.join(" "));

    let mut child = Command::new(cmd_name)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| RecordError::Io(e))?;

    let execution = async {
        let output = child
            .wait_with_output()
            .await
            .map_err(|e| RecordError::Io(e))?;
        Ok(ProcessResult {
            status: output.status,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    };

    let result = if let Some(d) = timeout_duration {
        match timeout(d, execution).await {
            Ok(res) => res,
            Err(_) => {
                // タイムアウト時にプロセスを殺す試み
                let _ = child.kill().await;
                Err(RecordError::Other(format!(
                    "Command {} timed out after {:?}",
                    cmd_name, d
                )))
            }
        }
    } else {
        execution.await
    };

    match &result {
        Ok(res) if !res.status.success() => {
            error!("Command failed: {} \nStderr: {}", cmd_name, res.stderr);
            Err(RecordError::CommandFailed {
                command: cmd_name.to_string(),
                exit_code: res.status.code(),
                stderr: res.stderr.clone(),
            })
        }
        _ => result,
    }
}

/// Record a media stream to a file using ffmpeg for a specified duration.
///
/// This function invokes the `ffmpeg` executable to capture `input_url` into `output_path`
/// for `duration`. It applies a safety timeout of `duration + 60s`; if the ffmpeg process
/// does not finish before that timeout, the function returns an error.
///
/// # Returns
///
/// `Ok(())` on successful recording, `Err(RecordError)` otherwise.
///
/// # Examples
///
/// ```rust,no_run
/// use std::time::Duration;
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// record_with_ffmpeg("rtmp://example/live/stream", "/tmp/output.ts", Duration::from_secs(30)).await?;
/// # Ok(()) }
/// ```
pub async fn record_with_ffmpeg(
    input_url: &str,
    output_path: &str,
    duration: Duration,
) -> Result<(), RecordError> {
    let duration_str = format!("{}", duration.as_secs());
    let args = [
        "-i",
        input_url,
        "-t",
        &duration_str,
        "-c",
        "copy",
        "-y", // 上書き許可
        output_path,
    ];

    // 録画時間に少しバッファを持たせたタイムアウトを設定
    let timeout_limit = duration + Duration::from_secs(60);

    execute_command("ffmpeg", &args, Some(timeout_limit)).await?;

    info!("Successfully recorded: {}", output_path);
    Ok(())
}