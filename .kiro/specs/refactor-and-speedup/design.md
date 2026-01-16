# 技術設計書

---
**目的**: 実装者間での解釈のズレを防ぎ、一貫した実装を保証するのに十分な詳細を提供する。
---

## 概要
このフィーチャーは、`rs-net-radio`プロジェクトの既存コードベースをリファクタリングし、パフォーマンス、テスト容易性、および保守性を向上させることを目的とします。主なターゲットユーザーは、本アプリケーションの開発者と運用者です。この変更により、システムの応答性が改善され、将来の機能追加が容易になります。

### ゴール
- ブロッキングI/Oを排除し、アプリケーションの応答性を向上させる。
- レイヤー化アーキテクチャを導入し、関心の分離を実現する。
- 依存性の注入（DI）とトレイト（インターフェース）ベースの設計により、テスト容易性を高める。
- `tracing`, `thiserror`, `anyhow` を導入し、ロギングとエラーハンドリングを標準化する。

### 非ゴール
- 新機能の追加。
- 公開APIの変更（ライブラリの利用者への影響はない）。
- データベーススキーマやCI/CD設定の変更。

## アーキテクチャ

### 既存アーキテクチャの分析
現在のシステムは、スケジューリングを行う`app`クレートと、録音ロジックを持つ`record-lib`クレートで構成されています。しかし、`record-lib`内では、HTTPリクエスト、外部コマンド実行、ビジネスロジックが密結合しており、テストと保守が困難です。また、`reqwest::blocking`と`std::process::Command`によるブロッキングI/Oがパフォーマンス上の大きなボトルネックとなっています。（詳細は`research.md`を参照）

### アーキテクチャパターンと境界マップ
本リファクタリングでは、クリーンアーキテクチャの原則に基づいた**レイヤー化アーキテクチャ**を採用します。これにより、ビジネスロジックをインフラストラクチャから完全に分離します。

```mermaid
graph TD
    subgraph App [Application Layer]
        A1[Use Cases (例: RecordProgramUseCase)]
    end

    subgraph Domain [Domain Layer]
        D1[Entities (Program, Episode)]
        D2[Repository Traits (ProgramRepo, RecordingRepo)]
        D3[Service Traits (RadioApi, VideoProcessor)]
    end

    subgraph Infra [Infrastructure Layer]
        I1[Web API Clients (ReqwestRadioApi)]
        I2[External Commands (TokioFfmpegVideoProcessor)]
        I3[Persistence (FileSystemRepo)]
    end
    
    subgraph Main [Composition Root]
        M1[main.rs / Scheduler]
    end

    M1 --> A1
    A1 --> D2
    A1 --> D3
    I1 --> D3
    I2 --> D3
    I3 --> D2
```

- **選択したパターン**: レイヤー化アーキテクチャ。ビジネスの中核（Domain）を、外部とのやり取り（Infra）から隔離します。
- **境界**:
    - **Domain**: アプリケーションのビジネスルールとエンティティを定義します。他のレイヤーに依存しません。
    - **Application**: ユースケースを定義し、Domainのオブジェクトを操作してタスクを実行します。
    - **Infrastructure**: 外部システム（Web API, ファイルシステム, 外部コマンド）との通信を実装します。Domainで定義されたトレイトを実装します。
    - **Main**: `main.rs`がDIコンテナの役割を果たし、具体的なInfraの実装をApplicationのユースケースに注入します。

### 技術スタック

| レイヤー | 選択 / バージョン | 役割 | 備考 |
|---|---|---|---|
| バックエンド / サービス | Rust | コア言語 | |
| 非同期ランタイム | tokio | 非同期処理の実行 | 既存のランタイムを適切に利用する |
| HTTPクライアント | reqwest (async) | 外部APIとの非同期通信 | `blocking`から非同期版へ移行 |
| ロギング | tracing | 構造化ロギング | `log`から移行 |
| エラーハンドリング | thiserror, anyhow | 型付けされたエラーと柔軟なエラー伝搬 | 新規導入 |

## コンポーネントとインターフェース

### Domain Layer

#### `Program`, `Episode` (Entities)
- **責務**: ラジオ番組やそのエピソードに関するビジネスデータを保持する。
- **インターフェース**:
```rust
pub struct Program {
    pub id: String,
    pub name: String,
    // ... 他のビジネス関連フィールド
}

pub struct Episode {
    pub id: String,
    pub title: String,
    pub download_url: String,
}
```

#### `ProgramRepository` (Trait)
- **責務**: `Program`エンティティの永続化を抽象化する。
- **インターフェース**:
```rust
#[async_trait]
pub trait ProgramRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<Program>, RepositoryError>;
    async fn save(&self, program: &Program) -> Result<(), RepositoryError>;
}
```

#### `RadioApi` (Trait)
- **責務**: ラジオ配信サービスのAPIとの通信を抽象化する。
- **インターフェース**:
```rust
#[async_trait]
pub trait RadioApi {
    async fn fetch_latest_episodes(&self, program_id: &str) -> Result<Vec<Episode>, ApiError>;
}
```

#### `VideoProcessor` (Trait)
- **責務**: `ffmpeg`などの外部コマンドによる動画・音声処理を抽象化する。
- **インターフェース**:
```rust
#[async_trait]
pub trait VideoProcessor {
    async fn record_episode(&self, episode: &Episode, output_path: &Path) -> Result<(), ProcessorError>;
}
```

### Application Layer

#### `RecordProgramUseCase`
- **責務**: 特定のプログラムの最新エピソードを録音する、というユースケースを実現する。
- **インターフェース**:
```rust
pub struct RecordProgramUseCase<A: RadioApi, P: VideoProcessor, R: ProgramRepository> {
    api: A,
    processor: P,
    repo: R,
}

impl<A, P, R> RecordProgramUseCase<A, P, R>
where
    A: RadioApi + Sync + Send,
    P: VideoProcessor + Sync + Send,
    R: ProgramRepository + Sync + Send,
{
    pub async fn execute(&self, program_id: &str) -> Result<(), anyhow::Error> {
        // 1. APIから最新エピソードを取得
        // 2. リポジトリで既存の状態と比較
        // 3. 必要ならVideoProcessorで録音
        // 4. リポジトリに状態を保存
        Ok(())
    }
}
```

### Infrastructure Layer

#### `ReqwestRadioApi`
- **責務**: `RadioApi`トレイトを`reqwest`を使用して実装する。
- `reqwest`の非同期クライアントを使用し、ブロッキングしない。

#### `TokioFfmpegVideoProcessor`
- **責務**: `VideoProcessor`トレイトを`ffmpeg`を使用して実装する。
- `tokio::process::Command`を使用して`ffmpeg`を非同期に実行するか、CPUバウンドな処理が支配的な場合は`tokio::task::spawn_blocking`内で同期的な`std::process::Command`を実行する。

## エラーハンドリング
- **Domain, Infra Layer**: `thiserror`を使用し、具体的なエラー型（例: `ApiError`, `ProcessorError`）を定義する。
- **Application, Main**: `anyhow::Error`を使用し、アプリケーション境界でのエラー集約とコンテキスト付与を行う。

## テスト戦略
- **単体テスト**: ドメインロジックとユースケースをテストする。リポジトリやサービスのトレイトはモック実装に差し替える。
- **統合テスト**: Infrastructure層のコンポーネントが、実際の外部システム（のテストダブル）と正しく連携できるかを確認する。

## 移行戦略
`research.md`に記載の通り、ハイブリッドアプローチを採用する。
1. **フェーズ1 (パフォーマンス改善)**: まず、既存のコードベース内のブロッキング呼び出し（`reqwest`, `Command`）を、`tokio::task::spawn_blocking`でラップするか非同期APIに置き換えることで、緊急のパフォーマンス問題を解決する。
2. **フェーズ2 (構造的リファクタリング)**: 上記で定義したレイヤー化アーキテクチャを導入する。まず`hibiki`サービスから始め、ドメインのトレイトを定義し、ロジックをユースケースに、インフラ実装をInfraレイヤーに分離する。
3. **フェーズ3 (完全移行)**: 残りのサービス（`onsen`, `radiko`）も新しいアーキテクチャに移行し、最終的にプロジェクト全体で構造を統一する。
