## 1. レイアウト移行
- [x] 1.1 `docker/` 配下の追跡対象ファイル・ディレクトリを棚卸しし、`.gitignore` に列挙されたパスを除外対象として確認する。
- [x] 1.2 残りのファイルとフォルダをリポジトリルートに移動し、既存の相対パスや import が崩れていないか確認する。
- [x] 1.3 旧 `docker/` ディレクトリを削除し、不要になった空ディレクトリや参照を整理する。

## 2. メタデータとドキュメント更新
- [x] 2.1 `pyproject.toml`、`uv.lock` などのプロジェクトメタデータの配置変更を反映し、パス依存のスクリプトや設定を更新する。
- [x] 2.2 `README.md`、`AGENTS.md`、および関連ドキュメントから `cd docker` 等の記述を取り除き、新しいルートレイアウト手順を記載する。
- [x] 2.3 CI や自動化スクリプトが新しいパスで動作することを確認し、必要に応じて更新する。

## 3. 検証
- [x] 3.1 `uv venv .venv` から `pytest`, `mypy --pretty src`, `ruff check src`, `black src tests` までの開発コマンドがルートディレクトリで問題なく実行できることを確認する。
- [x] 3.2 `docker build -f Dockerfile ...` と `docker run ...` がリポジトリルートをコンテキストとして成功することを確認する。
- [x] 3.3 `openspec validate flatten-project-root-structure --strict` を実行し、提案とスペック差分の整合性を保証する。
