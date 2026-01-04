# Switchbot Exporter

Prometheus のカスタムエクスポーターです。
Switchbot のステータスを取得します。

## 利用方法（配布）

Switchbot Exporter は Docker イメージとして配布されます。以下の手順でデプロイできます。

1. docker image を pull する

```sh
docker pull mizucopo/switchbot-exporter:latest
```

2. docker コンテナを立ち上げる

```sh
docker run --rm -d \
  -p 9171:9171 \
  -e SWITCHBOT_API_TOKEN=test_api_token \
  -e SWITCHBOT_API_SECRET=test_api_secret \
  mizucopo/switchbot-exporter:latest
```

| 環境変数名 | 必須有無 | 役割 |
|-:|:-:|-|
| SWITCHBOT_API_TOKEN | ◯ | Switchbot API認証に使用されるトークン |
| SWITCHBOT_API_SECRET | ◯ | Switchbot API認証に使用されるシークレット |
| SERVER_PORT |  | サーバーで利用するポート。デフォルトは 9171 |
| CACHE_DIR |  | キャッシュファイルを保存するディレクトリ。デフォルトは /tmp/switchbot |
| CACHE_EXPIRE_SECOND |  | キャッシュの有効期限（秒）。デフォルトは 600 |
| DELAY_SECOND |  | APIリクエスト間の遅延時間（秒）。デフォルトは 1 |

## 開発手順

Switchbot Exporter のローカル開発は uv が管理する Python 仮想環境を標準としています。Docker は配布検証やオプションの代替手段として利用できます。

### 前提ソフトウェア

- Python 3.13
- [uv CLI](https://docs.astral.sh/uv/)

### セットアップ

```sh
uv venv .venv
source .venv/bin/activate
uv pip sync uv.lock uv.dev.lock
```

必要な環境変数はリポジトリルートの `env.example` を `.env` にコピーして編集します。
python-decouple が自動的に `.env` ファイルを読み込むため、手動での環境変数読み込みは不要です。

```sh
cp env.example .env
# .env を編集して実際の値を設定
```

`.env` ファイルはプロジェクトルートに配置することで、自動的に読み込まれます。

### 開発コマンド

```sh
pytest tests
mypy --pretty src
ruff check src
black src tests
```

uv 仮想環境を利用していれば `poetry run` や Docker 特有のボリュームマウントは不要です。詳細は [AGENTS.md](./AGENTS.md) を参照してください。

### Docker を利用した動作確認（任意）

Docker での動作確認が必要な場合は `Dockerfile` を用いてコンテナをビルドできます。配布時と同じパッケージング形態で検証することを意図しています。

```sh
docker build -t mizucopo/switchbot-exporter:develop .
docker run --rm -d \
  -p 9171:9171 \
  -e SWITCHBOT_API_TOKEN=test_api_token \
  -e SWITCHBOT_API_SECRET=test_api_secret \
  mizucopo/switchbot-exporter:develop
```

起動後は `http://localhost:9171/metrics` にアクセスしてエクスポーターの挙動を確認できます。ローカル開発は uv 仮想環境で行い、Docker は配布および最終検証のために利用します。

## Contact

質問等は X まで ([@mizu_copo](https://twitter.com/mizu_copo)).

## License

This project is published under the MIT License. For more details, please refer to the [LICENSE file](/LICENSE).
