# OpenSpec インストラクション（日本語要約）

詳しい手順は `openspec/AGENTS.md` を参照してください。ここでは Switchbot Exporter プロジェクト向けの重要ポイントのみを日本語で整理します。

## 変更提案の基本フロー
- 新機能や破壊的変更を行うときは必ず `openspec/changes/<change-id>/` に `proposal.md`・`tasks.md`・必要なら `design.md` と差分スペックを用意する。
- `openspec validate <change-id> --strict` を実行し、すべてのファイルがフォーマット要件を満たしていることを確認する。
- 提案が承認されるまで実装に着手しない。

## 実装時のチェックリスト
1. `proposal.md`・`design.md`（あれば）・`tasks.md` を順番に読む。
2. タスクに沿って作業し、完了した項目は `- [x]` でマークする。
3. すべて完了したら差分スペックを本体仕様に反映し、必要に応じてアーカイブする。

## プロジェクト固有メモ
- ローカル開発は `docker/` ディレクトリで uv 管理の仮想環境を作成する（`uv venv .venv` → `source .venv/bin/activate`）。
- `uv.lock` と `uv.dev.lock` を `uv pip sync` で同期し、`pytest tests` / `mypy --pretty src` / `ruff check src` / `black src tests` を uv 環境内で実行する。
- Docker イメージは配布のためのフォーマットで、`docker-compose.yml` には配布サービスのみが含まれている（リリース検証時に利用する）。
