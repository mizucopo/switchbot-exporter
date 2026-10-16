# Exporter 仕様

## API と取得対象

SwitchBot API v1.1 の GET endpoint を使う。認証には HMAC-SHA256 / Base64 署名、millisecond timestamp、空 nonce を使用する。

`devices` はすべての physical `deviceList` を JSON で返す。メトリクス取得時は `Bot`、`Ceiling Light`、`Color Bulb`、`Contact Sensor`、`Curtain`、`Hub Mini`、`Indoor Cam`、`Meter`、`MeterPro(CO2)`、`Motion Sensor`、`Plug Mini (JP)`、`Remote` を対象にする。infrared remote list は対象外とする。`deviceType` の欠落は `-` として一覧に残し、メトリクス取得対象から除く。

成功した API GET の後に `DELAY_SECOND` だけ待機する。URL ごとに成功 response をメモリへ保存し、TTL 内の cache hit では API 呼び出しも待機も行わない。期限切れデータへの fallback は行わない。各 API request の timeout と取得順序は端末別の失敗時も同じとする。

## HTTP とメトリクス

`/metrics` は `text/plain; charset=utf-8` を返し、GET / HEAD / OPTIONS に対応する。未知の path は 404、未対応 method は 405 とする。

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

## 失敗時の応答

端末の status 取得または変換の失敗は、その端末全体の欠測として扱い、残りの対象端末を取得する。HTTP エラー（401/403を除く）、API の `statusCode` が存在して100以外、不正 JSON、`body` の欠落・null・object 以外、対象 metric の不正値が該当する。端末の全 metric を検証してから追加するため、一つでも不正値があれば、その端末の途中までの値や名前も反映しない。

| 状況 | HTTP | 公開内容 |
| --- | --- | --- |
| 一部端末だけ失敗 | 200 | 正常端末の sample と全 family の HELP / TYPE |
| 全対象端末が失敗 | 500 | 固定本文 `Internal Server Error` |
| 一覧の取得・形式検証に失敗 | 500 | 同じ固定本文。status 取得は行わない |
| 設定・client 初期化異常、いずれかの API の HTTP 401/403 | 500 | 同じ固定本文。部分結果も公開しない |
| 対象端末ゼロ（空一覧・対象外機種だけ） | 200 | 全 family の HELP / TYPE のみ |

正常な status object に対象 metric がない場合も正常取得として扱い、その端末の sample を省略する。`metrics` CLI も同じ取得方針を使い、部分成功・対象端末ゼロは終了コード0、全体失敗は非0とする。

失敗端末は0や期限切れの過去値で補完しない。TTL 内の正常 cache は引き続き利用する。HTTP・API・JSON エラーは保存せず、メトリクス取得で発見した status の形式・値の異常と一覧の形式異常も cache から除外し、次の scrape で再取得する。正常端末の cache は保持する。

欠測が生じた scrape は標準エラーへ欠測端末数だけを記録する。token・secret・upstream の生本文・端末名はこの通知や HTTP 応答へ含めない。部分成功は Prometheus の scrape 自体は成功するため、欠測は sample の不在と exporter のログで確認する。

Issue #87 で、端末一台の失敗でも全体を HTTP 500 とする契約から変更した。正常時の metric 名・label・順序は変わらない。失敗時の HTTP 500 を前提にしている監視は、部分成功時の欠測も確認する必要がある。判断は [ADR 0003](adr/0003-device-failure-isolation.md) に記録する。

## 設定

設定一覧と既定値は [README](../README.md#利用方法) を参照する。環境変数が `.env` より優先される。`.env` は作業 directory から親へ探索し、最初に見つかった一つだけを読み込む。引用符の除去と値の前後の空白除去を行い、重複キーは最後の定義を採用する。`export` prefix、inline comment、変数・escape 展開は行わない。

proxy はプロセス環境の `HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY` / `NO_PROXY`（小文字表記も可）を使う。独自 CA は `REQUESTS_CA_BUNDLE`、未設定または空なら `CURL_CA_BUNDLE` の PEM bundle ファイルを使用し、証明書・hostname を検証する。不正・空・読込不能な bundle は取得失敗とする。
