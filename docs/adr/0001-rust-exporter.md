# ADR 0001: Rust runtime とプロセス内共有 cache

状態: 採用（Issue #60 の移植依頼）

## 背景

既存 exporter は Python / Flask / Gunicorn、URL ごとのディスク cache、動的 inflect / inflection による metric 属性選択で構成されています。最新依頼は性能計測を行わず、Rust へ移植し、major 変更として扱うことです。

## 判断

axum / tokio を HTTP と非同期処理、reqwest / rustls を API GET、serde を JSON、hmac / sha2 / base64 を署名に使用します。Prometheus text は既存 HELP / TYPE / family順を保つ明示的な7項目 mapping で生成します。

HTTP application state は一つの SwitchBot client を遅延生成し、mutex で同時 scrape と URL cache を共有します。API GET は順序を保持し、各成功 response 後に設定 delay を設けます。TTL は monotonic clock で判断し、ディスク I/O を行いません。

API request には設定可能な全体 timeout を設けます。エラー時は scrape 全体を失敗させます。SIGINT / SIGTERM で HTTP server を graceful shutdown します。認証情報はログ・HTTP error bodyへ出しません。

## 影響

単一 binary / 単一プロセスとなり、Python 起動方法と持続 cache は廃止します。設定・CLI形式・escape の変更は [移行ガイド](../rust-migration.md) に記録します。Docker 配布と SemVer 採番は維持し、version source を Cargo manifest / lock へ移します。

実機や本番を利用しない mock による機能・互換性検証を行います。性能改善は未計測です。PR #66 由来の共通 release controller と上流 #169 / #170 の修正は対象外です。
