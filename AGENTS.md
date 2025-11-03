<!-- OPENSPEC:START -->
# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:
- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:
- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

# OpenSpec インストラクション（日本語）

このプロジェクトで仕様駆動開発を進める AI コーディングアシスタントのためのガイドです。

- 計画や変更提案に関する依頼を受けたら、必ず `@/openspec/AGENTS.md` を開いて内容を確認してください。
- 新しいケイパビリティ、破壊的変更、アーキテクチャの転換、大きな性能・セキュリティ改善を扱う際も本ファイルを参照してください。
- 実装開始前に仕様フォーマット、命名規約、プロジェクト構成を把握するための基礎資料として利用します。

# リポジトリガイドライン

## プロジェクト構成とモジュール配置
- `docker/` はアプリケーションコード、uv のメタデータ、Dockerfile を保持する作業用ルートディレクトリです。
- `docker/src/` には Flask エクスポーター（`app.py`）と SwitchBot クライアント（`switchbot.py`）があり、新しいモジュールはこの階層に追加します。
- `docker/tests/` はソース構成と同じディレクトリ構造で pytest スイートを配置します。機能を追加する際は対応する `test_*.py` を作成してください。
- ルートの `docker-compose.yml` は本番配布向けサービスのみを保持します。コンテナー配布の要件に合わせて同期し、`secrets.example` をローカルシークレットのテンプレートとして利用します。

## ビルド・テスト・開発コマンド
- `uv venv .venv`（`docker/` ディレクトリ内で実行）— uv 管理のローカル仮想環境を作成または更新します。
- `source .venv/bin/activate` と `uv pip sync uv.lock uv.dev.lock` — ピン留めされた運用およびツール依存関係をインストールします。
- `pytest tests`、`mypy --pretty src`、`ruff check src`、`black src tests` — これらの検証コマンドは uv 環境内で直接実行します。
- `act -j build-and-push` — PR を作成する前に GitHub Actions ワークフローをローカルでリハーサルします。
- Docker イメージは本番配布フォーマットです。リリース準備や成果物確認時に `docker compose build prod` または `docker build -f Dockerfile.prod` を使用し、日常開発では不要です。

## コーディング規約と命名
- 目標の Python バージョンは 3.13 で、型チェックは厳格設定です。公開関数には型注釈を付与し、`mypy --pretty ./src` を通過させてください。
- Black は 88 文字幅を強制し、Ruff が import の順序や docstring ルールを取り扱います。自動修正を積極的に活用してください。
- モジュールと関数は `snake_case`、クラスは `CamelCase`、Prometheus メトリクスは `switchbot_device_battery_percent` のような命名を採用します。

## テストガイドライン
- Pytest は `test_*.py` ファイルを検出します。ソース構成に合わせた場所へテストファイルを配置し、例として `src/switchbot.py` に対しては `tests/test_switchbot.py` を用意します。
- SwitchBot API への外部 HTTP 呼び出しはスタブ化し、テスト実行を決定的に保ちます。
- コミット前に必ず `pytest` を実行し、メトリクス公開に影響し得る変更ではカバレッジや確認結果を共有してください。

## コミットとプルリクエストのガイドライン
- Git 履歴に従い、`fix:`、`docs:`、`chore:` などの Conventional Commits 形式で要約します。
- コミットはスコープを絞り、複数コンポーネントにまたがる場合は本文で詳細を補足します。
- PR では変更概要、実行した検証コマンド、関連 Issue を記載し、メトリクスやエンドポイント出力が変わる場合はスクリーンショットなども添付するとレビューが容易になります。

## セキュリティと設定の注意事項
- ローカル開発では `secrets.example` を `.secrets` にコピーし、実際の認証情報は絶対にコミットしません。
- 必須環境変数は `SWITCHBOT_API_TOKEN` と `SWITCHBOT_API_SECRET` です。任意のキャッシュ設定（`CACHE_DIR`、`CACHE_EXPIRE_SECOND`、`DELAY_SECOND`）については `README.md` を参照してください。
- Docker を利用する際は、マウントする `CACHE_DIR` に書き込み権限があることを確認してエクスポーターのクラッシュを防ぎます。
