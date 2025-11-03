## 1. 調査
- [x] 1.1 Docker 依存の開発手順（README、`openspec/project.md`、AGENTS、スクリプト、Makefile など）を洗い出し、代替が必要な箇所を一覧化する。
- [x] 1.2 uv 仮想環境で `pytest` / `mypy` / `ruff` / `black` が問題なく実行できるか、必要なシステム依存関係を確認する。

## 2. ローカル環境の切り替え
- [x] 2.1 uv 仮想環境のセットアップ手順（`uv venv`、`uv pip sync` など）と環境変数管理の方法を README に反映し、Docker ワークフローは補足扱いに整理する。
- [x] 2.2 Docker 開発用コンテナを前提としたスクリプトやコマンドを更新し、uv 仮想環境から直接実行できるようにする。
- [x] 2.3 `docker-compose.yml` から `dev` サービスを削除し、必要な代替手順をドキュメントに示す。

## 3. OpenSpec 更新
- [x] 3.1 `openspec/project.md` を更新し、ローカル開発の標準が uv 仮想環境であることを明記する。
- [x] 3.2 `openspec/AGENTS.md` と `openspec/AGENTS.ja.md` を更新し、提案に従ったワークフローの変更点を反映する。
- [x] 3.3 新しい `specs/dev-environment/spec.md` を追加し、uv 仮想環境の要件と検証シナリオを定義する。

## 4. 検証
- [x] 4.1 uv 仮想環境上で `pytest ./docker/tests`、`uvx mypy --pretty docker/src`、`uvx ruff check docker/src`、`uvx black --check docker/src docker/tests` を実行し、動作を確認する。
- [x] 4.2 ドキュメントとスクリプトの手順に従い、新規クリーン環境でのセットアップが成功することを実際に手順書に沿って検証する。
