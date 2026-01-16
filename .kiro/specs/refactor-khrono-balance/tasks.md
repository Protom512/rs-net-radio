# Implementation Plan: refactor-khrono-balance

## Target Requirements

この実装プランは以下の要件に対応します:

- **1.4**: バッチ録音完了時のサマリー出力
- **2.2**: ストリーミングデータの4KBチャンク処理とメモリ制限
- **3.2**: HTTP 403/429エラーの処理
- **3.3**: HTML解析とストリーミングURL抽出
- **4.2**: 致命的エラーの処理
- **4.3**: エラーのerror.logへの記録
- **4.4**: unwrap()/expect()の排除

## Tasks

### エラーハンドリングの強化 (要件 4.2, 4.3, 4.4)

- [ ] 1. エラーハンドリング基盤の構築
- [ ] 1.1 thiserrorとanyhowを使用したエラー型の定義 (P)
  - `thiserror`でライブラリ層のエラー型`RecordError`を定義
  - `anyhow`でアプリケーション層のエラーコンテキスト付加を実装
  - ネットワーク、IO、FFmpeg、設定、認証、HTML解析の各エラー variant を追加
  - 致命的エラーと回復可能エラーの分類ロジックを実装
  - _Requirements: 4.2, 4.4_

- [ ] 1.2 構造化ロギングの実装 (P)
  - `tracing`および`tracing-subscriber`クレートを使用したロギング設定
  - `error.log`への永続化とローテーション設定
  - エラー発生源（モジュール名、関数名）とエラー内容のログ記録
  - 致命的エラー時のスタックトレース付きログ記録
  - _Requirements: 4.3_

- [ ] 1.3 unwrap()およびexpect()の排除とエラー伝播
  - 既存コードベースから`unwrap()`と`expect()`の使用箇所を特定
  - 全ての箇所を`?`演算子または`match`式に置き換え
  - エラーコンテキストを付加するための`.context()`または`.map_err()`を追加
  - 致命的エラー時のパニック回避と終了コード1での終了処理を実装
  - _Requirements: 4.2, 4.4_

### Hibikiラジオ対応の実装 (要件 3.2, 3.3)

- [ ] 2. Hibikiラジオ対応の実装
- [ ] 2.1 HTML解析とストリーミングURL抽出
  - `hibiki_scraper.rs`モジュールを作成し、HTML解析ロジックを実装
  - `scraper`クレートまたは同様のHTMLパーサーを使用
  - ストリーミングURLの抽出とバリデーション処理
  - HTML解析失敗時のエラーメッセージ出力とログ記録
  - サイト構造変更時のエラーハンドリング（終了コード3）
  - _Requirements: 3.3_

- [ ] 2.2 HTTPステータスコード403/429のエラーハンドリング (P)
  - HTTPレスポンスのステータスコード検証ロジックを実装
  - 403または429受信時のエラーメッセージ出力
  - 終了コード2でのプロセス終了処理
  - エラー内容の`error.log`への記録
  - _Requirements: 3.2_

- [ ] 2.3 HTTPリクエストヘッダーの設定 (P)
  - `reqwest::Client`にUser-Agentヘッダ（`Mozilla/5.0 ...`）を設定
  - Refererヘッダを含めたHTTPリクエスト送信処理を実装
  - 非同期HTTPリクエストのエラーハンドリング
  - _Requirements: 3.2_

### ストリーミング録音のメモリ管理 (要件 2.2)

- [ ] 3. ストリーミング録音のメモリ管理
- [ ] 3.1 4KBチャンク処理の実装
  - ストリーミングデータ受信時の4KBチャンク単位でのディスク書き出し処理
  - チャンクバッファの実装とメモリ効率の最適化
  - 非同期I/O処理を使用したバックグラウンド書き出し
  - _Requirements: 2.2_

- [ ] 3.2 メモリ使用量の監視と制限 (P)
  - メモリ使用量のリアルタイム監視ロジックを実装
  - 512MB制限のチェックと警告メカニズム
  - 制限超過時の適切なエラーハンドリングとリソース解放
  - メモリリークの検出と防止
  - _Requirements: 2.2_

### バッチ録音のサマリー機能 (要件 1.4)

- [ ] 4. バッチ録音のサマリー機能
- [ ] 4.1 録音結果の収集と集計
  - 並列タスクの結果を`JoinSet`で収集する処理を実装
  - 成功件数、失敗件数、各タスクの処理時間を記録
  - 失敗タスクの詳細（番組名、エラーメッセージ）を収集
  - _Requirements: 1.4_

- [ ] 4.2 サマリーの出力実装
  - 標準出力へのサマリー情報出力処理
  - 成功件数、失敗件数、合計処理時間のフォーマット
  - 失敗タスクの一覧表示
  - 統計情報の計算と表示（平均処理時間、成功率など）
  - _Requirements: 1.4_

### 統合と検証

- [ ] 5. 統合と検証
- [ ] 5.1 エラーハンドリングの統合テスト
  - 致命的エラーと回復可能エラーの統合テストケース作成
  - エラーログの出力形式と内容の検証
  - 終了コードの正確性テスト
  - _Requirements: 4.2, 4.3, 4.4_

- [ ] 5.2 Hibikiラジオ対応の統合テスト (P)
  - モックHTTPサーバーを使用した403/429エラーテスト
  - モックHTMLを使用したHTML解析テスト
  - エラーハンドリングの統合テスト
  - _Requirements: 3.2, 3.3_

- [ ] 5.3 ストリーミング録音の統合テスト (P)
  - メモリ使用量の監視テスト
  - 4KBチャンク処理の検証
  - 長時間録音時のメモリ制限テスト
  - _Requirements: 2.2_

- [ ] 5.4 バッチ録音の統合テスト (P)
  - 複数タスクの並列実行テスト
  - サマリー出力の検証
  - 失敗タスクのエラーハンドリングテスト
  - _Requirements: 1.4_

- [ ] 5.5* 単体テストの追加（MVP後対応可能）
  - Hibikiスクレイパーの単体テスト（モックHTML使用）
  - エラー型のテストカバレッジ向上
  - メモリ管理の境界テスト
  - _Requirements: 3.3, 4.4_

## Task Summary

**Total Tasks**: 5 major tasks, 13 sub-tasks
**Target Requirements**: 1.4, 2.2, 3.2, 3.3, 4.2, 4.3, 4.4
**Estimated Effort**: Medium (M)
**Risk Level**: Medium

### Requirements Coverage Matrix

| Requirement | Tasks | Coverage |
|-------------|-------|----------|
| 1.4 | 4.1, 4.2, 5.4 | ✅ Complete |
| 2.2 | 3.1, 3.2, 5.3 | ✅ Complete |
| 3.2 | 2.2, 2.3, 5.2 | ✅ Complete |
| 3.3 | 2.1, 5.2, 5.5 | ✅ Complete |
| 4.2 | 1.1, 1.3, 5.1 | ✅ Complete |
| 4.3 | 1.2, 5.1 | ✅ Complete |
| 4.4 | 1.1, 1.3, 5.1, 5.5 | ✅ Complete |

### Parallel Execution Opportunities

Tasks marked with `(P)` can be executed in parallel when:
- No data dependency on other pending tasks
- No conflicting files or shared mutable resources
- No prerequisite review/approval required

**Parallel Task Groups**:
- Group 1: 1.1, 1.2, 2.2, 2.3, 3.2 (Foundation & Implementation)
- Group 2: 5.2, 5.3, 5.4 (Testing)

### Critical Path

The following tasks form the critical path and must be completed sequentially:
1.1 → 1.3 → 2.1 → 3.1 → 4.1 → 4.2 → 5.1

### Quality Gates

Each major task must pass validation checkpoints before proceeding:
- Task 1: Error handling with context, structured logging works
- Task 2: HTML parsing and error handling work correctly
- Task 3: Memory usage ≤ 512MB
- Task 4: Summary output is accurate and complete
- Task 5: All integration tests pass
