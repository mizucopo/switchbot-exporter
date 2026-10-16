# Exporter 仕様

## API と取得対象

SwitchBot API v1.1 の GET endpoint を使う。認証には HMAC-SHA256 / Base64 署名、millisecond timestamp、空 nonce を使用する。

`devices` はすべての physical `deviceList` を JSON で返す。メトリクス取得時は `Bot`、`Ceiling Light`、`Color Bulb`、`Contact Sensor`、`Curtain`、`Hub Mini`、`Indoor Cam`、`Meter`、`MeterPro(CO2)`、`Motion Sensor`、`Plug Mini (JP)`、`Remote` を対象にする。infrared remote list は対象外とする。`deviceType` の欠落は `-` として一覧に残し、メトリクス取得対象から除く。

成功した API GET の後に `DELAY_SECOND` だけ待機する。URL ごとに成功 response をメモリへ保存し、TTL 内の cache hit では API 呼び出しも待機も行わない。失敗 response は保存せず、次の取得時に再試行する。期限切れデータへの fallback と部分成功は行わない。

## HTTP とメトリクス

`/metrics` は `text/plain; charset=utf-8` を返し、GET / HEAD / OPTIONS に対応する。取得失敗は HTTP 500、未知の path は 404、未対応 method は 405 とする。API の `statusCode` が存在して100以外の場合、不正 JSON、数値でない metric も取得失敗とする。

数値の単位・scale は API の値をそのまま使い、欠落 metric は省略する。sample がなくても全 family の HELP / TYPE を出力する。label は `device_id` / `device_name`、すべて gauge とする。

| API field | Prometheus metric | HELP |
| --- | --- | --- |
| `battery` | `switchbot_device_battery` | SwitchBot Battery level |
| `humidity` | `switchbot_device_humidity` | SwitchBot Humidity |
| `temperature` | `switchbot_device_temperature` | SwitchBot Temperature |
| `CO2` | `switchbot_device_co2` | SwitchBot CO2 |
| `voltage` | `switchbot_device_voltage` | SwitchBot Voltage |
| `weight` | `switchbot_device_weight` | SwitchBot Weight |
| `electricCurrent` | `switchbot_device_electric_current` | SwitchBot ElectricCurrent |

family は表の順序、sample はデバイス一覧の順序で出力する。重複 ID は最初の出現順序を保ち、値と名前は最後の定義を使う。label 内の引用符・バックスラッシュ・改行は Prometheus escape に変換する。HTTP 本文に末尾改行を付けず、`metrics` CLI には改行を付ける。浮動小数点数は Rust JSON serializer の文字列表記を使用する。

## 設定

設定一覧と既定値は [README](../README.md#利用方法) を参照する。環境変数が `.env` より優先される。`.env` は作業 directory から親へ探索し、最初に見つかった一つだけを読み込む。引用符の除去と値の前後の空白除去を行い、重複キーは最後の定義を採用する。`export` prefix、inline comment、変数・escape 展開は行わない。

proxy はプロセス環境の `HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY` / `NO_PROXY`（小文字表記も可）を使う。独自 CA は `REQUESTS_CA_BUNDLE`、未設定または空なら `CURL_CA_BUNDLE` の PEM bundle ファイルを使用し、証明書・hostname を検証する。不正・空・読込不能な bundle は取得失敗とする。
