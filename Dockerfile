# ビルドステージ
FROM rust:1.98-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked --bin switchbot-exporter

# 実行ステージ
FROM debian:bookworm-slim

RUN apt-get update \
  && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    ca-certificates tzdata \
  && ln -snf /usr/share/zoneinfo/Asia/Tokyo /etc/localtime \
  && echo "Asia/Tokyo" > /etc/timezone \
  && rm -rf /var/lib/apt/lists/*

ENV TZ=Asia/Tokyo
WORKDIR /app
COPY --from=builder /build/target/release/switchbot-exporter /usr/local/bin/switchbot-exporter
EXPOSE 9171
CMD ["switchbot-exporter", "exporter"]
