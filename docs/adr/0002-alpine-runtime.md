# ADR 0002: Alpine / musl の Docker image

状態: 採用

## 背景

Docker image は linux/amd64 と linux/arm64 に対応し、HTTPS と proxy を利用できる Rust 実行環境を必要とする。builder と runtime の libc を揃え、配布 image からビルド用の入力とツールを除外する。

## 判断

builder と runtime に Alpine Linux を使用し、musl 向けにコンパイルする。使用する Rust と Alpine のタグは `Dockerfile` で管理する。

rustls の暗号処理で使用する AWS-LC のため、C/C++ ツールを builder に追加する。runtime は CA 証明書と timezone data を含み、`Asia/Tokyo`、`/app`、exporter の既定 CMD を設定する。

`test` stage では既存の Rust 統合テストと公開されたテスト専用 TLS fixture を使う。依存を取得した後、外部ネットワークを無効にして HTTP / HTTPS、独自 CA、proxy、設定とメトリクスを検証する。PR CI は各 architecture の native runner でこの stage と runtime smoke を実行する。

## 影響

配布 image は musl を使用する。ビルドツール・テスト fixture は配布 image に含めず、runtime 内の smoke は BusyBox の sh / nc を使う。

exporter の公開 API・CLI・設定・メトリクスと Docker の起動方法は変更しない。既存の共有 release controller と採番方針を維持する。軽量化幅や性能改善を実証した変更として扱わない。
