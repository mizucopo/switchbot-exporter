#!/usr/bin/env bash
set -euo pipefail

image="${1:?Usage: docker_smoke.sh IMAGE}"
docker run --rm --network none "$image" switchbot-exporter --version
docker run --rm --network none "$image" switchbot-exporter --help

# 実機APIへ接続できない状態で、既定起動とSERVER_PORTとエラー応答を検証する。
container_id="$(docker run --detach --network none --env SERVER_PORT=19171 "$image")"
cleanup() {
  docker rm --force "$container_id" > /dev/null
}
trap cleanup EXIT

request() {
  docker exec "$container_id" bash -eu -c '
    exec 3<>/dev/tcp/127.0.0.1/19171
    printf "%s /metrics HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n" "$1" >&3
    head -n 1 <&3
  ' bash "$1"
}

response=""
for _ in {1..50}; do
  if response="$(request GET 2>/dev/null)"; then
    break
  fi
  sleep 0.1
done
[[ "$response" == $'HTTP/1.1 500 Internal Server Error\r' ]]
[[ "$(request HEAD)" == $'HTTP/1.1 500 Internal Server Error\r' ]]
[[ "$(request OPTIONS)" == $'HTTP/1.1 200 OK\r' ]]

# SIGTERMによる正常終了が確認されること。
docker kill --signal TERM "$container_id" > /dev/null
[[ "$(docker wait "$container_id")" == "0" ]]
echo 'Docker startup, HTTP routes, port configuration and SIGTERM smoke passed.'
