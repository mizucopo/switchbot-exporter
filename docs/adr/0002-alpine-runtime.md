# ADR 0002: Alpine / musl の Docker image

状態: 採用（Issue #73 の Alpine 化依頼）

## 背景

Rust 版の builder は `rust:1.94-bookworm`、runtime は `debian:bookworm-slim` を使用していた。ユーザーから Alpine 化の依頼があり、既存の起動・設定・HTTPS・proxy と amd64 / arm64 対応を維持して移行する。性能・サイズ計測は行わない。

## 判断

builder を `rust:1.94-alpine3.23`、runtime を `alpine:3.23` とする。両方の公式タグが linux/amd64 と linux/arm64 に対応することを確認した。Rust 1.94 系を維持し、builder も musl に合わせてコンパイルする。

rustls の暗号処理で使用する AWS-LC のため、C/C++ ツールを builder に追加する。runtime は CA 証明書と timezone data を含み、UID、`Asia/Tokyo`、`/app`、既定 CMD、port・設定、signal による終了を維持する。

`test` stage では既存の Rust 統合テストと公開されたテスト専用 TLS fixture を使う。依存を取得した後、外部ネットワークを無効にして HTTP / HTTPS、独自 CA、proxy、設定とメトリクスを検証する。PR CI は各 architecture の native runner でこの stage と runtime smoke を実行する。

## 影響

配布 image の libc は glibc から musl に変わる。ビルドツール・テスト fixture は配布 image に含めず、runtime 内の smoke は BusyBox の sh / nc を使う。runtime に Bash / Python / curl を追加しない。

exporter の公開 API・CLI・設定・メトリクスと Docker の起動方法は変更しない。既存の共有 release controller と採番方針を維持する。軽量化幅や性能改善を実証した変更として扱わない。
