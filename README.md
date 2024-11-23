# Switchbot Exporter

Prometheus のカスタムエクスポーターです。
Switchbot のステータスを取得します。

## 利用方法

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

> [!NOTE]
> 利用方法についてはまだ記載が終わっておりません

## 開発手順

開発環境は docker コンテナになります

1. docker image のビルドを行います

```sh
docker compose build
```

2. docker コンテナを立ち上げます

```sh
docker run --rm -it \
  -v $(pwd)/docker:/app \
  -p 9171:9171 \
  -e SWITCHBOT_API_TOKEN=test_api_token \
  -e SWITCHBOT_API_SECRET=test_api_secret \
  mizucopo/switchbot-exporter:develop \
  /bin/sh
```

3. ソースコードを編集します
4. テストの実行をします

```sh
poetry run pytest ./tests
```

5. フォーマットの確認をします

```sh
poetry run mypy --pretty ./src
```

```sh
poetry run ruff check ./src
```

```sh
poetry run black ./src ./tests
```

6. Github へプルリクエストを行います

## Contact

質問等は X まで ([@mizu_copo](https://twitter.com/mizu_copo)).

## License

This project is published under the MIT License. For more details, please refer to the [LICENSE file](/LICENSE).
