## ADDED Requirements
### Requirement: python-decouple Driven Configuration
The exporter MUST read runtime configuration through `python-decouple`, providing unified casting, defaults, and optional `.env` support across the CLI and HTTP server entry points.

#### Scenario: Load required SwitchBot credentials
- **WHEN** the exporter starts and needs `SWITCHBOT_API_TOKEN` と `SWITCHBOT_API_SECRET` を読み込む
- **THEN** `python-decouple` の `config` ヘルパーを用いて値を取得し
- **AND** 値が未設定の場合は `UndefinedValueError` もしくは同等の分かりやすい例外で起動を停止する。

#### Scenario: Apply typed defaults for optional settings
- **WHEN** `SERVER_PORT`, `CACHE_EXPIRE_SECOND`, `DELAY_SECOND` などの任意設定を評価する
- **THEN** `python-decouple` のキャスト機能を利用して型安全に値を読み込み
- **AND** `.env` または環境変数に値が無い場合でも既定値を適用して起動できる。

#### Scenario: Support local .env workflows
- **WHEN** コントリビューターがリポジトリルートに `.env` や `.secrets` を配置してローカル実行する
- **THEN** `python-decouple` がファイルを検知して設定を読み込み
- **AND** CLI コマンドと Flask アプリの両方で同じ設定が共有される。
