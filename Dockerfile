# ビルドステージ
FROM python:3.13-alpine3.20 AS builder

RUN apk add --no-cache \
    tzdata \
  && cp /usr/share/zoneinfo/Asia/Tokyo /etc/localtime \
  && echo "Asia/Tokyo" > /etc/timezone


# 実行ステージ
FROM python:3.13-alpine3.20

COPY --from=builder /usr/share/zoneinfo/Asia/Tokyo /etc/localtime
COPY --from=builder /etc/timezone /etc/timezone

WORKDIR /app
COPY pyproject.toml uv.lock ./
COPY src ./

RUN pip install --no-cache-dir uv \
  && uv sync --frozen \
  && rm -rf /root/.cache/uv

CMD [".venv/bin/gunicorn", "-w", "4", "-b", "0.0.0.0:9171", "--timeout", "180", "--chdir", "/app", "app:app"]
