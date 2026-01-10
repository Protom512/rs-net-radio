use record_lib::record::hibiki;
use std::env::set_var;
use tracing::debug; // log::debug ではなく tracing::debug を使う

fn main() {
    // 1. 環境変数の設定
    set_var("RS_NET_ARCHIVE_PATH", "./Temp");
    // tracingでも RUST_LOG の書式はほぼ同じです
    set_var("RUST_LOG", "debug");

    // 2. Tracing サブスクライバーの初期化
    // これで log クレート経由の出力も tracing が拾ってくれます
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // 3. 処理の実行
    hibiki::record(); // ライブラリ内の log::trace! なども表示されます

    debug!("Finished hibiki-toybox recording example");
}
