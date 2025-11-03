# プロジェクトコンテキスト

## 目的
SwitchBot デバイスの状態を Prometheus メトリクスとして公開し、ホームオートメーションや監視スタックがデバイスのヘルスや環境データを追跡できるようにする。

## 技術スタック
- HTTP エクスポーターとして Flask を用いた Python 3.13。
- uv で管理する依存関係（`pyproject.toml` + `uv.lock`）で `requests`、`prometheus_client`、`inflect/inflection`、`click` および開発ツール群をカバーする。
- 既存環境を汚さないよう、配布は Docker イメージを主経路とする。
- Prometheus サーバーがエクスポーターのエンドポイントをスクレイプする。

## プロジェクト規約

### コードスタイル
- マージ前に Black（88 文字幅）、Ruff、strict Mypy を必ず通し、モジュールには型注釈を整える。
- モジュール配置はリポジトリの慣例に従い（`docker/src` と `docker/tests` を対応させる）、命名は snake_case を守る。
- Prometheus のメトリクス名は説明的で安定させる（`switchbot_device_*` 形式）。

### アーキテクチャパターン
- `docker/src/app.py` が Flask エクスポーターを提供し、`docker/src/switchbot.py` が SwitchBot 公開 API をラップする。
- ローカル開発は uv で管理された Python 仮想環境上で行い、Docker イメージは配布や最終検証のためのオプションとして維持する。
- キャッシュや遅延の制御は SwitchBot クライアントの近くに集約し、API 利用を一元化する。

### テスト戦略
- `docker/tests` 配下で pytest によりユニットテストを行い、SwitchBot への外部呼び出しはスタブ化する。
- マージ前のチェックとして `mypy --pretty ./src`、`ruff check ./src`、`black ./src ./tests` を（uv で依存関係を整えた上で）実行する。
- 現状のエンドツーエンド検証は自動化された統合テストではなく、ステージング環境での Prometheus スクレイプテストに依存している。

### Git ワークフロー
- コミット履歴の明瞭化のため Conventional Commits（`fix:`、`feat:` 等）に従う。
- 変更はフィーチャーブランチで行い、デフォルトブランチへ直接プッシュしない。
- 可能な範囲で PR 前に GitHub Actions のワークフロー（`act -j build-and-push`）で検証する。

## ドメインコンテキスト
- SwitchBot のデバイス群（ボット、センサー、加湿器など）を対象とし、公式 API を通じて状態を取得する。
- メトリクスは可用性（オンライン/オフライン）、バッテリー残量、環境データ（温度、湿度、照度）を優先する。
- `SWITCHBOT_API_TOKEN` と `SWITCHBOT_API_SECRET` などのシークレットはバージョン管理外に置き、サンプル設定は `secrets.example` に提供する。

## 重要な制約
- SwitchBot API にはレート制限があるため、デフォルトの遅延（`DELAY_SECOND`）やキャッシュ有効期限（`CACHE_EXPIRE_SECOND`）は設定可能な状態で維持する。
- キャッシュを有効化する場合、エクスポーターは `CACHE_DIR` への書き込み権限を必要とするため、コンテナ内のマウント先は書き込み可能でなければならない。
- デプロイは Docker イメージを前提とし、ベアメタルへの直接インストールはスコープ外とする。

## 外部依存
- SwitchBot 公開 API（デバイス状態の取得用）
- Prometheus など `/metrics` エンドポイントをスクレイプできるシステム
- ビルドとプッシュの自動化を担う GitHub Actions ワークフロー
