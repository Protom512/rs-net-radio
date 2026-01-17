# MCP Server Setup Guide

## 概要

rs-net-radio プロジェクトで使用する MCP (Model Context Protocol) サーバーの設定ガイドです。

## 現在有効な MCP サーバー

### 1. Context7
- **用途**: 高度なコンテキスト管理
- **設定**: `.claude/settings.local.json` で有効化
- **機能**: コードベースのインテリジェントなインデックス化

### 2. Serena
- **用途**: AI ペアプログラミング、シンボル検索
- **設定**: `.claude/settings.local.json` で有効化
- **機能**:
  - シンボル検索 (`find_symbol`)
  - 概要取得 (`get_symbols_overview`)
  - プロジェクトアクティベーション (`activate_project`)
  - ディレクトリ一覧 (`list_dir`)
  - パターン検索 (`search_for_pattern`)

### 3. GitHub MCP Server (VSCode)
- **用途**: GitHub 連携
- **設定**: `.vscode/mcp.json` で設定
- **機能**: PR 操作、Issue 管理

## 推奨される追加 MCP サーバー

### Rust 開発に有用なサーバー

1. **Filesystem MCP**
   - 高度なファイル操作
   - バッチ処理

2. **Git MCP**
   - Git 操作の自動化
   - 変更履歴の分析

## 権限設定

`.claude/settings.local.json` で以下のコマンドが許可されています:

```json
{
  "permissions": {
    "allow": [
      "Bash(cargo test:*)",
      "Bash(cargo build:*)",
      "Bash(cargo clippy:*)",
      "Bash(cargo fmt:*)",
      "Bash(cargo doc:*)",
      "Bash(cargo watch:*)",
      "Bash(cargo expand:*)",
      "Bash(cargo check:*)",
      "Bash(cargo clean:*)",
      "Bash(cargo install:*)",
      "Bash(git diff:*)",
      "Bash(git status:*)",
      "Bash(git log:*)",
      // ... MCP サーバー権限
    ]
  }
}
```

## MCP サーバーの管理

### サーバーを追加する場合

1. `.mcp.json` または `.vscode/mcp.json` を編集
2. `.claude/settings.local.json` の `enabledMcpjsonServers` に追加
3. 必要な権限を `permissions.allow` に追加

### サーバーを無効化する場合

`.claude/settings.local.json` から該当するサーバーを削除:

```json
{
  "enabledMcpjsonServers": [
    "context7",
    "serena"
    // 無効化したいサーバーを削除
  ]
}
```

## トラブルシューティング

### Serena が動作しない

```bash
# uvx がインストールされているか確認
uvx --version

# Serena を更新
uvx --from git+https://github.com/oraios/serena serena --version
```

### Context7 の API キー問題

VSCode の設定で CONTEXT7_API_KEY を確認してください。

## 使用例

### Serena でシンボルを検索

```
Serena で "RecordService" trait を検索してください
```

### Context7 でコードを理解

```
record-lib のエラーハンドリング構造を説明してください
```
