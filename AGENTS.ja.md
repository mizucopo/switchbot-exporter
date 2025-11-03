# Repository Guidelines（日本語）

## プロジェクト構成とモジュール配置
- `docker/` はアプリ本体・uv 設定・Dockerfile を含む作業ルートです。
- `docker/src/` に Flask エクスポーター（`app.py`）と SwitchBot クライアント（`switchbot.py`）があります。新しいモジュールは同階層に追加してください。
- `docker/tests/` は pytest スイートを格納し、ソース構成と同じディレクトリ構成で `test_*.py` を配置します。
- ルートの `docker-compose.yml` と `secrets.example` はコンテナ開発環境の起動手順と秘密情報の雛形を示します。

## ビルド・テスト・開発コマンド
- `docker compose build` — `Dockerfile.dev` をもとに開発用イメージを構築します。
- `docker run --rm -it -v $(pwd)/docker:/app ... mizucopo/switchbot-exporter:develop /bin/sh` — 開発コンテナ内でシェルを起動します（完全なコマンドは `README.md` を参照）。
- `uv pip install --system --no-deps -r uv.lock -r uv.dev.lock` — コンテナ外で作業する際にアプリと開発用依存関係をインストールします。
- uv CLI は Dockerfile で PyPI の最新安定版を取得します。バージョン固定が必要になった場合は Dockerfile を更新してください。
- `pytest` — `docker/tests` 配下のユニットテストを実行します。
- `ruff check ./src` / `black ./src ./tests` — Python コードの lint/format を行います。
- `act -j build-and-push` — PR 作成前に GitHub Actions のビルドワークフローをローカルで検証します。

## コーディング規約と命名
- ターゲットは Python 3.13 で型チェックは厳格設定です。公開関数には型注釈を付与し、`mypy --pretty ./src` を通過させてください。
- Black は 88 文字幅を強制し、Ruff が import 並び替えや docstring ルールを検証します。自動修正を活用してください。
- モジュール/関数は `snake_case`、クラスは `CamelCase`、Prometheus メトリクスは `switchbot_device_battery_percent` のような命名を採用します。

## テスト方針
- Pytest は `test_*.py` を検出します。ソースと同じ構造でテストファイルを追加し、機能追加時は対応するテストを作成してください。
- SwitchBot API への外部通信はスタブ化し、テストを決定的に保ちます。
- コミット前に `pytest` を必ず実行し、メトリクス露出に影響が出る変更ではカバレッジや確認結果を PR に記載します。

## コミットとプルリクエスト運用
- Git 履歴にならい Conventional Commits（例: `fix:`, `docs:`, `chore:`）で要約します。
- 変更範囲は最小限に保ち、複数コンポーネントに跨る場合は本文で詳細を補足してください。
- PR では変更概要、実行した検証コマンド、関連 Issue を記載し、メトリクスやエンドポイントの出力が変わる場合はスクリーンショットなども添付するとレビューしやすくなります。

## セキュリティと設定の注意
- ローカル開発では `secrets.example` を `.secrets` にコピーし、実際の認証情報はコミットしないでください。
- 必須環境変数は `SWITCHBOT_API_TOKEN` と `SWITCHBOT_API_SECRET`、任意設定は `CACHE_DIR`・`CACHE_EXPIRE_SECOND`・`DELAY_SECOND` で、詳細は `README.md` に記載があります。
- Docker 実行時はマウントする `CACHE_DIR` に書き込み権限があることを確認し、エクスポーターのクラッシュを防ぎます。
