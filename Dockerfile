# ビルドステージ
FROM rust:1.99-alpine3.23 AS builder

RUN apk add --no-cache build-base

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked --bin switchbot-exporter

FROM builder AS test

COPY tests ./tests
RUN cargo fetch --locked
RUN --network=none cargo test --locked --offline --all-targets --all-features

# 実行ステージ
FROM alpine:3.24

RUN apk add --no-cache ca-certificates tzdata \
  && ln -snf /usr/share/zoneinfo/Asia/Tokyo /etc/localtime \
  && echo "Asia/Tokyo" > /etc/timezone

ENV TZ=Asia/Tokyo
WORKDIR /app
COPY --from=builder /build/target/release/switchbot-exporter /usr/local/bin/switchbot-exporter
EXPOSE 9171
CMD ["switchbot-exporter", "exporter"]
