#!/usr/bin/env bash
set -euo pipefail

# Docker の既定起動・HTTP 応答・終了と実行環境が維持されること。
# Arrange: 外部ネットワークへ接続できない runtime image が用意されること。
# Act: 既定 CMD と指定 port で起動し、HTTP と SIGTERM が送信されること。
# Assert: 既存の HTTP status・UID・timezone・正常終了が確認されること。

# Arrange
image="${1:?Usage: docker_smoke.sh IMAGE}"

# Act
docker run --rm --network none "$image" switchbot-exporter --version
docker run --rm --network none "$image" switchbot-exporter --help

# 実機APIへ接続できない状態で、既定起動とSERVER_PORTとエラー応答を検証する。
container_id="$(docker run --detach --network none --env SERVER_PORT=19171 "$image")"
cleanup() {
  docker rm --force "$container_id" > /dev/null
}
trap cleanup EXIT

request() {
  local response
  response="$(docker exec "$container_id" sh -eu -c '
    printf "%s /metrics HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n" "$1" |
      nc -w 2 127.0.0.1 19171
  ' sh "$1")" || return
  printf '%s\n' "${response%%$'\n'*}"
}

response=""
for _ in {1..50}; do
  if response="$(request GET 2>/dev/null)"; then
    break
  fi
  sleep 0.1
done

# Assert
[[ "$response" == $'HTTP/1.1 500 Internal Server Error\r' ]]
[[ "$(request HEAD)" == $'HTTP/1.1 500 Internal Server Error\r' ]]
[[ "$(request OPTIONS)" == $'HTTP/1.1 200 OK\r' ]]

# 既存の root UID と Asia/Tokyo の環境設定・実際の timezone が維持されること。
docker exec "$container_id" sh -eu -c '
  test "$(id -u)" = 0
  test "$TZ" = Asia/Tokyo
  test "$(date +%z)" = +0900
'

# SIGTERMによる正常終了が確認されること。
docker kill --signal TERM "$container_id" > /dev/null
[[ "$(docker wait "$container_id")" == "0" ]]
echo 'Docker startup, HTTP routes, port configuration, UID, timezone and SIGTERM smoke passed.'
