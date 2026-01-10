use tokio::process::Command;
use std::process::ExitStatus;
use std::time::Duration;
use tokio::time::timeout;
use log::{error, info, debug};
use crate::utils::RecordError;

/// 外部プロセスの実行結果を保持する構造体
pub struct ProcessResult {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

/// 非同期コマンド実行ヘルパー
/// ffmpeg や streamlink などの外部コマンドを非同期に実行する
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
        let output = child.wait_with_output().await.map_err(|e| RecordError::Io(e))?;
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
                Err(RecordError::Other(format!("Command {} timed out after {:?}", cmd_name, d)))
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
        _ => result
    }
}

/// ffmpeg 特有の録画実行ラッパー（将来的に Output Adapter となる）
pub async fn record_with_ffmpeg(
    input_url: &str,
    output_path: &str,
    duration: Duration,
) -> Result<(), RecordError> {
    let duration_str = format!("{}", duration.as_secs());
    let args = [
        "-i", input_url,
        "-t", &duration_str,
        "-c", "copy",
        "-y", // 上書き許可
        output_path,
    ];

    // 録画時間に少しバッファを持たせたタイムアウトを設定
    let timeout_limit = duration + Duration::from_secs(60);
    
    execute_command("ffmpeg", &args, Some(timeout_limit)).await?;
    
    info!("Successfully recorded: {}", output_path);
    Ok(())
}