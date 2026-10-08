# Switchbot Exporter

SwitchBot API のステータスを Prometheus に公開する Rust 製 exporter です。
`GET /metrics` で battery、humidity、temperature、CO2、voltage、weight、electric current を取得できます。

## 利用方法

配布形態は引き続き Docker image です。Rust 版への移行前に [移行ガイド](docs/rust-migration.md) を確認してください。

```sh
docker pull mizucopo/switchbot-exporter:latest
docker run --rm -d \
  -p 9171:9171 \
  --env-file .env \
  mizucopo/switchbot-exporter:latest
```

`env.example` を `.env` にコピーして認証情報を設定します。環境変数が `.env` の値より優先されます。
実行時の working directory から親へ探索し、最初に見つかった `.env` を読み込みます。
Docker では `--env-file` を使うか、`/app/.env` にファイルを mount します。

| 環境変数 | 必須 | 既定値 | 役割 |
| --- | --- | --- | --- |
| `SWITCHBOT_API_TOKEN` | ◯ | — | SwitchBot API token |
| `SWITCHBOT_API_SECRET` | ◯ | — | SwitchBot API secret |
| `SERVER_PORT` | | `9171` | `0.0.0.0` で listen する port。Docker でも適用 |
| `CACHE_DIR` | | `/tmp/switchbot` | 旧設定との互換用。空文字なら cache 無効。ディスクには書き込まない |
| `CACHE_EXPIRE_SECOND` | | `600` | URL ごとの共有メモリ cache TTL。0 以下なら cache 無効 |
| `DELAY_SECOND` | | `1` | 成功した API GET の後の待機秒数。小数・0 を指定可能 |
| `API_TIMEOUT_SECOND` | | `30` | API GET の送信から本文受信までの timeout 秒数。正の有限値 |
| `REQUESTS_CA_BUNDLE` | | 未設定 | HTTPS 検証に使う PEM CA bundle ファイル。指定時はこの bundle のみを信頼 |
| `CURL_CA_BUNDLE` | | 未設定 | `REQUESTS_CA_BUNDLE` が未設定または空のときの CA bundle |

proxy は `HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY` / `NO_PROXY`（小文字表記も可）をプロセス環境から読み込みます。Docker の `--env-file` や shell の `export` で渡してください。独自 CA を使う場合は PEM bundle を container 内へ mount し、そのパスを指定します。CA directory を使用していた場合は [移行ガイド](docs/rust-migration.md) を確認してください。

認証情報は最初の取得時に読み込みます。欠落や不正設定、API エラー、JSON エラーは `/metrics` の HTTP 500 として返し、部分的なメトリクスは返しません。認証なしでも exporter の起動・help・version 確認はできます。
cache は単一プロセス内で共有され、同時 scrape の API 取得は直列化されます。cache hit では待機も API 呼び出しも行いません。再起動時には cache が空になります。

## ローカル実行

Rust 1.94 以上を利用します。標準 toolchain は `rust-toolchain.toml` に記載しています。

```sh
cargo build --release --locked
./target/release/switchbot-exporter exporter
./target/release/switchbot-exporter devices
./target/release/switchbot-exporter device-status DEVICE_ID
./target/release/switchbot-exporter metrics
```

`devices` と `device-status` は JSON、`metrics` は Prometheus text を標準出力へ返します。
`exporter` は SIGINT / SIGTERM で新規接続の受付を止め、処理中のリクエストを終了して停止します。

## 開発と検証

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo build --release --locked
```

Rust テストはローカル HTTP mock と仮想時計で API 認証・取得・TTL・待機・エラー・HTTP route を確認します。実機 SwitchBot の操作や性能計測は行いません。
採番 controller の既存回帰テストには Python 3.14 と uv を使用します。これは開発・CI 用で、配布 image に Python は含みません。

```sh
uv sync --locked
uv run task check
```

Docker は `linux/amd64` と `linux/arm64` に対応します。PR CI は両方の native runner で image を build し、外部ネットワークを無効にした smoke を実行します。

```sh
docker build --check .
docker build -t switchbot-exporter:develop .
docker run --rm --network none switchbot-exporter:develop switchbot-exporter --version
bash tests/docker_smoke.sh switchbot-exporter:develop
```

リリース分類は [CONTRIBUTING.md](CONTRIBUTING.md)、採番・公開は [docs/release.md](docs/release.md)、実装判断は [ADR](docs/adr/0001-rust-exporter.md) を参照してください。

## Contact

質問等は X ([@mizu_copo](https://twitter.com/mizu_copo)) まで。

## License

MIT。詳細は [LICENSE](LICENSE) を参照してください。
