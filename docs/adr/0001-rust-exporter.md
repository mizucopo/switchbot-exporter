# ADR 0001: Rust runtime とプロセス内共有 cache

状態: 採用

## 背景

SwitchBot API のステータスを取得し、Prometheus text と CLI の JSON として公開する。API の呼び出し間隔と cache TTL を制御し、同時 scrape による重複取得を抑える必要がある。

## 判断

axum / tokio を HTTP と非同期処理、reqwest / rustls を API GET、serde を JSON、hmac / sha2 / base64 を署名に使用する。Prometheus text は7項目の明示的な mapping で生成する。

HTTP application state は一つの SwitchBot client を遅延生成し、mutex で同時 scrape と URL ごとの cache を共有する。API GET は順序を保持し、各成功 response 後に設定 delay を設ける。TTL は monotonic clock で判断し、ディスク I/O を行わない。

API request には設定可能な全体 timeout を設ける。エラー時は scrape 全体を失敗させる。SIGINT / SIGTERM と Windows の Ctrl+C / Ctrl+Break で HTTP server を graceful shutdown する。認証情報はログ・HTTP error body へ出さない。

## 影響

単一 binary / 単一プロセスで動作し、cache は再起動時に空になる。設定と利用方法は [README](../../README.md)、取得対象・HTTP・メトリクスの契約は [exporter 仕様](../exporter.md) に記録する。

version source は Cargo manifest / lock とし、Docker と native binary の配布には repo-template の共通 release controller を使用する。機能検証はローカル HTTP / TLS / proxy mock と仮想時計で行う。
