use record_lib::record::hibiki;
use std::env::set_var;

#[tokio::main]
async fn main() {
    // 1. 環境変数の設定
    set_var("RS_NET_ARCHIVE_PATH", "./Temp");
    set_var("RUST_LOG", "info");

    // 2. Tracing サブスクライバーの初期化
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // 3. 並列処理で録画（エラー時はアプリ境界で終了コードを決定）
    if let Err(e) = hibiki::record_parallel().await {
        tracing::error!("hibiki recording failed: {e}");
        std::process::exit(1);
    }

    tracing::info!("Finished hibiki-toybox recording example");
}
