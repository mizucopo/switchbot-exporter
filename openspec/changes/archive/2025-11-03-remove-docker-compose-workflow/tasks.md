## 1. リポジトリ整理
- [x] 1.1 `docker-compose.yml` を削除し、Compose 固有の参照をツール類から取り除く。
- [x] 1.2 `docker/Dockerfile.dev` を削除し、参照がないか確認する。
- [x] 1.3 `docker/Dockerfile.prod` を `docker/Dockerfile` にリネームし、新しい単一ファイル構成に合わせて参照を更新する。

## 2. ドキュメント更新
- [x] 2.1 任意のコンテナ検証手順を直接 `docker build` / `docker run` に置き換えるよう `README.md` を更新する。
- [x] 2.2 `AGENTS.md` のガイダンスを修正し、Compose が不要になったことを明確に伝える。

## 3. 検証
- [x] 3.1 `docker build -f docker/Dockerfile ...` で本番イメージをビルドし、`docker run` で起動できることを確認する。
- [x] 3.2 `openspec validate remove-docker-compose-workflow --strict` を実行し、提案と整合していることを検証する。
