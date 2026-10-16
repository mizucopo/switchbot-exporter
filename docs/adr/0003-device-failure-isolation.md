# ADR 0003: 端末単位の失敗隔離

状態: 採用（[Issue #87](https://github.com/mizucopo/switchbot-exporter/issues/87)）

## 背景

一台の status 取得・変換に失敗すると、正常端末のメトリクスも参照できなくなる。ADR 0001 の全体失敗という判断を変更し、正常端末の公開を継続する。一覧・認証・設定の異常と全台失敗は成功扱いにしない。

## 判断

一覧取得後、対象端末を順番に取得し、端末の全 metric を検証してから集計する。端末の失敗後も取得を続け、一台以上で正常な status object を取得できれば HTTP 200 と正常端末の sample を返す。失敗端末は欠測とし、0や期限切れの過去値で補完しない。全対象端末が失敗した場合は固定本文の HTTP 500 を返す。

一覧の取得・形式異常、設定・client 初期化異常、HTTP 401/403 は全体を失敗させる。対象端末ゼロと、対象 metric を含まない正常な status object は HELP / TYPE のみの成功応答を許可する。

正常な URL cache の TTL と API GET 後の delay、各 request の timeout は維持する。形式・値の検証に失敗した端末 status と不正な一覧は cache から除外し、次回 scrape で再試行する。

失敗の通知は標準エラーへの欠測端末数に限定する。新しい診断 metric は追加せず、認証値・upstream 本文・端末名は記録しない。HTTP と `metrics` CLI で同じ取得処理を使用する。

## 影響

端末別エラー時の HTTP 500 が部分成功の200へ変わり、`metrics` CLI も部分成功時は終了コード0になる。正常時の metric 名・label、API/設定の形式は維持する。監視側は scrape の成功に加え、sample の欠測と exporter のログを確認する。詳細な応答・cache 契約は [exporter 仕様](../exporter.md#失敗時の応答) を参照する。
