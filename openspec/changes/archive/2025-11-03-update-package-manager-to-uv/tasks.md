## 1. 調査
- [x] 1.1 Dockerfile、README、CI ワークフロー、スクリプト内の Poetry 参照箇所をすべて洗い出す。
- [x] 1.2 サポートする uv のリリースチャンネル（固定バージョンか最新か）を決め、インストール手順を記録する（PyPI 最新安定版を採用し、ドキュメントに明記）。

## 2. ツールチェーン移行
- [x] 2.1 `pyproject.toml` から `[tool.poetry]` 設定を削除し、uv で扱える PEP 621 メタデータへ移行する。
- [x] 2.2 `uv.lock`（配布用）と `uv.dev.lock`（開発ツール用）を生成し、`poetry.lock` をバージョン管理から外す。
- [x] 2.3 開発ツール類の `poetry run`/`poetry install` 呼び出しを、uv で依存を整えて直接コマンドを実行する手順へ置き換える。

## 3. ビルドと CI の更新
- [x] 3.1 `Dockerfile.dev` と `Dockerfile.prod` に uv のインストールを追加し、ビルド時の依存解決に利用する。
- [x] 3.2 GitHub Actions（およびローカルの `act` 実行）を uv インストール＋ uv による lint/test 実行に更新する（Docker ビルドのみのワークフローにつき追加変更は不要）。
- [x] 3.3 コンテナのエントリポイントやスクリプトが Poetry 管理の仮想環境を前提にしないことを確認する。

## 4. ドキュメントと検証
- [x] 4.1 `README.md`、`docker/README`（存在する場合）、その他のコントリビューター向け資料を uv での依存インストール手順に合わせて刷新する。
- [x] 4.2 `openspec/project.md`、`openspec/AGENTS.md`、`openspec/AGENTS.ja.md` に uv を標準ツールチェーンとして反映する。
- [x] 4.3 `pytest ./tests`、`mypy --pretty ./src`、`ruff check ./src`、`black ./src ./tests` を実行し、従来ワークフローとの同等性を確認する。
