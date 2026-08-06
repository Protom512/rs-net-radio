use crate::utils::RecordError;
use log::{debug, error, info};
use std::process::{Command, ExitStatus};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;

/// Reads an optional child pipe to end, returning an empty buffer when absent.
///
/// Used to drain `stdout`/`stderr` independently of `Child` ownership so the
/// child process can still be killed/reaped when a timeout fires.
async fn read_pipe_to_vec<R>(pipe: Option<R>) -> Vec<u8>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut buf = Vec::new();
    if let Some(mut p) = pipe {
        let _ = p.read_to_end(&mut buf).await;
    }
    buf
}

/// 外部プロセスの実行結果を保持する構造体
pub struct ProcessResult {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

/// 非同期コマンド実行ヘルパー
/// `ffmpeg` や `streamlink` などの外部コマンドを非同期に実行する
///
/// # Errors
///
/// Returns `RecordError` if the command fails to start, times out, or returns non-zero exit code.
pub async fn execute_command(
    cmd_name: &str,
    args: &[&str],
    timeout_duration: Option<Duration>,
) -> Result<ProcessResult, RecordError> {
    info!("Executing command: {} {}", cmd_name, args.join(" "));

    // `kill_on_drop(true)` is the safety net: if this future is ever dropped
    // (timeout, cancellation, or panic) the OS process is killed instead of
    // being orphaned. We additionally reap it explicitly on timeout below so
    // no zombie remains.
    let mut child = TokioCommand::new(cmd_name)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(RecordError::Io)?;

    // Drain stdout/stderr in separate tasks so `child` stays exclusively ours
    // to kill/reap when a timeout fires.
    let stdout_task = tokio::spawn(read_pipe_to_vec(child.stdout.take()));
    let stderr_task = tokio::spawn(read_pipe_to_vec(child.stderr.take()));

    let status = if let Some(d) = timeout_duration {
        match timeout(d, child.wait()).await {
            Ok(s) => s,
            Err(_elapsed) => {
                // Timeout: kill and reap so ffmpeg does not keep consuming the
                // stream/network/disk as an orphan or zombie.
                let _ = child.start_kill();
                let _ = child.wait().await;
                return Err(RecordError::Other(format!(
                    "Command {cmd_name} timed out after {d:?}"
                )));
            }
        }
    } else {
        child.wait().await
    };

    let status = status.map_err(RecordError::Io)?;
    let stdout_bytes = stdout_task.await.unwrap_or_default();
    let stderr_bytes = stderr_task.await.unwrap_or_default();

    let result = Ok(ProcessResult {
        status,
        stdout: String::from_utf8_lossy(&stdout_bytes).to_string(),
        stderr: String::from_utf8_lossy(&stderr_bytes).to_string(),
    });

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

/// `FFmpeg` 特有の録画実行ラッパー（将来的に Output Adapter となる）
///
/// # Errors
///
/// Returns `RecordError` if ffmpeg fails to execute or returns non-zero exit code.
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

    info!("Successfully recorded: {output_path}");
    Ok(())
}

/// `FFmpeg`入力ソース
#[derive(Debug, Clone)]
pub struct FfmpegInput {
    /// 入力URLまたはファイルパス
    pub url: String,
    /// カスタムヘッダー（HTTPリクエスト用）
    /// 各ヘッダーは "Key: Value\r\n" 形式である必要がある
    pub headers: Vec<String>,
    /// User-Agentヘッダー（特別扱い）
    pub user_agent: Option<String>,
}

impl FfmpegInput {
    /// 新しいHTTP入力を作成
    pub fn http(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers: Vec::new(),
            user_agent: None,
        }
    }

    /// 新しいファイル入力を作成
    pub fn file(path: impl Into<String>) -> Self {
        Self {
            url: path.into(),
            headers: Vec::new(),
            user_agent: None,
        }
    }

    /// ヘッダーを追加（"Key: Value\r\n"形式）
    #[must_use]
    pub fn header(mut self, header: impl Into<String>) -> Self {
        self.headers.push(header.into());
        self
    }

    /// 複数のヘッダーを追加
    #[must_use]
    pub fn headers(mut self, headers: Vec<String>) -> Self {
        self.headers.extend(headers);
        self
    }

    /// User-Agentを設定
    #[must_use]
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }
}

/// 非同期版`FFmpeg`コマンドビルダー
/// hibiki, onsen, radikoなどのレコーダーで使用する
#[derive(Debug, Default)]
pub struct FfmpegCommand {
    log_level: Option<String>,
    inputs: Vec<FfmpegInput>,
    video_codec: Option<String>,
    audio_codec: Option<String>,
    bitstream_filter: Option<String>,
    duration: Option<String>,
    output_path: Option<String>,
    overwrite: bool,
    no_video: bool,
    fflags: Option<String>,
}

impl FfmpegCommand {
    /// 新しい`FFmpeg`コマンドを作成
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// ログレベルを設定（デフォルト: warning）
    #[must_use]
    pub fn log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = Some(level.into());
        self
    }

    /// 入力を追加
    #[must_use]
    pub fn input(mut self, input: FfmpegInput) -> Self {
        self.inputs.push(input);
        self
    }

    /// 複数の入力を追加
    #[must_use]
    pub fn inputs(mut self, inputs: Vec<FfmpegInput>) -> Self {
        self.inputs.extend(inputs);
        self
    }

    /// ビデオコーデックを設定
    #[must_use]
    pub fn video_codec(mut self, codec: impl Into<String>) -> Self {
        self.video_codec = Some(codec.into());
        self
    }

    /// オーディオコーデックを設定
    #[must_use]
    pub fn audio_codec(mut self, codec: impl Into<String>) -> Self {
        self.audio_codec = Some(codec.into());
        self
    }

    /// ビットストリームフィルタを設定（例: `aac_adtstoasc`）
    #[must_use]
    pub fn bitstream_filter(mut self, filter: impl Into<String>) -> Self {
        self.bitstream_filter = Some(filter.into());
        self
    }

    /// 録画時間を設定（秒）
    #[must_use]
    pub fn duration(mut self, seconds: u64) -> Self {
        self.duration = Some(seconds.to_string());
        self
    }

    /// 出力パスを設定
    #[must_use]
    pub fn output(mut self, path: impl Into<String>) -> Self {
        self.output_path = Some(path.into());
        self
    }

    /// 上書きを有効にする（-y）
    #[must_use]
    pub fn overwrite(mut self, yes: bool) -> Self {
        self.overwrite = yes;
        self
    }

    /// ビデオなし（-vn）
    #[must_use]
    pub fn no_video(mut self) -> Self {
        self.no_video = true;
        self
    }

    /// `FFmpeg`フラグを設定（例: +discardcorrupt）
    #[must_use]
    pub fn fflags(mut self, flags: impl Into<String>) -> Self {
        self.fflags = Some(flags.into());
        self
    }

    /// コマンドを同期的に実行
    ///
    /// # Errors
    ///
    /// Returns `RecordError` if the command fails to execute or returns non-zero exit code.
    pub fn run(self) -> Result<FfmpegOutput, RecordError> {
        let output_path = self
            .output_path
            .as_ref()
            .ok_or_else(|| RecordError::Other("Output path not set".to_string()))?;

        let mut cmd = Command::new("ffmpeg");

        // ログレベル
        if let Some(level) = &self.log_level {
            cmd.arg("-loglevel").arg(level);
        }

        // FFmpegフラグ
        if let Some(flags) = &self.fflags {
            cmd.arg("-fflags").arg(flags);
        }

        // 入力とヘッダー
        for input in &self.inputs {
            // User-Agent
            if let Some(ua) = &input.user_agent {
                cmd.arg("-user_agent").arg(ua);
            }
            // ヘッダー（入力の前に指定する必要がある）
            for header in &input.headers {
                cmd.arg("-headers").arg(header);
            }
            // 入力URL
            cmd.arg("-i").arg(&input.url);
        }

        // 上書きフラグ
        if self.overwrite {
            cmd.arg("-y");
        }

        // ビデオコーデック
        if let Some(codec) = &self.video_codec {
            cmd.arg("-vcodec").arg(codec);
        }

        // オーディオコーデック
        if let Some(codec) = &self.audio_codec {
            cmd.arg("-acodec").arg(codec);
        }

        // ビットストリームフィルタ
        if let Some(filter) = &self.bitstream_filter {
            cmd.arg("-bsf:a").arg(filter);
        }

        // 録画時間
        if let Some(duration) = &self.duration {
            cmd.arg("-t").arg(duration);
        }

        // ビデオなし
        if self.no_video {
            cmd.arg("-vn");
        }

        // 出力パス
        cmd.arg(output_path);

        debug!("Executing ffmpeg: {cmd:?}");

        let output = cmd
            .output()
            .map_err(|e| RecordError::Other(format!("Failed to execute ffmpeg: {e}")))?;

        Ok(FfmpegOutput { output })
    }

    /// コマンドを非同期で実行
    ///
    /// # Errors
    ///
    /// Returns `RecordError` if the command fails to execute or returns non-zero exit code.
    pub async fn run_async(self) -> Result<FfmpegOutput, RecordError> {
        let output_path = self
            .output_path
            .as_ref()
            .ok_or_else(|| RecordError::Other("Output path not set".to_string()))?;

        let mut cmd = TokioCommand::new("ffmpeg");

        // ログレベル
        if let Some(level) = &self.log_level {
            cmd.arg("-loglevel").arg(level);
        }

        // FFmpegフラグ
        if let Some(flags) = &self.fflags {
            cmd.arg("-fflags").arg(flags);
        }

        // 入力とヘッダー
        for input in &self.inputs {
            // User-Agent
            if let Some(ua) = &input.user_agent {
                cmd.arg("-user_agent").arg(ua);
            }
            // ヘッダー（入力の前に指定する必要がある）
            for header in &input.headers {
                cmd.arg("-headers").arg(header);
            }
            // 入力URL
            cmd.arg("-i").arg(&input.url);
        }

        // 上書きフラグ
        if self.overwrite {
            cmd.arg("-y");
        }

        // ビデオコーデック
        if let Some(codec) = &self.video_codec {
            cmd.arg("-vcodec").arg(codec);
        }

        // オーディオコーデック
        if let Some(codec) = &self.audio_codec {
            cmd.arg("-acodec").arg(codec);
        }

        // ビットストリームフィルタ
        if let Some(filter) = &self.bitstream_filter {
            cmd.arg("-bsf:a").arg(filter);
        }

        // 録画時間
        if let Some(duration) = &self.duration {
            cmd.arg("-t").arg(duration);
        }

        // ビデオなし
        if self.no_video {
            cmd.arg("-vn");
        }

        // 出力パス
        cmd.arg(output_path);

        debug!("Executing ffmpeg: {cmd:?}");

        let output = cmd
            .output()
            .await
            .map_err(|e| RecordError::Other(format!("Failed to execute ffmpeg: {e}")))?;

        Ok(FfmpegOutput { output })
    }
}

/// `FFmpeg`実行結果
pub struct FfmpegOutput {
    output: std::process::Output,
}

impl FfmpegOutput {
    /// コマンドが成功したか
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.output.status.success()
    }

    /// 成功時に`Ok(())`、失敗時に`RecordError`を返す
    ///
    /// # Errors
    ///
    /// Returns `RecordError` if the command failed.
    pub fn into_result(self) -> Result<(), RecordError> {
        if self.is_success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&self.output.stderr).to_string();
            error!("ffmpeg failed: {stderr}");
            Err(RecordError::CommandFailed {
                command: "ffmpeg".to_string(),
                exit_code: self.output.status.code(),
                stderr,
            })
        }
    }

    /// 標準出力を取得
    #[must_use]
    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).to_string()
    }

    /// 標準エラー出力を取得
    #[must_use]
    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).to_string()
    }

    /// 終了コードを取得
    #[must_use]
    pub fn exit_code(&self) -> Option<i32> {
        self.output.status.code()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 正常系: 短時間で終了するコマンドが成功すること（パイプ読み取り含む）。
    /// クロスプラットフォームで実行され、再構成後の基本パスを検証する。
    #[tokio::test]
    async fn test_execute_command_succeeds_fast_command() {
        #[cfg(unix)]
        let (cmd, args): (&str, Vec<&str>) = ("true", vec![]);
        #[cfg(windows)]
        let (cmd, args): (&str, Vec<&str>) = ("cmd", vec!["/C", "exit", "0"]);

        let result = execute_command(cmd, &args, None).await;
        assert!(
            result.is_ok(),
            "fast command should succeed: {:?}",
            result.err()
        );
        let res = result.expect("fast command succeeds");
        assert!(res.status.success());
    }

    /// F22 回帰テスト (Unix): タイムアウト時に子プロセスが強制終了されること。
    /// `sleep 30` を 100ms タイムアウトで実行し、タイムアウトエラーが返ることを検証する。
    /// kill_on_drop + start_kill によりプロセスは残留しない。
    #[cfg(unix)]
    #[tokio::test]
    async fn test_execute_command_kills_child_on_timeout() {
        let result = execute_command("sleep", &["30"], Some(Duration::from_millis(100))).await;

        let err = result.expect_err("sleep 30 with 100ms timeout must time out");
        let msg = err.to_string();
        assert!(
            msg.contains("timed out"),
            "expected timeout error, got: {msg}"
        );
    }
}
