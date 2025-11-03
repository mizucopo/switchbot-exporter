## 1. 依存関係の更新
- [x] 1.1 `docker/pyproject.toml` に `python-decouple` を追加し、バージョン指定と互換性を確認する。
- [x] 1.2 `uv` を用いて `uv.lock` と `uv.dev.lock` を再生成し、依存関係の整合性を確認する。

## 2. アプリケーション実装
- [x] 2.1 `docker/src/app.py` の環境変数読み取りを `python-decouple` の `config` API へ置き換え、必須値と任意値のキャストを整理する。
- [x] 2.2 `.env` ファイル読み込みを考慮した設定ヘルパーを追加し、CLI と Flask サーバーの両方で共有する。
- [x] 2.3 既存テストもしくは新規テストで設定ロジックをカバーし、必須変数不足時の挙動を検証する。

## 3. ドキュメントと設定例
- [x] 3.1 `AGENTS.md` と `README.md` に `python-decouple` を使った環境変数セットアップ手順を追記し、`.env` 利用手順を明示する。
- [x] 3.2 `env.example` を `.env` 互換フォーマットとして最新化し、不要な項目があれば整理する。
- [x] 3.3 既存の `.secrets` ファイルを `.env` にリネームし、参照箇所をすべて更新する。
- [x] 3.4 `secrets.example` を `env.example` にリネームし、参照箇所をすべて更新する。
- [x] 3.5 `.env` と `env.example` がシークレットではなく環境変数であることを明確にするドキュメント更新。

## 4. 検証
- [x] 4.1 `pytest`, `mypy --pretty src`, `ruff check src`, `black --check src tests` を実行して型・ lint・フォーマット検証を通す。
- [x] 4.2 `openspec validate adopt-python-decouple-config --strict` を実行し、提案内容と差分スペックが整合していることを確認する。
