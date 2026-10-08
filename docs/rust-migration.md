# Python 版から Rust 版への移行

Issue #60 の最新依頼に従い、比較試作ではなく Python runtime を Rust へ置き換えます。性能計測は行っていないため、メモリ量・速度・起動時間・image size の改善を実証した変更ではありません。

## 維持する契約

- `/metrics` は `text/plain; charset=utf-8`、GET / HEAD / OPTIONS に対応します。失敗時は HTTP 500、未知の path は 404、未対応 method は 405 です。
- token / secret と既存の環境変数名・既定値、環境変数優先の `.env` 設定を維持します。空の `CACHE_DIR`、0 以下の TTL は cache 無効です。
- SwitchBot API v1.1 の GET endpoint、HMAC-SHA256 / Base64 署名、millisecond timestamp、空 nonce、デバイス一覧順を維持します。
- 一覧 CLI はすべての physical `deviceList` を返し、metrics 取得時だけ下記機種を選別します。infrared remote list は取得対象外です。
- 各 URL ごとの TTL、cache hit 時の待機省略、成功した実リクエスト後の `DELAY_SECOND` を維持します。retry、期限切れデータへの fallback、部分成功は行いません。
- 数値の単位・scale は変更せず、欠落 metric は省略します。sample がなくても全 family の HELP / TYPE を出力します。label は `device_id` / `device_name`、すべて gauge です。

対応機種は `Bot`、`Ceiling Light`、`Color Bulb`、`Contact Sensor`、`Curtain`、`Hub Mini`、`Indoor Cam`、`Meter`、`MeterPro(CO2)`、`Motion Sensor`、`Plug Mini (JP)`、`Remote` です。`deviceType` 欠落時は `-` として一覧に残し、metrics 取得対象から除きます。

| API field | Prometheus metric | HELP |
| --- | --- | --- |
| `battery` | `switchbot_device_battery` | SwitchBot Battery level |
| `humidity` | `switchbot_device_humidity` | SwitchBot Humidity |
| `temperature` | `switchbot_device_temperature` | SwitchBot Temperature |
| `CO2` | `switchbot_device_co2` | SwitchBot CO2 |
| `voltage` | `switchbot_device_voltage` | SwitchBot Voltage |
| `weight` | `switchbot_device_weight` | SwitchBot Weight |
| `electricCurrent` | `switchbot_device_electric_current` | SwitchBot ElectricCurrent |

family と sample の順序も維持します。HTTP 本文に末尾改行を付けず、metrics CLI には改行を付けます。浮動小数点数の文字列表記は Rust JSON serializer に従いますが、Prometheus の数値としての意味は同じです。

## 利用者の変更事項

1. Python / Flask / Gunicorn の起動を `switchbot-exporter exporter` へ変更します。Python module import、`python src/app.py`、`app:app`、Gunicorn worker 設定は利用できません。Docker の既定コマンドは新 binary を起動します。
2. cache はディスクから共有メモリへ変わります。既存 cache file や volume は読まず、再起動や別プロセスとの cache 共有も行いません。`CACHE_DIR` の非空値は互換用の有効化指定として受け付け、directory は作成しません。
3. `.env` は実行時の working directory を起点に探索します。binary の配置場所に関係なく、設定のある directory で実行してください。`settings.ini` の読込は終了します。使用していた場合は `.env` または環境変数へ移してください。旧 `.env` と同じく `export` prefix、inline comment、変数・escape 展開は行いません。
4. `devices` / `device-status DEVICE_ID` の名前は維持しますが、出力を Python repr から JSON へ変更します。出力解析を行う script は JSON 対応にしてください。
5. 新しい `API_TIMEOUT_SECOND` は既定 30 秒です。従来の無期限 API GET と Gunicorn の 180 秒 worker timeout から変わるため、必要なら正の秒数を設定してください。timeout / HTTP error / 不正JSON は次の取得で再試行できます。
6. API の `statusCode` が存在して 100 以外の場合は取得失敗とします。不正JSONや失敗 statusCode は cache に保存しません。metric が数値でない場合も HTTP 500 にします。
7. label 内の引用符・バックスラッシュ・改行を正しい Prometheus escape に変換します。従来の引用符だけの変換で不正だった名前は有効な label として出力されます。
8. `SERVER_PORT` は Docker の既定起動にも適用します。port を変更した場合は host 側の port mapping と Prometheus の scrape target も合わせて変更してください。

採番は `release:major` 分類を持つ PR のマージ後に既存 Actions が行います。移植 PR では現在の version `2.0.21` を `Cargo.toml` と `Cargo.lock` に移し、手動で次の番号を設定しません。マージ・製品公開・デプロイはこの実装作業には含みません。
